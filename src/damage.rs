//! Indexed projections of archived JSON into the common relational schema.
use crate::{archive, replay_data, vehicles::Catalog};
use anyhow::Result;
use rusqlite::{Connection, params};
use serde_json::Value;
pub fn initialize(db: &Connection) -> Result<()> {
    crate::migrations::initialize(db)
}
fn text(v: &Value, field: &str) -> Option<String> {
    v[field]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}
fn number(v: &Value, field: &str) -> Option<i64> {
    v[field].as_i64()
}
fn first(v: &Value) -> &Value {
    v.as_array().and_then(|a| a.first()).unwrap_or(v)
}
fn tank(v: &str) -> String {
    v.replacen(':', "-", 1)
}
fn identity(v: &str) -> Option<(i64, i64)> {
    let v = v.trim().trim_start_matches('(').trim_end_matches(')');
    if let Some((id, life)) = v.split_once(',') {
        Some((id.trim().parse().ok()?, life.trim().parse().ok()?))
    } else {
        Some((v.parse().ok()?, 0))
    }
}
fn project(db: &Connection, doc: &replay_data::Document, catalog: &Catalog) -> Result<()> {
    let Some(h) = doc.blocks.first() else {
        return Ok(());
    };
    let Ok(key) = replay_data::identity(h) else {
        return Ok(());
    };
    let result = doc.blocks.get(1).map(first).unwrap_or(&Value::Null);
    let complete = result["personal"]
        .as_object()
        .is_some_and(|p| p.values().any(|v| v["details"].is_object()));
    let previous: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM battles WHERE battle_id=?1 AND results_available=1)",
        [&key],
        |r| r.get(0),
    )?;
    if previous && !complete {
        return Ok(());
    }
    let own = number(h, "playerID").filter(|id| *id > 0);
    if let Some(id) = own {
        db.execute("INSERT INTO players(account_id,real_name) VALUES (?1,?2) ON CONFLICT(account_id) DO UPDATE SET real_name=coalesce(excluded.real_name,players.real_name)",params![id,text(h,"playerName")])?;
    }
    if let Some(players) = result["players"].as_object() {
        for (id, p) in players {
            if let Ok(id) = id.parse::<i64>() {
                if id <= 0 {
                    continue;
                }
                db.execute("INSERT INTO players(account_id,real_name) VALUES (?1,?2) ON CONFLICT(account_id) DO UPDATE SET real_name=coalesce(excluded.real_name,players.real_name)",params![id,text(p,"realName")])?;
            }
        }
    }
    let battle = crate::replay::from_json(h, doc.blocks.get(1), catalog)?;
    crate::battle_record::save(db, &battle)?;
    db.execute(
        "UPDATE battles SET own_player_id=?2,results_available=?3 WHERE battle_id=?1",
        params![key, own, complete],
    )?;
    db.execute("DELETE FROM incoming_damage WHERE battle_id=?1", [&key])?;
    db.execute("DELETE FROM outgoing_damage WHERE battle_id=?1", [&key])?;
    db.execute("DELETE FROM battle_participants WHERE battle_id=?1", [&key])?;
    let mut ids = std::collections::BTreeSet::new();
    for source in [&h["vehicles"], &result["vehicles"]] {
        if let Some(values) = source.as_object() {
            for id in values.keys() {
                if let Ok(id) = id.parse::<i64>() {
                    ids.insert(id);
                }
            }
        }
    }
    if let Some(personal) = result["personal"].as_object() {
        for p in personal.values() {
            if let Some(details) = p["details"].as_object() {
                for id in details.keys() {
                    if let Some((id, _)) = identity(id) {
                        ids.insert(id);
                    }
                }
            }
        }
    }
    for id in ids {
        let idtext = id.to_string();
        let hv = &h["vehicles"][&idtext];
        let rv = first(&result["vehicles"][&idtext]);
        let account = number(rv, "accountDBID")
            .filter(|id| *id > 0)
            .or_else(|| number(hv, "accountDBID").filter(|id| *id > 0))
            .or_else(|| {
                (text(hv, "name") == text(h, "playerName") && text(hv, "name").is_some())
                    .then_some(own)
                    .flatten()
            });
        let player = account
            .map(|a| &result["players"][a.to_string()])
            .unwrap_or(&Value::Null);
        if let Some(a) = account {
            db.execute("INSERT INTO players(account_id,real_name) VALUES (?1,?2) ON CONFLICT(account_id) DO UPDATE SET real_name=coalesce(excluded.real_name,players.real_name)",params![a,text(player,"realName")])?;
        }
        let vehicle = text(hv, "vehicleType").map(|s| tank(&s));
        let tier = vehicle
            .as_ref()
            .and_then(|s| catalog.get(s))
            .map(|v| v.tier);
        let shown = text(player, "name")
            .or_else(|| text(hv, "fakeName"))
            .or_else(|| text(hv, "name"));
        let real = text(player, "realName");
        let alias = shown
            .as_ref()
            .filter(|s| real.as_ref().is_some_and(|r| r != *s))
            .cloned();
        db.execute(
            "INSERT INTO battle_participants(battle_id,vehicle_id,player_id,display_name,real_name,alias,tank,tier,team) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                key,
                id,
                account,
                shown,
                real,
                alias,
                vehicle,
                tier,
                number(player, "team")
                    .or_else(|| number(rv, "team"))
                    .or_else(|| number(hv, "team"))
            ],
        )?;
    }
    // Per-player totals from the battle results, separate from damage exchanged with us.
    if let Some(vehicles) = result["vehicles"].as_object() {
        for (id, records) in vehicles {
            let Ok(id) = id.parse::<i64>() else {
                continue;
            };
            let rows: Vec<&Value> = match records.as_array() {
                Some(a) => a.iter().collect(),
                None => vec![records],
            };
            let total = |field: &str| -> Option<i64> {
                if rows.is_empty() {
                    return None;
                }
                rows.iter()
                    .try_fold(0i64, |sum, row| sum.checked_add(number(row, field)?))
            };
            db.execute("UPDATE battle_participants SET total_damage=?3,shots=?4,hits=?5,penetrations=?6,spotting_assist=?7,tracking_assist=?8,stun_assist=?9 WHERE battle_id=?1 AND vehicle_id=?2",params![key,id,total("damageDealt"),total("shots"),total("directEnemyHits"),total("piercingEnemyHits"),total("damageAssistedRadio"),total("damageAssistedTrack"),total("damageAssistedStun")])?;
        }
    }
    if let Some(personal) = result["personal"].as_object() {
        for (own_vehicle, p) in personal {
            let Some(details) = p["details"].as_object() else {
                continue;
            };
            for (target, d) in details {
                let Some((id, life)) = identity(target) else {
                    continue;
                };
                let incoming = number(d, "damageReceived");
                let hits = number(d, "directHitsReceived");
                let blocked = number(d, "damageBlockedByArmor");
                let killer = number(p, "killerID").map(|k| i64::from(k == id));
                if incoming.is_some_and(|n| n > 0)
                    || hits.is_some_and(|n| n > 0)
                    || blocked.is_some_and(|n| n > 0)
                    || killer == Some(1)
                {
                    db.execute("INSERT INTO incoming_damage VALUES (?1,?2,?3,?4,?5,?6,NULL,NULL,NULL,?7,?8)",params![key,own_vehicle,id,life,incoming,hits,blocked,killer])?;
                }
                let outgoing = number(d, "damageDealt");
                let direct = number(d, "directHits");
                let penetrations = number(d, "piercings");
                let explosion = number(d, "explosionHits");
                if [outgoing, direct, penetrations, explosion]
                    .into_iter()
                    .flatten()
                    .any(|n| n > 0)
                {
                    db.execute(
                        "INSERT INTO outgoing_damage VALUES (?1,?2,?3,?4,?5,?6,?7,?8,NULL)",
                        params![
                            key,
                            own_vehicle,
                            id,
                            life,
                            outgoing,
                            direct,
                            penetrations,
                            explosion
                        ],
                    )?;
                }
            }
        }
    }
    Ok(())
}
/// Backfill archives including deleted original replays. One archive per savepoint.
pub fn backfill(db: &Connection, catalog: &Catalog) -> Result<(Vec<String>, Vec<String>)> {
    let transaction = db.unchecked_transaction()?;
    let pending = {
        let mut stmt=db.prepare("SELECT a.path,a.captured_at FROM replay_archives a LEFT JOIN projection_index i ON i.replay_path=a.path WHERE i.version IS NULL OR i.version<>2 OR i.captured_at<>a.captured_at")?;
        stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    let mut changed = Vec::new();
    let mut errors = Vec::new();
    for (path, captured) in pending {
        db.execute_batch("SAVEPOINT replay_projection")?;
        let result = (|| -> Result<()> {
            let payload: Vec<u8> = db.query_row(
                "SELECT payload FROM archive_store.replay_blobs WHERE path=?1",
                [&path],
                |r| r.get(0),
            )?;
            let doc = match archive::document(&payload, path.clone()) {
                Ok(doc) => doc,
                Err(_) => {
                    // The original bytes remain archived even for unreadable formats.
                    db.execute("INSERT INTO projection_index VALUES (?1,?2,2) ON CONFLICT(replay_path) DO UPDATE SET captured_at=excluded.captured_at,version=excluded.version",params![path,captured])?;
                    return Ok(());
                }
            };
            db.execute("DELETE FROM replay_documents WHERE replay_path=?1", [&path])?;
            for (i, block) in doc.blocks.iter().enumerate() {
                db.execute(
                    "INSERT INTO replay_documents VALUES (?1,?2,?3)",
                    params![path, i as i64, serde_json::to_string(block)?],
                )?;
            }
            project(db, &doc, catalog)?;
            if let Some(h) = doc.blocks.first() {
                if let Ok(key) = replay_data::identity(h) {
                    changed.push(key);
                }
            }
            db.execute("INSERT INTO projection_index VALUES (?1,?2,2) ON CONFLICT(replay_path) DO UPDATE SET captured_at=excluded.captured_at,version=excluded.version",params![path,captured])?;
            Ok(())
        })();
        if let Err(e) = result {
            db.execute_batch("ROLLBACK TO replay_projection")?;
            errors.push(format!("SQL-Aufbereitung {path}: {e:#}"));
        }
        db.execute_batch("RELEASE replay_projection")?;
    }
    transaction.commit()?;
    Ok((changed, errors))
}
