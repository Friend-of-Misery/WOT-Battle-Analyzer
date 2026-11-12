//! Typed access to the single relational battle model. No Battle JSON is stored.
use crate::{economy::Economy, replay::Battle};
use anyhow::Result;
use rusqlite::{Connection, params};
pub const TABLE: &str = r#"CREATE TABLE IF NOT EXISTS battles (battle_id TEXT PRIMARY KEY,date TEXT NOT NULL DEFAULT '',player TEXT NOT NULL DEFAULT '',map TEXT NOT NULL DEFAULT '',map_id TEXT NOT NULL DEFAULT '',tank TEXT NOT NULL DEFAULT '',class TEXT NOT NULL DEFAULT '',tier INTEGER NOT NULL DEFAULT 0,role TEXT NOT NULL DEFAULT '',mode INTEGER NOT NULL DEFAULT 0,result TEXT,damage INTEGER,assist INTEGER,kills INTEGER,base_xp INTEGER,credits INTEGER,shots INTEGER,hits INTEGER,penetrations INTEGER,explosion_hits INTEGER,received INTEGER,own_player_id INTEGER,results_available INTEGER NOT NULL DEFAULT 0,summary_available INTEGER NOT NULL DEFAULT 0);
CREATE INDEX IF NOT EXISTS battles_date ON battles(date DESC,battle_id);
CREATE INDEX IF NOT EXISTS battles_tank ON battles(tank);
CREATE TABLE IF NOT EXISTS battle_economy(battle_id TEXT PRIMARY KEY REFERENCES battles(battle_id),gross INTEGER,base INTEGER,premium INTEGER,personal_reserve INTEGER,clan_reserve INTEGER,missions INTEGER,other INTEGER,repair INTEGER,ammunition INTEGER,consumables INTEGER,gold_expenses INTEGER,bond_expenses INTEGER);"#;
pub fn save(db: &Connection, b: &Battle) -> Result<bool> {
    let changed=db.execute("INSERT INTO battles(battle_id,date,player,map,map_id,tank,class,tier,role,mode,result,damage,assist,kills,base_xp,credits,shots,hits,penetrations,explosion_hits,received,summary_available) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,1) ON CONFLICT(battle_id) DO UPDATE SET date=excluded.date,player=excluded.player,map=excluded.map,map_id=excluded.map_id,tank=excluded.tank,class=excluded.class,tier=excluded.tier,role=excluded.role,mode=excluded.mode,result=excluded.result,damage=excluded.damage,assist=excluded.assist,kills=excluded.kills,base_xp=excluded.base_xp,credits=excluded.credits,shots=excluded.shots,hits=excluded.hits,penetrations=excluded.penetrations,explosion_hits=excluded.explosion_hits,received=excluded.received,summary_available=1 WHERE battles.summary_available=0 OR battles.result IS NULL OR excluded.result IS NOT NULL",params![b.key,b.date,b.player,b.map,b.map_id,b.tank,b.class,b.tier,b.role,b.mode,b.result,b.damage,b.assist,b.kills,b.base_xp,b.credits,b.shots,b.hits,b.penetrations,b.explosion_hits,b.received])? > 0;
    if changed {
        if let Some(e) = &b.economy {
            db.execute("INSERT INTO battle_economy VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13) ON CONFLICT(battle_id) DO UPDATE SET gross=excluded.gross,base=excluded.base,premium=excluded.premium,personal_reserve=excluded.personal_reserve,clan_reserve=excluded.clan_reserve,missions=excluded.missions,other=excluded.other,repair=excluded.repair,ammunition=excluded.ammunition,consumables=excluded.consumables,gold_expenses=excluded.gold_expenses,bond_expenses=excluded.bond_expenses",params![b.key,e.gross,e.base,e.premium,e.personal_reserve,e.clan_reserve,e.missions,e.other,e.repair,e.ammunition,e.consumables,e.gold_expenses,e.bond_expenses])?;
        } else {
            db.execute("DELETE FROM battle_economy WHERE battle_id=?1", [&b.key])?;
        }
    }
    Ok(changed)
}
const SELECT: &str = "SELECT b.battle_id,b.date,b.player,b.map,b.map_id,b.tank,b.class,b.tier,b.role,b.mode,b.result,b.damage,b.assist,b.kills,b.base_xp,b.credits,b.shots,b.hits,b.penetrations,b.explosion_hits,b.received,e.battle_id,e.gross,e.base,e.premium,e.personal_reserve,e.clan_reserve,e.missions,e.other,e.repair,e.ammunition,e.consumables,e.gold_expenses,e.bond_expenses FROM battles b LEFT JOIN battle_economy e USING(battle_id)";
fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Battle> {
    Ok(Battle {
        key: r.get(0)?,
        date: r.get(1)?,
        player: r.get(2)?,
        map: r.get(3)?,
        map_id: r.get(4)?,
        tank: r.get(5)?,
        class: r.get(6)?,
        tier: r.get(7)?,
        role: r.get(8)?,
        mode: r.get(9)?,
        result: r.get(10)?,
        damage: r.get(11)?,
        assist: r.get(12)?,
        kills: r.get(13)?,
        base_xp: r.get(14)?,
        credits: r.get(15)?,
        shots: r.get(16)?,
        hits: r.get(17)?,
        penetrations: r.get(18)?,
        explosion_hits: r.get(19)?,
        received: r.get(20)?,
        economy: if r.get::<_, Option<String>>(21)?.is_some() {
            Some(Economy {
                gross: r.get(22)?,
                base: r.get(23)?,
                premium: r.get(24)?,
                personal_reserve: r.get(25)?,
                clan_reserve: r.get(26)?,
                missions: r.get(27)?,
                other: r.get(28)?,
                repair: r.get(29)?,
                ammunition: r.get(30)?,
                consumables: r.get(31)?,
                gold_expenses: r.get(32)?,
                bond_expenses: r.get(33)?,
            })
        } else {
            None
        },
    })
}
pub fn all(db: &Connection) -> Result<Vec<Battle>> {
    let mut stmt = db.prepare(&format!(
        "{SELECT} WHERE b.summary_available=1 ORDER BY b.date DESC,b.battle_id"
    ))?;
    let values = stmt
        .query_map([], row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(values)
}
pub fn get(db: &Connection, key: &str) -> Result<Option<Battle>> {
    use rusqlite::OptionalExtension;
    Ok(db
        .query_row(
            &format!("{SELECT} WHERE b.battle_id=?1 AND b.summary_available=1"),
            [key],
            row,
        )
        .optional()?)
}
pub const JSON_VIEW: &str = "CREATE VIEW battle_json AS SELECT b.battle_id AS identity,b.date,json_object('key',b.battle_id,'date',b.date,'player',b.player,'map',b.map,'map_id',b.map_id,'tank',b.tank,'class',b.class,'tier',b.tier,'role',b.role,'mode',b.mode,'result',b.result,'damage',b.damage,'assist',b.assist,'kills',b.kills,'base_xp',b.base_xp,'credits',b.credits,'shots',b.shots,'hits',b.hits,'penetrations',b.penetrations,'explosion_hits',b.explosion_hits,'received',b.received,'economy',json(CASE WHEN e.battle_id IS NULL THEN 'null' ELSE json_object('gross',e.gross,'base',e.base,'premium',e.premium,'personal_reserve',e.personal_reserve,'clan_reserve',e.clan_reserve,'missions',e.missions,'other',e.other,'repair',e.repair,'ammunition',e.ammunition,'consumables',e.consumables,'gold_expenses',e.gold_expenses,'bond_expenses',e.bond_expenses) END)) AS payload FROM battles b LEFT JOIN battle_economy e USING(battle_id) WHERE b.summary_available=1;";
