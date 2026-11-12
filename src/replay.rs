use crate::vehicles::Catalog;
use anyhow::{Context, Result, ensure};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Battle {
    pub key: String,
    pub date: String,
    pub player: String,
    pub map: String,
    pub map_id: String,
    pub tank: String,
    pub class: String,
    pub tier: u8,
    #[serde(default)]
    pub role: String,
    pub mode: i64,
    pub result: Option<String>,
    pub damage: Option<i64>,
    pub assist: Option<i64>,
    pub kills: Option<i64>,
    pub base_xp: Option<i64>,
    pub credits: Option<i64>,
    #[serde(default)]
    pub economy: Option<crate::economy::Economy>,
    pub shots: Option<i64>,
    #[serde(default)]
    pub hits: Option<i64>,
    #[serde(default)]
    pub penetrations: Option<i64>,
    #[serde(default)]
    pub explosion_hits: Option<i64>,
    pub received: Option<i64>,
}
fn number(r: &mut impl Read) -> Result<u32> {
    let mut b = [0; 4];
    r.read_exact(&mut b)?;
    Ok(u32::from_le_bytes(b))
}
fn json_block(r: &mut impl Read) -> Result<Value> {
    let size = number(r)? as usize;
    ensure!(size <= 16 * 1024 * 1024, "Replay JSON exceeds 16 MiB limit");
    let mut b = vec![0; size];
    r.read_exact(&mut b)?;
    Ok(serde_json::from_slice(&b)?)
}
fn text(v: &Value, key: &str) -> Result<String> {
    Ok(v[key]
        .as_str()
        .with_context(|| format!("Missing replay field: {key}"))?
        .to_owned())
}

pub fn parse(path: &Path, catalog: &Catalog) -> Result<Battle> {
    let mut reader = BufReader::new(File::open(path)?);
    ensure!(
        number(&mut reader)? == 0x1134_3212,
        "Invalid replay signature"
    );
    let count = number(&mut reader)?;
    ensure!((1..=16).contains(&count), "Invalid replay block count");
    let header = json_block(&mut reader)?;
    let results = if count >= 2 {
        Some(json_block(&mut reader)?)
    } else {
        None
    };
    from_json(&header, results.as_ref(), catalog)
}
pub fn from_json(header: &Value, result: Option<&Value>, catalog: &Catalog) -> Result<Battle> {
    let player = text(header, "playerName")?;
    let tank = text(header, "playerVehicle")?;
    let date = NaiveDateTime::parse_from_str(&text(header, "dateTime")?, "%d.%m.%Y %H:%M:%S")?
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    let map_id = text(header, "mapName")?;
    let mode = header["battleType"]
        .as_i64()
        .context("Missing battle type")?;
    let vehicle = catalog.get(&tank);
    let mut battle = Battle {
        key: serde_json::to_string(&serde_json::json!([
            date,
            header["playerID"],
            player,
            tank,
            map_id,
            mode
        ]))?,
        date,
        player,
        map: text(header, "mapDisplayName")?,
        map_id,
        tank,
        class: vehicle
            .map(|v| v.class.clone())
            .unwrap_or_else(|| "unknown".into()),
        tier: vehicle.map(|v| v.tier).unwrap_or(0),
        role: vehicle.map(|v| v.role.clone()).unwrap_or_default(),
        mode,
        result: None,
        damage: None,
        assist: None,
        kills: None,
        base_xp: None,
        credits: None,
        economy: None,
        shots: None,
        hits: None,
        penetrations: None,
        explosion_hits: None,
        received: None,
    };
    let Some(result) = result else {
        return Ok(battle);
    };
    let result = if result.is_array() {
        &result[0]
    } else {
        result
    };
    let Some(personal) = result["personal"].as_object() else {
        return Ok(battle);
    };
    // Multi-vehicle modes need a separate aggregation model; never choose an arbitrary tank.
    let tanks: Vec<_> = personal
        .iter()
        .filter(|(k, _)| k.parse::<u32>().is_ok())
        .collect();
    if tanks.len() != 1 {
        return Ok(battle);
    }
    let stats = tanks[0].1;
    let winner = result["common"]["winnerTeam"].as_i64();
    let team = stats["team"]
        .as_i64()
        .or_else(|| personal.get("avatar").and_then(|a| a["team"].as_i64()));
    battle.result = match (winner, team) {
        (Some(0), Some(1 | 2)) => Some("draw".into()),
        (Some(w @ (1 | 2)), Some(t @ (1 | 2))) => Some(if w == t { "win" } else { "loss" }.into()),
        _ => None,
    };
    battle.damage = stats["damageDealt"].as_i64();
    battle.assist = stats["damageAssistedRadio"]
        .as_i64()
        .zip(stats["damageAssistedTrack"].as_i64())
        .zip(stats["damageAssistedStun"].as_i64())
        .map(|((radio, track), stun)| radio + track + stun);
    battle.kills = stats["kills"].as_i64();
    battle.base_xp = stats["originalXP"].as_i64();
    battle.credits = stats["credits"].as_i64();
    battle.economy = Some(crate::economy::parse(stats));
    battle.shots = stats["shots"].as_i64().filter(|n| *n >= 0);
    // These are the opponent counters used by WoT's own SHOTS / HITS result records.
    battle.hits = stats["directEnemyHits"].as_i64().filter(|n| *n >= 0);
    battle.penetrations = stats["piercingEnemyHits"].as_i64().filter(|n| *n >= 0);
    battle.explosion_hits = stats["explosionHits"].as_i64().filter(|n| *n >= 0);
    battle.received = stats["damageReceived"].as_i64();
    Ok(battle)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn header() -> Value {
        serde_json::json!({"playerName":"Test", "playerID":42,"playerVehicle":"usa-A07_T20", "dateTime":"08.11.2026 17:00:00", "mapName":"map", "mapDisplayName":"Karte", "battleType":1})
    }
    #[test]
    fn missing_results_are_not_zero_or_loss() {
        let b = from_json(&header(), None, &Catalog::new()).unwrap();
        assert!(b.damage.is_none());
        assert!(b.result.is_none());
    }
    #[test]
    fn avatar_is_not_mistaken_for_vehicle_and_win_is_team_relative() {
        let r = serde_json::json!([{"personal":{"1569":{"team":2,"damageDealt":900,"damageAssistedRadio":50,"damageAssistedTrack":20,"damageAssistedStun":0},"avatar":{"team":2}},"common":{"winnerTeam":2}}]);
        let b = from_json(&header(), Some(&r), &Catalog::new()).unwrap();
        assert_eq!(b.result.as_deref(), Some("win"));
        assert_eq!(b.damage, Some(900));
        assert_eq!(b.assist, Some(70));
    }
    #[test]
    fn multiple_vehicles_are_not_silently_mixed() {
        let r = serde_json::json!([{"personal":{"1":{"damageDealt":100},"2":{"damageDealt":900}},"common":{"winnerTeam":1}}]);
        assert!(
            from_json(&header(), Some(&r), &Catalog::new())
                .unwrap()
                .damage
                .is_none()
        );
    }
}
