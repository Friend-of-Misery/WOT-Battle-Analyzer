use rusqlite::Connection;
use serde_json::{Value, json};
use std::fs;
use wot_insights::{
    archive, damage,
    vehicles::{Catalog, Vehicle},
};
fn capture(db: &Connection, path: &std::path::Path, date: &str, real: &str, complete: bool) {
    let h = json!({"dateTime":date,"playerID":42,"playerName":"Me","playerVehicle":"usa-A07_T20","mapName":"map","mapDisplayName":"Testkarte","battleType":1,
    "vehicles":{"10":{"name":"Me","vehicleType":"usa:A07_T20","team":1},"20":{"name":"Alias","fakeName":"Alias","vehicleType":"sweden:S13_Leo","team":2},"30":{"name":"Friend","vehicleType":"usa:A07_T20","team":1}}});
    let mut result = json!([{"players":{"42":{"name":"Me","realName":"Me","team":1},"77":{"name":"Alias","realName":real,"team":2},"88":{"name":"Friend","realName":"Friend","team":1}},"vehicles":{"10":[{"accountDBID":42}],"20":[{"accountDBID":77,"damageDealt":1800,"shots":10,"directEnemyHits":7,"piercingEnemyHits":5,"damageAssistedRadio":321,"damageAssistedTrack":0,"damageAssistedStun":123}],"30":[{"accountDBID":88}]},"personal":{"1569":{"killerID":20,"details":{"(20, 0)":{"damageReceived":500,"damageDealt":125,"directHitsReceived":2,"directHits":3,"piercings":1,"explosionHits":1,"damageBlockedByArmor":250}}}},"future":{"everything":[null,0,"test"]}}]);
    if !complete {
        result = Value::Null;
    }
    let blocks = if complete { vec![h, result] } else { vec![h] };
    let mut bytes = 0x1134_3212u32.to_le_bytes().to_vec();
    bytes.extend_from_slice(&(blocks.len() as u32).to_le_bytes());
    for block in blocks {
        let b = serde_json::to_vec(&block).unwrap();
        bytes.extend_from_slice(&(b.len() as u32).to_le_bytes());
        bytes.extend(b);
    }
    bytes.extend([255, 0, 12]);
    fs::write(path, &bytes).unwrap();
    archive::capture(db, path, &bytes).unwrap();
}
#[test]
fn damage_players_aliases_and_archived_only_backfill() {
    let temp = tempfile::tempdir().unwrap();
    let database = temp.path().join("db.sqlite");
    let db = Connection::open(database).unwrap();
    damage::initialize(&db).unwrap();
    let path = temp.path().join("test.wotreplay");
    capture(&db, &path, "10.11.2026 19:00:00", "RealAccount", true);
    fs::remove_file(&path).unwrap();
    assert!(damage::backfill(&db, &Catalog::new()).unwrap().1.is_empty());
    let totals: Vec<Option<i64>>=db.query_row("SELECT gesamt_schaden,schuesse,treffer,durchschlaege,assist_aufklaerung,assist_kette,assist_betaeubung FROM v_spieler_gefechte WHERE spieler_id=77",[],|r|(0..7).map(|i|r.get(i)).collect()).unwrap();
    assert_eq!(
        totals,
        vec![
            Some(1800),
            Some(10),
            Some(7),
            Some(5),
            Some(321),
            Some(0),
            Some(123)
        ]
    );
    assert_eq!(
        db.query_row(
            "SELECT gesamt_schaden FROM gefecht_teilnehmer WHERE spieler_id=88",
            [],
            |r| r.get::<_, Option<i64>>(0)
        )
        .unwrap(),
        None
    );
    db.execute("UPDATE projection_index SET version=1", [])
        .unwrap();
    db.execute("UPDATE battle_participants SET total_damage=NULL", [])
        .unwrap();
    assert!(damage::backfill(&db, &Catalog::new()).unwrap().1.is_empty());
    assert_eq!(
        db.query_row(
            "SELECT gesamt_schaden FROM gefecht_teilnehmer WHERE spieler_id=77",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1800
    );
    let row=db.query_row("SELECT echter_name,alias,panzer,stufe,karte,schaden,durchschlagstreffer,munitionstyp,zerstoert_von,zuordnung FROM v_schaden_eingehend",[],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,Option<i64>>(3)?,r.get::<_,String>(4)?,r.get::<_,i64>(5)?,r.get::<_,Option<i64>>(6)?,r.get::<_,Option<String>>(7)?,r.get::<_,i64>(8)?,r.get::<_,String>(9)?))).unwrap();
    assert_eq!(
        row,
        (
            "RealAccount".into(),
            "Alias".into(),
            "sweden-S13_Leo".into(),
            None,
            "Testkarte".into(),
            500,
            None,
            None,
            1,
            "Gegner".into()
        )
    );
    let outgoing: (i64, i64, i64) = db
        .query_row(
            "SELECT schaden,durchschlagstreffer,explosionstreffer FROM schaden_ausgehend",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(outgoing, (125, 1, 1));
    assert_eq!(
        db.query_row(
            "SELECT zuordnung FROM v_spieler_gefechte WHERE spieler_id=88",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "Mitspieler"
    );
    assert_eq!(db.query_row("SELECT json_extract(payload,'$[0].future.everything[2]') FROM replay_json WHERE block_index=1",[],|r|r.get::<_,String>(0)).unwrap(),"test");
    let mut catalog = Catalog::new();
    catalog.insert(
        "sweden-S13_Leo".into(),
        Vehicle {
            class: "mediumTank".into(),
            tier: 7,
            role: String::new(),
        },
    );
    db.execute("UPDATE projection_index SET version=1", [])
        .unwrap();
    damage::backfill(&db, &catalog).unwrap();
    assert_eq!(
        db.query_row("SELECT stufe FROM v_schaden_eingehend", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        7
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM schaden_eingehend", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    let duplicate = temp.path().join("incomplete.wotreplay");
    capture(&db, &duplicate, "10.11.2026 19:00:00", "", false);
    damage::backfill(&db, &catalog).unwrap();
    assert_eq!(
        db.query_row("SELECT count(*) FROM schaden_eingehend", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    let second = temp.path().join("second.wotreplay");
    capture(&db, &second, "11.11.2026 19:00:00", "NewAccountName", true);
    damage::backfill(&db, &catalog).unwrap();
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM spieler WHERE account_id=77",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM v_spieler_gefechte WHERE spieler_id=77",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        2
    );
    assert_eq!(
        db.query_row(
            "SELECT echter_name FROM spieler WHERE account_id=77",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "NewAccountName"
    );
    assert_eq!(
        db.query_row(
            "SELECT echter_name FROM v_schaden_eingehend ORDER BY datum LIMIT 1",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "RealAccount"
    );
}
