use anyhow::Result;
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let (language, names) = wot_insights::vehicle_names::load(std::path::Path::new(&args[1]))?;
    let path = std::path::Path::new(&args[2]);
    let mut catalogs: wot_insights::vehicle_names::Names = std::fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    println!("{}: {} official vehicle names", language, names.len());
    catalogs.insert(language, names);
    std::fs::write(path, serde_json::to_string_pretty(&catalogs)? + "\n")?;
    Ok(())
}
