//! Official vehicle display names, kept separate from immutable replay identifiers.
use anyhow::{Context, Result, ensure};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{OnceLock, RwLock},
};
pub type Names = BTreeMap<String, BTreeMap<String, String>>;
fn catalog() -> &'static RwLock<Names> {
    static NAMES: OnceLock<RwLock<Names>> = OnceLock::new();
    NAMES.get_or_init(|| {
        RwLock::new(
            serde_json::from_str(include_str!("locales/vehicle_names.json"))
                .expect("validated vehicle names"),
        )
    })
}
pub fn display(tank: &str, language: &str) -> Option<String> {
    let normalized = tank.replacen(':', "-", 1);
    let names = catalog().read().ok()?;
    [language, "en", "de"]
        .into_iter()
        .find_map(|lang| names.get(lang)?.get(&normalized).cloned())
}
pub fn source_language(language: &str) -> Option<String> {
    let names = catalog().read().ok()?;
    [language, "en", "de"]
        .into_iter()
        .find(|lang| names.contains_key(*lang))
        .map(str::to_owned)
}
/// Decode GNU gettext MO catalogs without translating or approximating names.
pub fn read_mo(bytes: &[u8]) -> Result<BTreeMap<String, String>> {
    ensure!(bytes.len() >= 28, "MO header truncated");
    let little = match &bytes[..4] {
        [0xde, 0x12, 0x04, 0x95] => true,
        [0x95, 0x04, 0x12, 0xde] => false,
        _ => anyhow::bail!("Invalid MO signature"),
    };
    let word = |offset: usize| -> Result<usize> {
        let b: [u8; 4] = bytes
            .get(offset..offset.checked_add(4).context("MO offset overflow")?)
            .context("MO offset outside file")?
            .try_into()?;
        Ok(if little {
            u32::from_le_bytes(b)
        } else {
            u32::from_be_bytes(b)
        } as usize)
    };
    let count = word(8)?;
    let originals = word(12)?;
    let translations = word(16)?;
    ensure!(count <= 200_000, "MO entry count too large");
    let string = |table: usize, index: usize| -> Result<String> {
        let slot = table
            .checked_add(index.checked_mul(8).context("MO table overflow")?)
            .context("MO table overflow")?;
        let len = word(slot)?;
        let offset = word(slot + 4)?;
        let end = offset.checked_add(len).context("MO string overflow")?;
        Ok(
            std::str::from_utf8(bytes.get(offset..end).context("MO string outside file")?)?
                .split('\0')
                .next()
                .unwrap_or("")
                .to_owned(),
        )
    };
    let mut names = BTreeMap::new();
    for i in 0..count {
        let key = string(originals, i)?;
        let value = string(translations, i)?;
        if !key.is_empty() && !value.is_empty() {
            names.insert(key, value);
        }
    }
    Ok(names)
}
pub fn load(game: &Path) -> Result<(String, BTreeMap<String, String>)> {
    let version = std::fs::read_to_string(game.join("loc_version.xml"))?;
    let language = version
        .split("<language>")
        .nth(1)
        .and_then(|s| s.split("</language>").next())
        .context("Client language missing")?
        .trim()
        .to_lowercase();
    ensure!(
        crate::i18n::valid_language(&language),
        "Unsupported client language"
    );
    let refs = crate::vehicles::name_references(game)?;
    let directory = game.join("res/text/lc_messages");
    let mut domains = BTreeMap::new();
    let mut names = BTreeMap::new();
    for (tank, reference) in refs {
        let Some((domain, key)) = reference.trim_start_matches('#').split_once(':') else {
            continue;
        };
        if !domains.contains_key(domain) {
            let path = directory.join(format!("{domain}.mo"));
            let data = match std::fs::read(&path) {
                Ok(bytes) => read_mo(&bytes)?,
                Err(_) => BTreeMap::new(),
            };
            domains.insert(domain.to_owned(), data);
        }
        if let Some(name) = domains.get(domain).and_then(|d| d.get(key)) {
            names.insert(tank, name.clone());
        }
    }
    ensure!(!names.is_empty(), "No official vehicle names found");
    Ok((language, names))
}
/// Recheck file metadata; reload names only after the client localization changes.
pub fn refresh(game: &Path) -> Result<()> {
    static SIGNATURE: OnceLock<std::sync::Mutex<String>> = OnceLock::new();
    let version = game.join("loc_version.xml");
    if !version.exists() {
        return Ok(());
    }
    let mut paths = vec![version, game.join("res/packages/scripts.pkg")];
    paths.extend(
        std::fs::read_dir(game.join("res/text/lc_messages"))?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|e| e == "mo")),
    );
    paths.sort();
    let mut signature = game.to_string_lossy().into_owned();
    for path in paths {
        let m = std::fs::metadata(&path)?;
        signature.push_str(&format!(
            "{}:{}:{:?};",
            path.display(),
            m.len(),
            m.modified()?
        ));
    }
    let mut previous = SIGNATURE
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| anyhow::anyhow!("Vehicle names cache unavailable"))?;
    if *previous == signature {
        return Ok(());
    }
    let (language, names) = load(game)?;
    catalog()
        .write()
        .map_err(|_| anyhow::anyhow!("Vehicle names cache unavailable"))?
        .entry(language)
        .or_default()
        .extend(names);
    *previous = signature;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn mo(little: bool) -> Vec<u8> {
        let strings = [("A67_T57_58", "T57 Heavy"), ("R04_T-34", "Т-34")];
        let count = strings.len();
        let mut bytes = vec![0u8; 28 + count * 16];
        let write = |bytes: &mut Vec<u8>, offset: usize, value: u32| {
            bytes[offset..offset + 4].copy_from_slice(&if little {
                value.to_le_bytes()
            } else {
                value.to_be_bytes()
            })
        };
        write(&mut bytes, 0, 0x950412de);
        write(&mut bytes, 8, count as u32);
        write(&mut bytes, 12, 28);
        write(&mut bytes, 16, (28 + count * 8) as u32);
        for (i, (key, value)) in strings.iter().enumerate() {
            for (table, text) in [(28, *key), (28 + count * 8, *value)] {
                let offset = bytes.len();
                write(&mut bytes, table + i * 8, text.len() as u32);
                write(&mut bytes, table + i * 8 + 4, offset as u32);
                bytes.extend_from_slice(text.as_bytes());
                bytes.push(0);
            }
        }
        bytes
    }
    #[test]
    fn gettext_endianness_unicode_and_invalid_offsets() {
        for little in [true, false] {
            let bytes = mo(little);
            let names = read_mo(&bytes).unwrap();
            assert_eq!(names["A67_T57_58"], "T57 Heavy");
            assert_eq!(names["R04_T-34"], "Т-34");
            assert!(read_mo(&bytes[..40]).is_err());
        }
        assert!(read_mo(&[0; 28]).is_err());
    }
    #[test]
    fn official_names_replace_internal_codes_and_missing_language_has_explicit_fallback() {
        assert_eq!(
            display("usa-A67_T57_58", "de").as_deref(),
            Some("T57 Heavy")
        );
        assert_eq!(
            display("usa:A67_T57_58", "de").as_deref(),
            Some("T57 Heavy")
        );
        assert_eq!(source_language("de").as_deref(), Some("de"));
        assert_eq!(display("not-a-real-vehicle", "de"), None);
        let names: Names =
            serde_json::from_str(include_str!("locales/vehicle_names.json")).unwrap();
        assert!(names["de"].len() > 1200);
        assert!(
            names
                .values()
                .flat_map(|m| m.values())
                .all(|n| !n.trim().is_empty() && !n.starts_with('#'))
        );
    }
}
