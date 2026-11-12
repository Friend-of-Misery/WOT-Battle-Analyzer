//! Lossless replay capture. Unknown JSON fields and binary packets are retained.
use anyhow::{Context, Result, ensure};
use chrono::{Duration, Local, NaiveDateTime};
use rusqlite::{Connection, params};
use std::{
    fs,
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
    time::SystemTime,
};
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

#[derive(Clone, Debug)]
pub struct Candidate {
    pub path: PathBuf,
    pub size: u64,
    pub modified: SystemTime,
    pub date: String,
}

fn pack(bytes: &[u8]) -> Result<Vec<u8>> {
    if let Ok(archive) = pack_blocks(bytes) {
        return Ok(archive);
    }
    // Future/partial formats must remain recoverable even when no JSON decoder applies.
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    zip.start_file(
        "original.wotreplay",
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
    )?;
    zip.write_all(bytes)?;
    Ok(zip.finish()?.into_inner())
}

fn pack_blocks(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut input = Cursor::new(bytes);
    let mut word = [0; 4];
    input.read_exact(&mut word)?;
    ensure!(
        u32::from_le_bytes(word) == 0x1134_3212,
        "Ungültige Replay-Signatur"
    );
    input.read_exact(&mut word)?;
    let count = u32::from_le_bytes(word);
    ensure!((1..=16).contains(&count), "Unbekannte Replay-Blockanzahl");
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let compressed =
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("header.bin", compressed)?;
    zip.write_all(&bytes[..8])?;
    for index in 0..count {
        input.read_exact(&mut word)?;
        let size = u32::from_le_bytes(word) as usize;
        ensure!(size <= 16 * 1024 * 1024, "JSON-Block zu groß");
        let mut block = vec![0; size];
        input.read_exact(&mut block)?;
        let _: serde_json::Value = serde_json::from_slice(&block)?;
        zip.start_file(format!("block-{index}.json"), compressed)?;
        zip.write_all(&block)?;
    }
    // The original stream is already encrypted/compressed. Preserve every remaining byte.
    zip.start_file(
        "stream.bin",
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
    )?;
    zip.write_all(&bytes[input.position() as usize..])?;
    Ok(zip.finish()?.into_inner())
}

fn unpack(archive: &[u8]) -> Result<Vec<u8>> {
    let mut zip = ZipArchive::new(Cursor::new(archive))?;
    if let Ok(mut original) = zip.by_name("original.wotreplay") {
        let mut bytes = Vec::new();
        original.read_to_end(&mut bytes)?;
        return Ok(bytes);
    }
    let mut bytes = Vec::new();
    zip.by_name("header.bin")?.read_to_end(&mut bytes)?;
    ensure!(bytes.len() == 8, "Ungültiger Archivkopf");
    let count = u32::from_le_bytes(bytes[4..8].try_into()?);
    ensure!((1..=16).contains(&count), "Ungültige Archiv-Blockanzahl");
    for index in 0..count {
        let mut block = Vec::new();
        zip.by_name(&format!("block-{index}.json"))?
            .read_to_end(&mut block)?;
        bytes.extend_from_slice(&u32::try_from(block.len())?.to_le_bytes());
        bytes.extend_from_slice(&block);
    }
    zip.by_name("stream.bin")?.read_to_end(&mut bytes)?;
    Ok(bytes)
}

pub fn capture(connection: &Connection, path: &Path, bytes: &[u8]) -> Result<()> {
    let path = path.canonicalize()?;
    let archive = pack(bytes)?;
    ensure!(unpack(&archive)? == bytes, "Archivprüfung fehlgeschlagen");
    let captured = Local::now().to_rfc3339();
    connection.execute("INSERT INTO archive_store.replay_blobs VALUES (?1,?2,?3) ON CONFLICT(path) DO UPDATE SET captured_at=excluded.captured_at,payload=excluded.payload",params![path.to_string_lossy(),captured,archive])?;
    connection.execute(
        "INSERT OR REPLACE INTO replay_archives VALUES (?1,?2,?3,1)",
        params![path.to_string_lossy(), bytes.len() as i64, captured],
    )?;
    if let Ok(document) = crate::replay_data::read(bytes, String::new())
        && let Some(header) = document.blocks.first()
        && let Ok(identity) = crate::replay_data::identity(header)
    {
        connection.execute(
            "INSERT OR REPLACE INTO replay_archive_battles(path,identity) VALUES (?1,?2)",
            params![path.to_string_lossy(), identity],
        )?;
    }
    Ok(())
}

pub fn document(payload: &[u8], source: String) -> Result<crate::replay_data::Document> {
    crate::replay_data::read(&unpack(payload)?, source)
}

pub fn preview(folder: &Path) -> Result<Vec<Candidate>> {
    let root = folder
        .canonicalize()
        .context("Replay-Ordner konnte nicht geöffnet werden")?;
    let cutoff = Local::now().naive_local() - Duration::days(30);
    let mut pending = vec![root.clone()];
    let mut candidates = Vec::new();
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                let directory = entry.path().canonicalize()?;
                if directory.starts_with(&root) {
                    pending.push(directory);
                }
                continue;
            }
            if !kind.is_file()
                || !entry
                    .path()
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("wotreplay"))
            {
                continue;
            }
            let path = entry.path().canonicalize()?;
            ensure!(
                path.starts_with(&root),
                "Datei außerhalb des Replay-Ordners"
            );
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            if name == "temp.wotreplay" || name == "replay_last_battle.wotreplay" {
                continue;
            }
            let date = (|| -> Result<NaiveDateTime> {
                let mut file = fs::File::open(&path)?;
                let mut head = [0; 12];
                file.read_exact(&mut head)?;
                ensure!(
                    u32::from_le_bytes(head[..4].try_into()?) == 0x1134_3212,
                    "Signatur"
                );
                let size = u32::from_le_bytes(head[8..12].try_into()?) as usize;
                ensure!(size <= 16 * 1024 * 1024, "Blockgröße");
                let mut json = vec![0; size];
                file.read_exact(&mut json)?;
                let value: serde_json::Value = serde_json::from_slice(&json)?;
                Ok(NaiveDateTime::parse_from_str(
                    value["dateTime"].as_str().context("Gefechtsdatum fehlt")?,
                    "%d.%m.%Y %H:%M:%S",
                )?)
            })();
            if let Ok(date) = date
                && date < cutoff
            {
                let metadata = path.metadata()?;
                candidates.push(Candidate {
                    path,
                    size: metadata.len(),
                    modified: metadata.modified()?,
                    date: date.to_string(),
                });
            }
        }
    }
    candidates.sort_by(|a, b| a.date.cmp(&b.date).then(a.path.cmp(&b.path)));
    Ok(candidates)
}

pub fn delete_archived(
    database: &Path,
    folder: &Path,
    candidates: &[Candidate],
) -> Result<(usize, u64, Vec<String>)> {
    let root = folder.canonicalize()?;
    let connection = crate::migrations::open(database)?;
    connection.execute_batch("PRAGMA synchronous=FULL;")?;
    let mut deleted = 0;
    let mut freed = 0;
    let mut errors = Vec::new();
    for candidate in candidates {
        let result = (|| -> Result<()> {
            let path = candidate.path.canonicalize()?;
            ensure!(
                path.starts_with(&root),
                "Datei außerhalb des geprüften Ordners"
            );
            ensure!(
                !fs::symlink_metadata(&candidate.path)?
                    .file_type()
                    .is_symlink(),
                "Verknüpfung wird nicht gelöscht"
            );
            let meta = path.metadata()?;
            ensure!(
                meta.len() == candidate.size && meta.modified()? == candidate.modified,
                "Datei seit Vorschau geändert"
            );
            let bytes = fs::read(&path)?;
            // Always archive first, including not-yet-imported and incomplete battles.
            capture(&connection, &path, &bytes)?;
            let saved: Vec<u8> = connection.query_row(
                "SELECT payload FROM archive_store.replay_blobs WHERE path=?1",
                [path.to_string_lossy()],
                |row| row.get(0),
            )?;
            ensure!(
                unpack(&saved)? == bytes,
                "Gespeichertes Archiv stimmt nicht überein"
            );
            ensure!(
                fs::read(&path)? == bytes && path.metadata()?.modified()? == candidate.modified,
                "Datei während Sicherung geändert"
            );
            fs::remove_file(&path)?;
            Ok(())
        })();
        match result {
            Ok(()) => {
                deleted += 1;
                freed += candidate.size;
            }
            Err(error) => errors.push(format!("{}: {error:#}", candidate.path.display())),
        }
    }
    Ok((deleted, freed, errors))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn replay() -> Vec<u8> {
        let mut bytes = 0x1134_3212u32.to_le_bytes().to_vec();
        bytes.extend_from_slice(&2u32.to_le_bytes());
        for block in [
            br#"{"dateTime":"01.01.2020 12:00:00","unknown":{"future":[1,2]}}"#.as_slice(),
            br#"{"personal":{},"unknown":true}"#,
        ] {
            bytes.extend_from_slice(&(block.len() as u32).to_le_bytes());
            bytes.extend_from_slice(block);
        }
        bytes.extend_from_slice(&[0, 255, 4, 8, 0, 127]);
        bytes
    }
    #[test]
    fn archive_preserves_unknown_fields_and_binary_bytes_exactly() {
        let bytes = replay();
        assert_eq!(unpack(&pack(&bytes).unwrap()).unwrap(), bytes);
        let partial = &bytes[..bytes.len() - 12];
        assert_eq!(unpack(&pack(partial).unwrap()).unwrap(), partial);
    }
    #[test]
    fn deletion_archives_first_and_skips_changed_files() {
        let temp = tempfile::tempdir().unwrap();
        let folder = temp.path().join("replays");
        fs::create_dir(&folder).unwrap();
        let path = folder.join("old.wotreplay");
        let bytes = replay();
        fs::write(&path, &bytes).unwrap();
        let candidates = preview(&folder).unwrap();
        assert_eq!(candidates.len(), 1);
        let db = temp.path().join("archive.db");
        let (deleted, _, errors) = delete_archived(&db, &folder, &candidates).unwrap();
        assert_eq!(deleted, 1);
        assert!(errors.is_empty());
        assert!(!path.exists());
        let connection = crate::migrations::open(&db).unwrap();
        let saved: Vec<u8> = connection
            .query_row("SELECT payload FROM archive_store.replay_blobs", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(unpack(&saved).unwrap(), bytes);
        fs::write(&path, &bytes).unwrap();
        let candidates = preview(&folder).unwrap();
        fs::write(&path, b"changed").unwrap();
        let (deleted, _, errors) = delete_archived(&db, &folder, &candidates).unwrap();
        assert_eq!(deleted, 0);
        assert_eq!(errors.len(), 1);
        assert!(path.exists());
    }
}
