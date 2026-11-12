use rusqlite::{Connection, params};
use std::{fs, io::Write};
use wot_insights::{store::Store, vehicles::Catalog};
fn legacy(
    db: &std::path::Path,
    file: &std::path::Path,
    bad: bool,
) -> (wot_insights::replay::Battle, Vec<u8>) {
    let header = serde_json::json!({"playerName":"Test","playerID":42,"playerVehicle":"usa-A07_T20","dateTime":"10.11.2026 19:00:00","mapName":"map","mapDisplayName":"Karte","battleType":1,"future":{"all":[1,null]}});
    let battle = wot_insights::replay::from_json(&header, None, &Catalog::new()).unwrap();
    let block = serde_json::to_vec(&header).unwrap();
    let mut bytes = 0x1134_3212u32.to_le_bytes().to_vec();
    bytes.extend(1u32.to_le_bytes());
    bytes.extend((block.len() as u32).to_le_bytes());
    bytes.extend(block);
    bytes.extend([0, 255, 100]);
    fs::write(file, &bytes).unwrap();
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    zip.start_file(
        "original.wotreplay",
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    zip.write_all(&bytes).unwrap();
    let payload = zip.finish().unwrap().into_inner();
    let con = Connection::open(db).unwrap();
    con.execute_batch("CREATE TABLE battles(identity TEXT PRIMARY KEY,date TEXT,payload TEXT); CREATE TABLE sources(path TEXT PRIMARY KEY,size INTEGER,modified TEXT,parser_version INTEGER); CREATE TABLE replay_archives(path TEXT PRIMARY KEY,original_size INTEGER,captured_at TEXT,format_version INTEGER,payload BLOB); CREATE TABLE replay_archive_battles(path TEXT PRIMARY KEY,identity TEXT);PRAGMA user_version=2;").unwrap();
    con.execute(
        "INSERT INTO battles VALUES (?1,?2,?3)",
        params![
            battle.key,
            battle.date,
            if bad {
                "{".into()
            } else {
                serde_json::to_string(&battle).unwrap()
            }
        ],
    )
    .unwrap();
    con.execute(
        "INSERT INTO sources VALUES (?1,?2,?3,3)",
        params![
            file.to_string_lossy(),
            bytes.len(),
            format!("{:?}", file.metadata().unwrap().modified().unwrap())
        ],
    )
    .unwrap();
    let path = file.canonicalize().unwrap().to_string_lossy().into_owned();
    con.execute(
        "INSERT INTO replay_archives VALUES (?1,?2,'captured',1,?3)",
        params![path, bytes.len(), payload],
    )
    .unwrap();
    con.execute(
        "INSERT INTO replay_archive_battles VALUES (?1,?2)",
        params![path, battle.key],
    )
    .unwrap();
    (battle, bytes)
}
#[test]
fn legacy_data_archives_and_unknown_fields_survive_migration_and_reopen() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("battles.db");
    let file = temp.path().join("original.wotreplay");
    let (before, bytes) = legacy(&db, &file, false);
    let timestamp =
        wot_insights::migrations::timestamp(file.metadata().unwrap().modified().unwrap()).unwrap();
    fs::remove_file(&file).unwrap();
    for _ in 0..2 {
        let store = Store::open(&db).unwrap();
        let battle = store.battles().unwrap().remove(0);
        assert_eq!(
            serde_json::to_value(&battle).unwrap(),
            serde_json::to_value(&before).unwrap()
        );
        let document = store.battle_document(&battle).unwrap();
        assert_eq!(document.original_size, bytes.len());
        assert_eq!(document.stream, [0, 255, 100]);
        assert_eq!(document.blocks[0]["future"]["all"][0], 1);
    }
    let con = wot_insights::migrations::open(&db).unwrap();
    assert_eq!(
        con.query_row("SELECT modified_ns FROM sources", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        timestamp
    );
    assert!(
        !wot_insights::migrations::columns(&con, "replay_archives")
            .unwrap()
            .contains(&"payload".into())
    );
    assert_eq!(
        con.query_row("SELECT count(*) FROM archive_store.replay_blobs", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert!(wot_insights::migrations::archive_path(&db).exists());
    assert_eq!(
        con.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| r
            .get::<_, i64>(
            0
        ))
        .unwrap(),
        0
    );
}
#[test]
fn failed_json_migration_keeps_original_tables_and_blobs() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("battles.db");
    let file = temp.path().join("original.wotreplay");
    legacy(&db, &file, true);
    assert!(Store::open(&db).is_err());
    let con = Connection::open(&db).unwrap();
    assert_eq!(
        con.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        con.query_row("SELECT payload FROM battles", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "{"
    );
    assert_eq!(
        con.query_row(
            "SELECT count(*) FROM replay_archives WHERE payload IS NOT NULL",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
}
#[test]
fn missing_archive_file_is_reported_without_creating_an_empty_replacement() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("battles.db");
    let file = temp.path().join("replay.wotreplay");
    legacy(&db, &file, false);
    drop(Store::open(&db).unwrap());
    let archive = wot_insights::migrations::archive_path(&db);
    let retained = temp.path().join("retained.db");
    fs::rename(&archive, &retained).unwrap();
    let error = match Store::open(&db) {
        Ok(_) => panic!("missing archive must be reported"),
        Err(e) => e,
    };
    assert!(format!("{error:#}").contains("Archivdatei fehlt"));
    assert!(!archive.exists());
    assert!(retained.exists());
}
