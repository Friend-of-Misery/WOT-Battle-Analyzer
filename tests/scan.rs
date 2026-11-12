use std::{fs, path::Path};
use wot_insights::{scan::Worker, store::Store, vehicles::Catalog};
fn replay(path: &Path, damage: i64) {
    let h = serde_json::json!({"playerID":42,"playerName":"Test","dateTime":"08.11.2026 12:00:00","playerVehicle":"usa-A07_T20","mapName":"map","mapDisplayName":"Map","battleType":1});
    let r = serde_json::json!([{"personal":{"1":{"team":1,"damageDealt":damage}},"common":{"winnerTeam":1}}]);
    let mut bytes = 0x1134_3212u32.to_le_bytes().to_vec();
    bytes.extend(2u32.to_le_bytes());
    for v in [h, r] {
        let raw = serde_json::to_vec(&v).unwrap();
        bytes.extend((raw.len() as u32).to_le_bytes());
        bytes.extend(raw);
    }
    fs::write(path, bytes).unwrap();
}
#[test]
fn catalog_cache_and_unchanged_scan_return_no_records() {
    let temp = tempfile::tempdir().unwrap();
    let game = temp.path().join("game");
    let replays = temp.path().join("replays");
    fs::create_dir(&replays).unwrap();
    let file = replays.join("battle.wotreplay");
    replay(&file, 123);
    let db = temp.path().join("db.sqlite");
    let mut store = Store::open(&db).unwrap();
    assert_eq!(store.import(&replays, &Catalog::new()).unwrap().imported, 1);
    let mut worker = Worker::default();
    let first = worker.scan(&db, &game, &replays, false).unwrap();
    assert!(first.replace_all);
    assert_eq!(first.battles.len(), 1);
    assert_eq!(first.battles[0].damage, Some(123));
    let next = worker.scan(&db, &game, &replays, true).unwrap();
    assert!(!next.replace_all);
    assert!(next.battles.is_empty());
    let unchanged = store.import(&replays, &Catalog::new()).unwrap();
    assert_eq!(unchanged.unchanged, 1);
    assert!(unchanged.updated.is_empty());
    assert!(!next.projections_changed);
    replay(&file, 12345);
    let changed = store.import(&replays, &Catalog::new()).unwrap();
    assert_eq!(changed.updated.len(), 1);
    assert_eq!(changed.updated[0].damage, Some(12345));
    let connection = rusqlite::Connection::open(&db).unwrap();
    assert!(
        wot_insights::migrations::columns(&connection, "sources")
            .unwrap()
            .contains(&"modified_ns".into())
    );
    assert!(
        !wot_insights::migrations::columns(&connection, "battles")
            .unwrap()
            .contains(&"payload".into())
    );
}
