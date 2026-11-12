//! Versioned, transactional migration from the original JSON/German schema.
use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, params};
use std::{
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
const VERSION: i64 = 11;
pub fn archive_path(path: &Path) -> PathBuf {
    path.with_file_name(format!(
        "{}.archive.db",
        path.file_stem().unwrap_or_default().to_string_lossy()
    ))
}
pub fn attach_archive(db: &Connection) -> Result<()> {
    let attached: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM pragma_database_list WHERE name='archive_store')",
        [],
        |r| r.get(0),
    )?;
    if attached {
        return Ok(());
    }
    let file = db
        .path()
        .filter(|p| !p.is_empty())
        .map(|p| archive_path(Path::new(p)));
    if let Some(path) = &file {
        if !path.exists()
            && table(db, "replay_archives")?
            && !columns(db, "replay_archives")?
                .iter()
                .any(|c| c == "payload")
        {
            let count: i64 =
                db.query_row("SELECT count(*) FROM replay_archives", [], |r| r.get(0))?;
            ensure!(
                count == 0,
                "Archivdatei fehlt: {}. Bitte Datenbank und zugehörige Archivdatei gemeinsam wiederherstellen.",
                path.display()
            );
        }
    }
    let name = file
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| ":memory:".into());
    db.execute("ATTACH DATABASE ?1 AS archive_store", [name])?;
    db.execute_batch("PRAGMA archive_store.journal_mode=WAL; PRAGMA archive_store.synchronous=FULL; CREATE TABLE IF NOT EXISTS archive_store.replay_blobs(path TEXT PRIMARY KEY,captured_at TEXT NOT NULL,payload BLOB NOT NULL);")?;
    Ok(())
}
pub fn open(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let db = Connection::open(path)?;
    db.busy_timeout(Duration::from_secs(10))?;
    initialize(&db)?;
    Ok(db)
}
pub fn columns(db: &Connection, table: &str) -> Result<Vec<String>> {
    let mut q = db.prepare("SELECT name FROM pragma_table_info(?1)")?;
    Ok(q.query_map([table], |r| r.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}
fn table(db: &Connection, name: &str) -> Result<bool> {
    Ok(db.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE type='table' AND name=?1)",
        [name],
        |r| r.get(0),
    )?)
}
pub fn timestamp(time: SystemTime) -> Result<i64> {
    let value = match time.duration_since(UNIX_EPOCH) {
        Ok(d) => i128::try_from(d.as_nanos())?,
        Err(e) => -i128::try_from(e.duration().as_nanos())?,
    };
    Ok(i64::try_from(value)?)
}
fn old_timestamp(value: &str) -> Option<i64> {
    if let Some(ticks) = value
        .split("intervals: ")
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .and_then(|s| s.parse::<i128>().ok())
    {
        return i64::try_from((ticks - 116444736000000000) * 100).ok();
    }
    let seconds = value
        .split("tv_sec: ")
        .nth(1)?
        .split(',')
        .next()?
        .trim()
        .parse::<i128>()
        .ok()?;
    let nanos = value
        .split("tv_nsec: ")
        .nth(1)?
        .split_whitespace()
        .next()?
        .parse::<i128>()
        .ok()?;
    i64::try_from(seconds * 1_000_000_000 + nanos).ok()
}
pub fn initialize(db: &Connection) -> Result<()> {
    attach_archive(db)?;
    let version: i64 = db.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    ensure!(
        version <= VERSION,
        "Datenbank stammt aus einer neueren Programmversion ({version})."
    );
    if version == VERSION {
        db.execute_batch("PRAGMA foreign_keys=ON;")?;
        return Ok(());
    }
    db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=OFF;")?;
    let old_archive = columns(db, "replay_archives")?
        .iter()
        .any(|s| s == "payload");
    if old_archive {
        // Commit and verify the complete external copy BEFORE removing any original BLOB.
        db.execute_batch("BEGIN IMMEDIATE; INSERT INTO archive_store.replay_blobs SELECT path,captured_at,payload FROM main.replay_archives WHERE true ON CONFLICT(path) DO UPDATE SET captured_at=excluded.captured_at,payload=excluded.payload; COMMIT;")?;
        let missing:i64=db.query_row("SELECT count(*) FROM main.replay_archives a LEFT JOIN archive_store.replay_blobs b USING(path) WHERE b.payload IS NULL OR a.payload<>b.payload",[],|r|r.get(0))?;
        ensure!(
            missing == 0,
            "Archivkopie unvollständig; Originalarchiv bleibt erhalten."
        );
    }
    let tx = db.unchecked_transaction()?;
    for view in [
        "v_schaden_eingehend",
        "v_schaden_ausgehend",
        "v_spieler_gefechte",
        "battle_json",
        "spieler",
        "gefecht_teilnehmer",
        "schaden_eingehend",
        "schaden_ausgehend",
        "gefechte",
        "replay_json",
        "replay_sql_index",
    ] {
        let is_view: bool = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE name=?1 AND type='view')",
            [view],
            |r| r.get(0),
        )?;
        if is_view {
            db.execute_batch(&format!("DROP VIEW {view}"))?;
        }
    }
    let legacy = columns(db, "battles")?.iter().any(|s| s == "payload");
    if legacy {
        db.execute_batch("ALTER TABLE battles RENAME TO legacy_battles")?;
    }
    let old_sources =
        table(db, "sources")? && !columns(db, "sources")?.iter().any(|s| s == "modified_ns");
    if old_sources {
        db.execute_batch("ALTER TABLE sources RENAME TO legacy_sources")?;
    }
    if old_archive {
        db.execute_batch("ALTER TABLE replay_archives RENAME TO legacy_archives")?;
    }
    db.execute_batch(crate::battle_record::TABLE)?;
    db.execute_batch(include_str!("schema.sql"))?;
    if legacy {
        let mut q = db.prepare("SELECT payload FROM legacy_battles")?;
        for value in q.query_map([], |r| r.get::<_, String>(0))? {
            let battle: crate::replay::Battle = serde_json::from_str(&value?).context(
                "Bestehendes Gefecht konnte nicht migriert werden; Migration wird zurückgerollt",
            )?;
            crate::battle_record::save(db, &battle)?;
        }
    }
    if table(db, "gefechte")? {
        db.execute_batch("INSERT INTO battles(battle_id,date,map,map_id,own_player_id,results_available) SELECT gefechts_id,coalesce(datum,''),coalesce(karte,''),coalesce(karten_id,''),nullif(eigener_spieler_id,0),ergebnis_vorhanden FROM gefechte WHERE true ON CONFLICT(battle_id) DO UPDATE SET own_player_id=excluded.own_player_id,results_available=excluded.results_available;")?;
    }
    for (old, new, pairs) in LEGACY_TABLES {
        if !table(db, old)? {
            continue;
        }
        let available = columns(db, old)?;
        let pairs: Vec<_> = pairs
            .split(',')
            .map(|p| p.split_once(':').unwrap())
            .filter(|(a, _)| available.iter().any(|c| c == a))
            .collect();
        let select = pairs
            .iter()
            .map(|(a, b)| {
                if *b == "player_id" {
                    format!("nullif({a},0)")
                } else {
                    a.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join(",");
        db.execute_batch(&format!(
            "INSERT OR REPLACE INTO {new}({}) SELECT {select} FROM {old};",
            pairs.iter().map(|(_, b)| *b).collect::<Vec<_>>().join(",")
        ))?;
    }
    db.execute("DELETE FROM players WHERE account_id<=0", [])?;
    if old_sources {
        let has_version = columns(db, "legacy_sources")?
            .iter()
            .any(|s| s == "parser_version");
        let mut q = db.prepare(&format!(
            "SELECT path,size,modified,{} FROM legacy_sources",
            if has_version { "parser_version" } else { "0" }
        ))?;
        for r in q.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
            ))
        })? {
            let (path, size, modified, parser) = r?;
            db.execute(
                "INSERT INTO sources VALUES (?1,?2,?3,?4)",
                params![
                    source_key(Path::new(&path)),
                    size,
                    old_timestamp(&modified),
                    parser
                ],
            )?;
        }
    }
    if old_archive {
        db.execute_batch("INSERT INTO replay_archives SELECT path,original_size,captured_at,format_version FROM legacy_archives;")?;
    }
    for name in [
        "schaden_eingehend",
        "schaden_ausgehend",
        "gefecht_teilnehmer",
        "gefechte",
        "spieler",
        "replay_json",
        "replay_sql_index",
        "legacy_battles",
        "legacy_sources",
        "legacy_archives",
    ] {
        if table(db, name)? {
            db.execute_batch(&format!("DROP TABLE {name}"))?;
        }
    }
    db.execute_batch(include_str!("compatibility.sql"))?;
    db.execute_batch(crate::battle_record::JSON_VIEW)?;
    db.pragma_update(None, "user_version", VERSION)?;
    // New typed schema never references a second battle summary table.
    let violations: i64 =
        db.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| {
            r.get(0)
        })?;
    ensure!(
        violations == 0,
        "Fremdschlüsselprüfung fehlgeschlagen ({violations}); Migration wird zurückgerollt."
    );
    tx.commit()?;
    db.execute_batch("PRAGMA foreign_keys=ON;")?;
    if old_archive {
        // Release the former BLOB pages; the committed external archive remains intact.
        db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); VACUUM;")?;
    }
    Ok(())
}
const LEGACY_TABLES: &[(&str, &str, &str)] = &[
    (
        "spieler",
        "players",
        "account_id:account_id,echter_name:real_name",
    ),
    (
        "gefecht_teilnehmer",
        "battle_participants",
        "gefechts_id:battle_id,fahrzeug_id:vehicle_id,spieler_id:player_id,angezeigter_name:display_name,echter_name:real_name,alias:alias,panzer:tank,stufe:tier,team:team,gesamt_schaden:total_damage,schuesse:shots,treffer:hits,durchschlaege:penetrations,assist_aufklaerung:spotting_assist,assist_kette:tracking_assist,assist_betaeubung:stun_assist",
    ),
    (
        "schaden_eingehend",
        "incoming_damage",
        "gefechts_id:battle_id,eigenes_fahrzeug:own_vehicle,fahrzeug_id:vehicle_id,leben_index:life_index,schaden:damage,direkte_treffer:direct_hits,durchschlagstreffer:penetrating_hits,explosionstreffer:explosion_hits,munitionstyp:ammunition_type,geblockter_schaden:blocked_damage,zerstoert_von:killed_by",
    ),
    (
        "schaden_ausgehend",
        "outgoing_damage",
        "gefechts_id:battle_id,eigenes_fahrzeug:own_vehicle,fahrzeug_id:vehicle_id,leben_index:life_index,schaden:damage,direkte_treffer:direct_hits,durchschlagstreffer:penetrating_hits,explosionstreffer:explosion_hits,munitionstyp:ammunition_type",
    ),
    (
        "replay_json",
        "replay_documents",
        "replay_pfad:replay_path,block_index:block_index,payload:payload",
    ),
    (
        "replay_sql_index",
        "projection_index",
        "replay_pfad:replay_path,captured_at:captured_at,version:version",
    ),
];

pub fn source_key(path: &Path) -> String {
    let resolved = path.canonicalize().unwrap_or_else(|_| path.to_owned());
    let text = resolved.to_string_lossy().into_owned();
    #[cfg(windows)]
    {
        text.trim_start_matches(r"\\?\")
            .replace('\\', "/")
            .to_ascii_lowercase()
    }
    #[cfg(not(windows))]
    {
        text
    }
}
/// Attach the optional archive to the SQL browser without granting write access.
pub fn attach_readonly_archive(db: &Connection, path: &Path) -> Result<()> {
    let archive = archive_path(path);
    if archive.exists() {
        let file = archive
            .canonicalize()?
            .to_string_lossy()
            .trim_start_matches(r"\\?\")
            .replace('\\', "/");
        let uri = format!(
            "file:{}?mode=ro",
            file.replace('%', "%25")
                .replace('?', "%3F")
                .replace('#', "%23")
        );
        db.execute("ATTACH DATABASE ?1 AS archive_store", [uri])?;
    }
    Ok(())
}
