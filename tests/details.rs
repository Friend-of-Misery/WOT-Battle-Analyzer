use std::fs;
use wot_insights::{store::Store, vehicles::Catalog};

#[test]
fn full_details_survive_deleted_original_and_legacy_archive_index() {
    let temp = tempfile::tempdir().unwrap();
    let folder = temp.path().join("replays");
    fs::create_dir(&folder).unwrap();
    let path = folder.join("custom-name.wotreplay");
    let database = temp.path().join("battles.db");
    let header = serde_json::json!({"playerName":"Test", "playerID":42,"playerVehicle":"usa-A07_T20", "dateTime":"10.11.2026 19:01:15", "mapName":"map", "mapDisplayName":"Karte", "battleType":1, "futureHeader":{"unchanged":true}});
    let result = serde_json::json!([{"personal":{"1":{"team":1,"damageDealt":1234,"futureStat":{"zero":0,"missing":null}}},"common":{"winnerTeam":1}}]);
    let extra = serde_json::json!({"additionalUnknownBlock":[1,2,3]});
    let mut bytes = 0x1134_3212u32.to_le_bytes().to_vec();
    bytes.extend_from_slice(&3u32.to_le_bytes());
    for value in [&header, &result, &extra] {
        let block = serde_json::to_vec(value).unwrap();
        bytes.extend_from_slice(&(block.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&block);
    }
    bytes.extend_from_slice(&[255, 0, 17, 90]);
    fs::write(&path, &bytes).unwrap();
    let mut store = Store::open(&database).unwrap();
    assert_eq!(store.import(&folder, &Catalog::new()).unwrap().imported, 1);
    let battle = store.battles().unwrap().remove(0);
    fs::remove_file(&path).unwrap();
    for legacy in [false, true] {
        if legacy {
            rusqlite::Connection::open(&database)
                .unwrap()
                .execute("DELETE FROM replay_archive_battles", [])
                .unwrap();
        }
        let document = store.battle_document(&battle).unwrap();
        assert_eq!(
            document.blocks,
            vec![header.clone(), result.clone(), extra.clone()]
        );
        assert_eq!(document.stream, [255, 0, 17, 90]);
        assert_eq!(document.original_size, bytes.len());
    }
}
