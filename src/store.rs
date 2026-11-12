use crate::{battle_record, migrations, replay::Battle, vehicles::Catalog};
use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use std::{collections::BTreeSet, fs, path::Path};
const PARSER_VERSION: i64 = 3;
pub struct Store {
    connection: Connection,
}
#[derive(Default, Debug)]
pub struct ImportReport {
    pub imported: usize,
    pub unchanged: usize,
    pub errors: Vec<String>,
    pub updated: Vec<Battle>,
    pub projections_changed: bool,
}
impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        Ok(Self {
            connection: migrations::open(path)?,
        })
    }
    pub fn battles(&self) -> Result<Vec<Battle>> {
        battle_record::all(&self.connection)
    }
    pub fn battle_document(&self, battle: &Battle) -> Result<crate::replay_data::Document> {
        let mut indexed = self.connection.prepare("SELECT a.path,b2.payload FROM replay_archive_battles b JOIN replay_archives a ON a.path=b.path JOIN archive_store.replay_blobs b2 ON b2.path=a.path WHERE b.identity=?1 ORDER BY a.original_size DESC")?;
        let mut rows = indexed.query([&battle.key])?;
        while let Some(row) = rows.next()? {
            let path: String = row.get(0)?;
            let payload: Vec<u8> = row.get(1)?;
            if let Ok(document) = crate::archive::document(&payload, path)
                && document
                    .blocks
                    .first()
                    .and_then(|h| crate::replay_data::identity(h).ok())
                    .as_ref()
                    == Some(&battle.key)
            {
                return Ok(document);
            }
        }
        // Older archives have no identity index. Filename narrows the common case,
        // but the full identity is always verified before displaying any data.
        let date = chrono::NaiveDateTime::parse_from_str(&battle.date, "%Y-%m-%d %H:%M:%S")?;
        let pattern = format!("%{}%", date.format("%Y%m%d_%H%M"));
        let mut sources = self
            .connection
            .prepare("SELECT path FROM sources WHERE path LIKE ?1")?;
        for path in sources.query_map([&pattern], |r| r.get::<_, String>(0))? {
            let path = path?;
            if let Ok(bytes) = fs::read(&path)
                && let Ok(document) = crate::replay_data::read(&bytes, path)
                && document
                    .blocks
                    .first()
                    .and_then(|h| crate::replay_data::identity(h).ok())
                    .as_ref()
                    == Some(&battle.key)
            {
                return Ok(document);
            }
        }
        let mut archives = self.connection.prepare("SELECT a.path,b.payload FROM replay_archives a JOIN archive_store.replay_blobs b ON b.path=a.path ORDER BY (a.path LIKE ?1) DESC,a.original_size DESC")?;
        let mut rows = archives.query([pattern])?;
        while let Some(row) = rows.next()? {
            let path: String = row.get(0)?;
            let payload: Vec<u8> = row.get(1)?;
            if let Ok(document) = crate::archive::document(&payload, path.clone())
                && document
                    .blocks
                    .first()
                    .and_then(|h| crate::replay_data::identity(h).ok())
                    .as_ref()
                    == Some(&battle.key)
            {
                let _ = self.connection.execute(
                    "INSERT OR REPLACE INTO replay_archive_battles(path,identity) VALUES (?1,?2)",
                    params![path, battle.key],
                );
                return Ok(document);
            }
        }
        anyhow::bail!(
            "Für dieses Gefecht ist noch kein vollständiges Replay-Archiv verfügbar. Bitte die vorhandenen Replays über Jetzt einlesen importieren. Bereits gespeicherte Statistikwerte bleiben sichtbar."
        )
    }

    pub fn import(&mut self, folder: &Path, catalog: &Catalog) -> Result<ImportReport> {
        let mut report = ImportReport::default();
        let mut paths = Vec::new();
        collect(folder, &mut paths)?;
        let mut changed = BTreeSet::new();
        let tx = self.connection.transaction()?;
        // Read the small source index once, not a SQL query for every unchanged file.
        let indexed = {
            let mut q = tx.prepare("SELECT path,size,modified_ns,parser_version FROM sources")?;
            q.query_map([], |r| {
                Ok((
                    migrations::source_key(Path::new(&r.get::<_, String>(0)?)),
                    (
                        r.get::<_, i64>(1)?,
                        r.get::<_, Option<i64>>(2)?,
                        r.get::<_, i64>(3)?,
                    ),
                ))
            })?
            .collect::<rusqlite::Result<std::collections::HashMap<_, _>>>()?
        };
        for path in paths {
            let result = (|| -> Result<()> {
                let metadata = path.metadata()?;
                let modified = migrations::timestamp(metadata.modified()?)?;
                let path_text = migrations::source_key(&path);
                if indexed
                    .get(&path_text)
                    .is_some_and(|(size, time, version)| {
                        *size == metadata.len() as i64
                            && *time == Some(modified)
                            && *version == PARSER_VERSION
                    })
                {
                    report.unchanged += 1;
                    return Ok(());
                }
                let bytes = fs::read(&path)?;
                let after = path.metadata()?;
                anyhow::ensure!(
                    after.len() == metadata.len() && after.modified()? == metadata.modified()?,
                    "Replay is still being written; retry later"
                );
                crate::archive::capture(&tx, &path, &bytes)?;
                let doc = crate::replay_data::read(&bytes, path_text.to_string())?;
                let battle = crate::replay::from_json(
                    doc.blocks.first().context("Replay header missing")?,
                    doc.blocks.get(1),
                    catalog,
                )?;
                if battle_record::save(&tx, &battle)? {
                    changed.insert(battle.key);
                }
                tx.execute("INSERT INTO sources VALUES (?1,?2,?3,?4) ON CONFLICT(path) DO UPDATE SET size=excluded.size,modified_ns=excluded.modified_ns,parser_version=excluded.parser_version",params![path_text,metadata.len(),modified,PARSER_VERSION])?;
                report.imported += 1;
                Ok(())
            })();
            if let Err(e) = result {
                report.errors.push(format!("{}: {e:#}", path.display()));
            }
        }
        refresh_catalog(&tx, catalog, &mut changed)?;
        tx.commit()?;
        let (projected, errors) = crate::damage::backfill(&self.connection, catalog)?;
        report.projections_changed = !projected.is_empty();
        changed.extend(projected);
        report.errors.extend(errors);
        for key in changed {
            if let Some(battle) = battle_record::get(&self.connection, &key)? {
                report.updated.push(battle);
            }
        }
        Ok(report)
    }
}
fn refresh_catalog(
    db: &Connection,
    catalog: &Catalog,
    changed: &mut BTreeSet<String>,
) -> Result<()> {
    let mut entries: Vec<_> = catalog
        .iter()
        .map(|(k, v)| (k, &v.class, v.tier, &v.role))
        .collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    let signature = serde_json::to_string(&entries)?;
    let previous: Option<String> = db
        .query_row("SELECT value FROM app_meta WHERE key='catalog'", [], |r| {
            r.get(0)
        })
        .ok();
    if previous.as_deref() == Some(&signature) {
        return Ok(());
    }
    for (tank, class, tier, role) in entries {
        let mut q = db.prepare_cached(
            "SELECT battle_id FROM battles WHERE tank=?1 AND (class<>?2 OR tier<>?3 OR role<>?4)",
        )?;
        for key in q.query_map(params![tank, class, tier, role], |r| r.get::<_, String>(0))? {
            changed.insert(key?);
        }
        db.execute("UPDATE battles SET class=?2,tier=?3,role=?4 WHERE tank=?1 AND (class<>?2 OR tier<>?3 OR role<>?4)",params![tank,class,tier,role])?;
        db.execute(
            "UPDATE battle_participants SET tier=?2 WHERE tank=?1 AND (tier IS NULL OR tier<>?2)",
            params![tank, tier],
        )?;
    }
    db.execute("INSERT INTO app_meta VALUES ('catalog',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[signature])?;
    Ok(())
}
fn collect(folder: &Path, paths: &mut Vec<std::path::PathBuf>) -> Result<()> {
    for entry in
        fs::read_dir(folder).with_context(|| format!("Replay-Ordner: {}", folder.display()))?
    {
        let entry = entry?;
        let name = entry.file_name();
        if matches!(
            name.to_string_lossy().to_ascii_lowercase().as_str(),
            "temp.wotreplay" | "replay_last_battle.wotreplay"
        ) {
            continue;
        }
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            collect(&entry.path(), paths)?;
        } else if entry
            .path()
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("wotreplay"))
        {
            paths.push(entry.path());
        }
    }
    Ok(())
}
