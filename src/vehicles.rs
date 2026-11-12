//! Read vehicle classes and tiers from the user's own client, without an API key.
use anyhow::{Context, Result, bail, ensure};
use std::{collections::HashMap, fs::File, io::Read, path::Path};

#[derive(Clone, Debug)]
pub struct Vehicle {
    pub class: String,
    pub tier: u8,
    pub role: String,
}

pub type Catalog = HashMap<String, Vehicle>;

fn u16_at(b: &[u8], p: usize) -> Result<usize> {
    Ok(u16::from_le_bytes(b.get(p..p + 2).context("Truncated XML field")?.try_into()?) as usize)
}
fn u32_at(b: &[u8], p: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        b.get(p..p + 4).context("Truncated XML field")?.try_into()?,
    ))
}

// BigWorld packed XML: a string dictionary followed by nodes with end-offset descriptors.
#[derive(Debug)]
enum Node {
    Children(HashMap<String, Node>),
    Text(String),
    Integer(u32),
    Other,
}
fn read_node(b: &[u8], names: &[String], depth: usize) -> Result<Node> {
    ensure!(depth < 64, "XML nesting exceeds limit");
    let count = u16_at(b, 0)?;
    let start = 6 + count * 6;
    ensure!(start <= b.len(), "Truncated XML node");
    let mut previous = (u32_at(b, 2)? & 0x0fff_ffff) as usize;
    let mut children = HashMap::new();
    for i in 0..count {
        let name = names
            .get(u16_at(b, 6 + i * 6)?)
            .context("Invalid XML dictionary index")?;
        let descriptor = u32_at(b, 8 + i * 6)?;
        let end = (descriptor & 0x0fff_ffff) as usize;
        ensure!(end >= previous, "Reversed XML offsets");
        let payload = b
            .get(start + previous..start + end)
            .context("Invalid XML payload")?;
        let child = match descriptor >> 28 {
            0 => read_node(payload, names, depth + 1)?,
            1 => Node::Text(std::str::from_utf8(payload)?.to_owned()),
            2 if payload.len() <= 4 => {
                let mut value = [0; 4];
                value[..payload.len()].copy_from_slice(payload);
                Node::Integer(u32::from_le_bytes(value))
            }
            _ => Node::Other,
        };
        children.insert(name.clone(), child);
        previous = end;
    }
    Ok(Node::Children(children))
}
fn packed_xml(b: &[u8]) -> Result<Node> {
    ensure!(
        b.starts_with(&[0x45, 0x4e, 0xa1, 0x62, 0]),
        "Unknown packed XML format"
    );
    let mut pos = 5;
    let mut names = Vec::new();
    loop {
        let len = b
            .get(pos..)
            .context("Missing XML dictionary")?
            .iter()
            .position(|x| *x == 0)
            .context("Unterminated XML dictionary")?;
        if len == 0 {
            pos += 1;
            break;
        }
        names.push(std::str::from_utf8(&b[pos..pos + len])?.to_owned());
        pos += len + 1;
    }
    read_node(&b[pos..], &names, 0)
}

pub fn load(game: &Path) -> Result<Catalog> {
    let path = game.join("res/packages/scripts.pkg");
    let mut archive =
        zip::ZipArchive::new(File::open(&path).with_context(|| path.display().to_string())?)?;
    let mut catalog = Catalog::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let name = entry.name().to_owned();
        let parts: Vec<_> = name.split('/').collect();
        if parts.len() != 5
            || parts[..3] != ["scripts", "item_defs", "vehicles"]
            || parts[4] != "list.xml"
        {
            continue;
        }
        ensure!(entry.size() < 16 * 1024 * 1024, "Vehicle list too large");
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes)?;
        let Node::Children(tanks) = packed_xml(&bytes)? else {
            bail!("Invalid vehicle list")
        };
        for (tank, node) in tanks {
            let Node::Children(fields) = node else {
                continue;
            };
            let Some(Node::Text(tags)) = fields.get("tags") else {
                continue;
            };
            let class = ["lightTank", "mediumTank", "heavyTank", "AT-SPG", "SPG"]
                .into_iter()
                .find(|class| tags.split_whitespace().any(|tag| tag == *class));
            let Some(class) = class else { continue };
            let Some(Node::Integer(tier)) = fields.get("level") else {
                continue;
            };
            catalog.insert(
                format!("{}-{tank}", parts[3]),
                Vehicle {
                    class: class.to_owned(),
                    tier: u8::try_from(*tier)?,
                    role: tags
                        .split_whitespace()
                        .find(|tag| tag.starts_with("role_"))
                        .unwrap_or("")
                        .to_owned(),
                },
            );
        }
    }
    ensure!(!catalog.is_empty(), "No vehicle classes found in client");
    Ok(catalog)
}

pub fn class_label(class: &str) -> &str {
    match class {
        "lightTank" => "Light",
        "mediumTank" => "Medium",
        "heavyTank" => "Heavy",
        "AT-SPG" => "TD",
        "SPG" => "Arty",
        _ => "Unbekannt",
    }
}

pub const NATIONS: [(&str, &str); 11] = [
    ("germany", "Deutschland"),
    ("usa", "USA"),
    ("ussr", "UdSSR"),
    ("uk", "Großbritannien"),
    ("france", "Frankreich"),
    ("china", "China"),
    ("japan", "Japan"),
    ("czech", "Tschechoslowakei"),
    ("poland", "Polen"),
    ("sweden", "Schweden"),
    ("italy", "Italien"),
];
pub const ROLE_GROUPS: [(&str, &str); 8] = [
    ("assault", "Sturm"),
    ("support", "Unterstützung"),
    ("universal", "Mehrzweck"),
    ("break", "Durchbruch"),
    ("sniper", "Scharfschütze"),
    ("scout", "Aufklärung"),
    ("SPG", "Artillerie"),
    ("unknown", "Nicht zugeordnet"),
];
pub fn nation(tank: &str) -> &str {
    tank.split_once('-')
        .map(|(nation, _)| nation)
        .unwrap_or("unknown")
}
pub fn nation_label(nation: &str) -> &str {
    NATIONS
        .iter()
        .find(|(code, _)| *code == nation)
        .map(|(_, label)| *label)
        .unwrap_or("Unbekannt")
}
pub fn role_group(role: &str) -> &str {
    if role == "role_SPG" {
        return "SPG";
    }
    match role.rsplit_once('_').map(|(_, group)| group) {
        Some(group @ ("assault" | "support" | "universal" | "break" | "sniper" | "scout")) => group,
        _ => "unknown",
    }
}
pub fn role_label(role: &str) -> &str {
    ROLE_GROUPS
        .iter()
        .find(|(code, _)| *code == role_group(role))
        .map(|(_, label)| *label)
        .unwrap_or("Nicht zugeordnet")
}

/// Reuse the vehicle catalog until the package or selected game directory changes.
pub fn load_cached(game: &Path) -> Result<std::sync::Arc<Catalog>> {
    cached(game, || load(game))
}
fn cached(
    game: &Path,
    loader: impl FnOnce() -> Result<Catalog>,
) -> Result<std::sync::Arc<Catalog>> {
    use std::{
        path::PathBuf,
        sync::{Arc, Mutex, OnceLock},
        time::SystemTime,
    };
    type Cached = HashMap<PathBuf, (u64, SystemTime, Arc<Catalog>)>;
    static CACHE: OnceLock<Mutex<Cached>> = OnceLock::new();
    let package = game.join("res/packages/scripts.pkg").canonicalize()?;
    let metadata = std::fs::metadata(&package)?;
    let modified = metadata.modified()?;
    let mut cache = CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| anyhow::anyhow!("Fahrzeugkatalog-Cache konnte nicht gesperrt werden"))?;
    if let Some((size, time, catalog)) = cache.get(&package)
        && *size == metadata.len()
        && *time == modified
    {
        return Ok(Arc::clone(catalog));
    }
    let catalog = Arc::new(loader()?);
    cache.insert(package, (metadata.len(), modified, Arc::clone(&catalog)));
    Ok(catalog)
}

#[cfg(test)]
mod cache_tests {
    use super::*;
    #[test]
    fn package_change_invalidates_cache() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("res/packages/scripts.pkg");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, b"first").unwrap();
        let a = cached(temp.path(), || Ok(Catalog::new())).unwrap();
        let b = cached(temp.path(), || {
            panic!("unchanged package must not be parsed")
        })
        .unwrap();
        assert!(std::sync::Arc::ptr_eq(&a, &b));
        std::fs::write(&file, b"second package").unwrap();
        let c = cached(temp.path(), || Ok(Catalog::new())).unwrap();
        assert!(!std::sync::Arc::ptr_eq(&a, &c));
    }
}
