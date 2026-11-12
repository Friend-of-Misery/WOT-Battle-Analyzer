use std::{fs, io::Write};
use tempfile::tempdir;
use wot_insights::{store::Store, vehicles::Catalog};

fn replay(path: &std::path::Path, complete: bool) {
    let header = serde_json::json!({"playerID":42,"playerName":"Test","dateTime":"08.11.2026 12:00:00","playerVehicle":"usa-A07_T20","mapName":"map","mapDisplayName":"Testkarte","battleType":1});
    let result = serde_json::json!([{"personal":{"1569":{"team":1,"damageDealt":1234,"originalXP":700,"damageAssistedRadio":100,"damageAssistedTrack":20,"damageAssistedStun":0,"credits":2000,"originalCredits":1000,"subtotalCredits":1500,"appliedPremiumCreditsFactor100":150,"boosterCredits":500,"orderCredits":0,"eventCredits":0,"autoRepairCost":100,"autoLoadCost":[200,0],"autoEquipCost":[300,0,0],"shots":10,"directEnemyHits":6,"piercingEnemyHits":4,"directHits":99,"piercings":77,"explosionHits":2}},"common":{"winnerTeam":1}}]);
    let mut file = fs::File::create(path).unwrap();
    file.write_all(&[0x12, 0x32, 0x34, 0x11]).unwrap();
    file.write_all(&(if complete { 2u32 } else { 1u32 }).to_le_bytes())
        .unwrap();
    let blocks = if complete {
        vec![header, result]
    } else {
        vec![header]
    };
    for block in blocks {
        let bytes = serde_json::to_vec(&block).unwrap();
        file.write_all(&(bytes.len() as u32).to_le_bytes()).unwrap();
        file.write_all(&bytes).unwrap();
    }
}

#[test]
fn incremental_import_deduplicates_and_upgrades_incomplete_battle() {
    let temp = tempdir().unwrap();
    let folder = temp.path().join("replays");
    fs::create_dir(&folder).unwrap();
    let path = folder.join("battle.wotreplay");
    let mut store = Store::open(&temp.path().join("stats.db")).unwrap();
    let catalog = Catalog::new();
    replay(&path, false);
    assert_eq!(store.import(&folder, &catalog).unwrap().imported, 1);
    assert!(store.battles().unwrap()[0].result.is_none());
    assert_eq!(store.import(&folder, &catalog).unwrap().unchanged, 1);
    replay(&path, true);
    assert_eq!(store.import(&folder, &catalog).unwrap().imported, 1);
    let battles = store.battles().unwrap();
    assert_eq!(battles.len(), 1);
    assert_eq!(battles[0].damage, Some(1234));
    fs::copy(&path, folder.join("copy.wotreplay")).unwrap();
    store.import(&folder, &catalog).unwrap();
    assert_eq!(store.battles().unwrap().len(), 1);
    replay(&folder.join("incomplete-copy.wotreplay"), false);
    store.import(&folder, &catalog).unwrap();
    assert_eq!(store.battles().unwrap()[0].damage, Some(1234));
}

#[test]
fn corrupt_replay_does_not_block_valid_import_or_allocate_from_untrusted_length() {
    let temp = tempdir().unwrap();
    replay(&temp.path().join("valid.wotreplay"), true);
    let mut corrupt = fs::File::create(temp.path().join("bad.wotreplay")).unwrap();
    corrupt
        .write_all(&[0x12, 0x32, 0x34, 0x11, 1, 0, 0, 0, 255, 255, 255, 255])
        .unwrap();
    let mut store = Store::open(&temp.path().join("stats.db")).unwrap();
    let report = store.import(temp.path(), &Catalog::new()).unwrap();
    assert_eq!(report.imported, 1);
    assert_eq!(report.errors.len(), 1);
    assert_eq!(store.battles().unwrap().len(), 1);
}

#[test]
fn unchanged_replays_receive_new_role_metadata_without_losing_results() {
    let temp = tempdir().unwrap();
    let folder = temp.path().join("replays");
    fs::create_dir(&folder).unwrap();
    replay(&folder.join("battle.wotreplay"), true);
    let mut store = Store::open(&temp.path().join("stats.db")).unwrap();
    store.import(&folder, &Catalog::new()).unwrap();
    let catalog = Catalog::from([(
        "usa-A07_T20".into(),
        wot_insights::vehicles::Vehicle {
            class: "mediumTank".into(),
            tier: 7,
            role: "role_MT_sniper".into(),
        },
    )]);
    let report = store.import(&folder, &catalog).unwrap();
    assert_eq!(report.unchanged, 1);
    assert_eq!(report.imported, 0);
    let battles = store.battles().unwrap();
    assert_eq!(battles.len(), 1);
    assert_eq!(battles[0].role, "role_MT_sniper");
    assert_eq!(battles[0].tier, 7);
    assert_eq!(battles[0].damage, Some(1234));
    assert_eq!(battles[0].result.as_deref(), Some("win"));
}

fn create_legacy(
    database: &std::path::Path,
    folder: &std::path::Path,
    parser: Option<i64>,
    remove: &[&str],
) {
    let file = folder.join("battle.wotreplay");
    let battle = wot_insights::replay::parse(&file, &Catalog::new()).unwrap();
    let mut payload = serde_json::to_value(battle).unwrap();
    for field in remove {
        payload.as_object_mut().unwrap().remove(*field);
    }
    let db = rusqlite::Connection::open(database).unwrap();
    db.execute_batch("CREATE TABLE battles(identity TEXT PRIMARY KEY,date TEXT NOT NULL,payload TEXT NOT NULL); CREATE TABLE sources(path TEXT PRIMARY KEY,size INTEGER NOT NULL,modified TEXT NOT NULL);PRAGMA user_version=2;").unwrap();
    if parser.is_some() {
        db.execute_batch(
            "ALTER TABLE sources ADD COLUMN parser_version INTEGER NOT NULL DEFAULT 0",
        )
        .unwrap();
    }
    db.execute(
        "INSERT INTO battles VALUES (?1,?2,?3)",
        rusqlite::params![
            payload["key"].as_str(),
            payload["date"].as_str(),
            payload.to_string()
        ],
    )
    .unwrap();
    let meta = file.metadata().unwrap();
    db.execute(
        "INSERT INTO sources(path,size,modified) VALUES (?1,?2,?3)",
        rusqlite::params![
            file.to_string_lossy(),
            meta.len(),
            format!("{:?}", meta.modified().unwrap())
        ],
    )
    .unwrap();
    if let Some(version) = parser {
        db.execute("UPDATE sources SET parser_version=?1", [version])
            .unwrap();
    }
}
#[test]
fn old_database_reimports_unchanged_files_for_credit_accounting() {
    let temp = tempdir().unwrap();
    let folder = temp.path().join("replays");
    fs::create_dir(&folder).unwrap();
    let database = temp.path().join("stats.db");
    replay(&folder.join("battle.wotreplay"), true);
    create_legacy(&database, &folder, None, &["economy"]);
    let mut store = Store::open(&database).unwrap();
    assert!(store.battles().unwrap()[0].economy.is_none());
    assert_eq!(store.import(&folder, &Catalog::new()).unwrap().imported, 1);
    assert_eq!(
        store.battles().unwrap()[0].economy.as_ref().unwrap().net(),
        Some(1400)
    );
    assert_eq!(store.import(&folder, &Catalog::new()).unwrap().unchanged, 1);
}
#[test]
fn prior_parser_version_reimports_enemy_hit_counters_and_preserves_credit_accounting() {
    let temp = tempdir().unwrap();
    let folder = temp.path().join("replays");
    fs::create_dir(&folder).unwrap();
    let database = temp.path().join("stats.db");
    replay(&folder.join("battle.wotreplay"), true);
    create_legacy(
        &database,
        &folder,
        Some(1),
        &["hits", "penetrations", "explosion_hits"],
    );
    let mut store = Store::open(&database).unwrap();
    assert!(store.battles().unwrap()[0].hits.is_none());
    assert_eq!(store.import(&folder, &Catalog::new()).unwrap().imported, 1);
    let battles = store.battles().unwrap();
    assert_eq!(battles[0].shots, Some(10));
    assert_eq!(battles[0].hits, Some(6));
    assert_eq!(battles[0].penetrations, Some(4));
    assert_eq!(battles[0].explosion_hits, Some(2));
    assert_eq!(battles[0].economy.as_ref().unwrap().net(), Some(1400));
}
