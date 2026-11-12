use std::{path::PathBuf, time::Instant};
fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).map(PathBuf::from).collect();
    anyhow::ensure!(
        args.len() == 3,
        "Usage: scan_benchmark <database> <game-directory> <replay-directory>"
    );
    let mut worker = wot_insights::scan::Worker::default();
    for n in 1..=3 {
        let start = Instant::now();
        let result = worker.scan(&args[0], &args[1], &args[2], true)?;
        println!(
            "Scan {n}: {:.1} ms; imported {}; unchanged {}; returned records {}; full reload {}; errors {}",
            start.elapsed().as_secs_f64() * 1000.,
            result.imported,
            result.unchanged,
            result.battles.len(),
            result.replace_all,
            result.errors.len()
        );
    }
    Ok(())
}
