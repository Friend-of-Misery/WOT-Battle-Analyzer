#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use anyhow::Result;
use eframe::egui;
use std::path::PathBuf;
use wot_insights::i18n;
mod details_view;
mod interaction_view;
mod maps_view;
mod sql_view;
mod ui;
use ui::{App, value};
use wot_insights::{stats, store::Store, vehicles};

fn paths() -> (PathBuf, PathBuf, PathBuf) {
    let game = std::env::var_os("WOT_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\games\World_of_Tanks_EU"));
    let replays = std::env::var_os("WOT_REPLAY_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| game.join("replays"));
    let db = std::env::var_os("WOT_INSIGHTS_DB")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("data"))
                .join("WoTInsights/battles.db")
        });
    (game, replays, db)
}
fn main() -> Result<()> {
    let (game, replays, database) = paths();
    if std::env::args().any(|a| a == "--import") {
        let catalog = vehicles::load(&game)?;
        let mut store = Store::open(&database)?;
        let report = store.import(&replays, &catalog)?;
        let battles = store.battles()?;
        let random: Vec<_> = battles.iter().filter(|b| b.mode == 1).collect();
        let s = stats::summarize(random.iter().copied());
        println!(
            "Vehicle catalog: {} | Imported: {} | Unchanged: {} | Errors: {}",
            catalog.len(),
            report.imported,
            report.unchanged,
            report.errors.len()
        );
        println!(
            "Random battles: {} | Results: {} | Winrate: {} | Avg damage: {}",
            s.count,
            s.known_results,
            value(s.winrate(), "%"),
            value(s.avg_damage(), "")
        );
        for error in report.errors.iter().take(20) {
            eprintln!("{error}");
        }
        let mut groups: Vec<_> = stats::maps_by_class(random.iter().copied())
            .into_iter()
            .collect();
        groups.sort_by_key(|a| std::cmp::Reverse(a.1.known_results));
        for ((map, class), s) in groups.iter().take(15) {
            println!(
                "{map} | {} | n={} | WR {} | DMG {}",
                vehicles::class_label(class),
                s.known_results,
                value(s.winrate(), "%"),
                value(s.avg_damage(), "")
            );
        }
        return Ok(());
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280., 820.])
            .with_min_inner_size([1000., 620.]),
        ..Default::default()
    };
    eframe::run_native(
        wot_insights::brand::NAME,
        options,
        Box::new(move |cc| {
            ui::configure(&cc.egui_ctx);
            Ok(Box::new(App::new(game, replays, database)))
        }),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))
}
