use anyhow::{Context, Result};
use chrono::Local;
use eframe::egui::{self, Color32, RichText, Stroke, Vec2};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};
use wot_insights::{
    analysis::{self, Filter, TankRow},
    replay::Battle,
    stats::{self, Summary},
    vehicles,
};

pub(super) const ACCENT: Color32 = Color32::from_rgb(65, 190, 255);
pub(super) const GREEN: Color32 = Color32::from_rgb(55, 235, 115);
pub(super) const RED: Color32 = Color32::from_rgb(255, 100, 105);
const MUTED: Color32 = Color32::from_rgb(190, 201, 216);
const VIEWS: [&str; 12] = [
    "Übersicht",
    "Panzer",
    "Gefechte",
    "Karten",
    "Verlauf",
    "Einstellungen",
    "Verdienst",
    "Replay-Archiv",
    "SQL",
    "Schaden",
    "Spielersuche",
    "Info",
];

pub fn configure(ctx: &egui::Context) {
    // Use the installed Windows UI font; keep egui's fonts as glyph fallbacks.
    static UI_FONT: std::sync::OnceLock<Option<std::sync::Arc<egui::FontData>>> =
        std::sync::OnceLock::new();
    let font = UI_FONT.get_or_init(|| {
        let windows = std::env::var_os("WINDIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("C:/Windows"));
        std::fs::read(windows.join("Fonts/segoeui.ttf"))
            .ok()
            .map(|bytes| std::sync::Arc::new(egui::FontData::from_owned(bytes)))
    });
    if let Some(font) = font {
        let mut fonts = egui::FontDefinitions::default();
        fonts
            .font_data
            .insert("Segoe UI".into(), std::sync::Arc::clone(font));
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "Segoe UI".into());
        ctx.set_fonts(fonts);
    }
    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.panel_fill = Color32::from_rgb(27, 30, 35);
    style.visuals.window_fill = Color32::from_rgb(32, 36, 42);
    style.visuals.override_text_color = Some(Color32::from_rgb(237, 241, 247));
    style.visuals.weak_text_color = Some(MUTED);
    style.visuals.hyperlink_color = ACCENT;
    style.visuals.selection.bg_fill = Color32::from_rgb(48, 87, 116);
    style.spacing.item_spacing = Vec2::new(9., 7.);
    style.spacing.button_padding = Vec2::new(11., 6.);
    ctx.set_style(style);
    typography(ctx, 17.);
}
fn typography(ctx: &egui::Context, size: f32) {
    let size = if size.is_finite() {
        size.clamp(14., 22.)
    } else {
        17.
    };
    let mut style = (*ctx.style()).clone();
    for (text_style, points) in [
        (egui::TextStyle::Body, size),
        (egui::TextStyle::Button, size),
        (egui::TextStyle::Small, size - 2.),
        (egui::TextStyle::Monospace, size - 1.),
        (egui::TextStyle::Heading, size + 7.),
    ] {
        let family = if text_style == egui::TextStyle::Monospace {
            egui::FontFamily::Monospace
        } else {
            egui::FontFamily::Proportional
        };
        style
            .text_styles
            .insert(text_style, egui::FontId::new(points, family));
    }
    style.spacing.interact_size.y = size + 10.;
    style.spacing.item_spacing.y = 8.;
    ctx.set_style(style);
}
pub fn value(n: Option<f64>, suffix: &str) -> String {
    n.map(|v| wot_insights::tr_format!("{v:.1}{suffix}"))
        .unwrap_or_else(|| "–".into())
}
fn integer(n: Option<i64>) -> String {
    n.map(|v| money(Some(v as f64)))
        .unwrap_or_else(|| "–".into())
}
pub(super) fn tank_name(tank: &str) -> String {
    let tank = tank.split_once('-').map(|(_, t)| t).unwrap_or(tank);
    let tank = tank
        .split_once('_')
        .filter(|(code, _)| code.chars().any(|c| c.is_ascii_digit()))
        .map(|(_, name)| name)
        .unwrap_or(tank);
    tank.replace('_', " ")
}
fn wr_color(wr: Option<f64>) -> Color32 {
    match wr {
        Some(v) if v >= 50. => GREEN,
        Some(_) => RED,
        None => MUTED,
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Sort {
    Name,
    Date,
    Battles,
    Winrate,
    Damage,
    Assist,
    Kills,
    Xp,
    Credits,
    Net,
    CleanNet,
    UnboostedNet,
    TotalNet,
    Expenses,
    Shots,
    Hits,
    Penetrations,
    HitRate,
    PenetrationRate,
    PenetrationPerShot,
}
#[derive(Clone, Copy)]
struct Sorting {
    column: Sort,
    descending: bool,
}
impl Sorting {
    fn heading(&mut self, ui: &mut egui::Ui, label: &str, column: Sort) {
        let label = crate::i18n::text(label);
        let suffix = if self.column == column {
            if self.descending { " ↓" } else { " ↑" }
        } else {
            ""
        };
        if ui
            .selectable_label(
                self.column == column,
                crate::i18n::widget(wot_insights::tr_format!("{label}{suffix}")),
            )
            .clicked()
        {
            if self.column == column {
                self.descending = !self.descending;
            } else {
                self.column = column;
                self.descending = column != Sort::Name;
            }
        }
    }
}
use wot_insights::scan::Loaded;
type CleanupResult = Result<(usize, u64, Vec<String>)>;

pub struct App {
    preferences: Preferences,
    settings_path: PathBuf,
    database: PathBuf,
    battles: Vec<Battle>,
    worker: std::sync::Arc<std::sync::Mutex<wot_insights::scan::Worker>>,
    receiver: Option<mpsc::Receiver<Result<Loaded>>>,
    last_scan: Instant,
    last_success: Option<String>,
    message: String,
    import_failed: bool,
    errors: Vec<String>,
    tank_sort: Sorting,
    battle_sort: Sorting,
    finance_sort: Sorting,
    page: usize,
    selected: Option<String>,
    details: crate::details_view::State,
    cleanup_candidates: Vec<wot_insights::archive::Candidate>,
    cleanup_folder: String,
    cleanup_confirmed: bool,
    cleanup_message: String,
    cleanup_receiver: Option<mpsc::Receiver<CleanupResult>>,
    sql: crate::sql_view::State,
    damage_view: crate::interaction_view::State,
    player_view: crate::interaction_view::State,
}
impl App {
    pub fn new(game: PathBuf, replays: PathBuf, database: PathBuf) -> Self {
        let settings_path = database.with_file_name("settings.json");
        let settings = if settings_path.exists() {
            Preferences::load(&settings_path)
        } else {
            Ok(Preferences::default())
        };
        let settings_error = settings
            .as_ref()
            .err()
            .map(|e| wot_insights::tr_format!("Einstellungen konnten nicht geladen werden: {e:#}"));
        let mut preferences = settings.unwrap_or_default();
        if preferences.game_folder.is_empty() || std::env::var_os("WOT_GAME_DIR").is_some() {
            preferences.game_folder = game.to_string_lossy().into_owned();
        }
        if preferences.replay_folder.is_empty() || std::env::var_os("WOT_REPLAY_DIR").is_some() {
            preferences.replay_folder = replays.to_string_lossy().into_owned();
        }

        let mut app = Self {
            preferences,
            settings_path,
            database,
            battles: Vec::new(),
            worker: Default::default(),
            receiver: None,
            last_scan: Instant::now(),
            last_success: None,
            message: String::new(),
            import_failed: false,
            errors: settings_error.into_iter().collect(),
            tank_sort: Sorting {
                column: Sort::Battles,
                descending: true,
            },
            battle_sort: Sorting {
                column: Sort::Date,
                descending: true,
            },
            finance_sort: Sorting {
                column: Sort::CleanNet,
                descending: true,
            },
            page: 0,
            selected: None,
            details: Default::default(),
            cleanup_candidates: Vec::new(),
            cleanup_folder: String::new(),
            cleanup_confirmed: false,
            cleanup_message: String::new(),
            cleanup_receiver: None,
            sql: Default::default(),
            damage_view: Default::default(),
            player_view: Default::default(),
        };
        if app.preferences.filter.player.is_empty() {
            app.preferences.filter.player = app
                .battles
                .first()
                .map(|b| b.player.clone())
                .unwrap_or_default();
        }
        // Loading a saved database works even when automatic import is paused.
        app.scan_with_import(app.preferences.automatic);
        app
    }
    fn render(&mut self, ctx: &egui::Context) {
        crate::i18n::set_language(&self.preferences.language);
        if ctx.style().text_styles[&egui::TextStyle::Body].size != self.preferences.font_size {
            typography(ctx, self.preferences.font_size);
        }
        let previous = self.preferences.clone();
        egui::TopBottomPanel::top("navigation").show(ctx, |ui| {
            ui.add_space(5.);
            ui.horizontal(|ui| {
                ui.label(crate::i18n::widget(
                    RichText::new(wot_insights::brand::NAME).size(23.).strong(),
                ));
                ui.label(crate::i18n::widget(
                    RichText::new(wot_insights::tr_format!(
                        "{}  /  LOKALE STATISTIK",
                        env!("CARGO_PKG_VERSION")
                    ))
                    .small()
                    .color(MUTED),
                ));
            });
            ui.horizontal_wrapped(|ui| {
                for i in [0, 1, 2, 9, 10, 6, 3, 4, 7, 8, 5, 11] {
                    if i == 8 && !self.preferences.sql_enabled {
                        continue;
                    }
                    ui.selectable_value(
                        &mut self.preferences.view,
                        i,
                        crate::i18n::widget(VIEWS[i]),
                    );
                }
            });
            ui.add_space(3.);
        });
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                let (label, color) = if self.import_failed {
                    ("● Importfehler", RED)
                } else if self.receiver.is_some() {
                    ("● Lese ein …", ACCENT)
                } else if self.preferences.automatic {
                    ("● Import aktiv", GREEN)
                } else {
                    ("● Pausiert", MUTED)
                };
                ui.colored_label(
                    color,
                    egui::RichText::from(crate::i18n::rich(label)).strong(),
                );
                ui.separator();
                ui.label(crate::i18n::widget(&self.message));
                if let Some(time) = &self.last_success {
                    ui.weak(wot_insights::tr_format!("Stand {time}"));
                }
                if ui
                    .add_enabled(
                        self.receiver.is_none(),
                        egui::Button::new(crate::i18n::widget("Jetzt einlesen")),
                    )
                    .clicked()
                {
                    self.scan();
                }
            });
        });
        if !matches!(self.preferences.view, 5 | 7 | 8 | 9 | 10 | 11) {
            egui::TopBottomPanel::top("filters")
                .show(ctx, |ui| filters(ui, &self.battles, &mut self.preferences));
        }
        egui::CentralPanel::default().show(ctx,|ui|{
            if self.preferences.view==11 {self.info(ui); return;}
            if self.preferences.view==5 {self.settings(ui); return;}
            if self.preferences.view==7 {self.replay_archive(ui); return;}
            if self.preferences.view==8 {self.sql.show(ui, &self.database); return;}
            if self.preferences.view==9 {self.damage_view.show(ui, &self.database, true, &mut self.selected); return;}
            if self.preferences.view==10 {self.player_view.show(ui, &self.database, false, &mut self.selected); return;}
            let filtered=analysis::filter(&self.battles,&self.preferences.filter,&self.preferences.favorites,Local::now().date_naive());
            let total=stats::summarize(filtered.iter().copied());
            ui.horizontal(|ui|{
                ui.heading(crate::i18n::rich(VIEWS[self.preferences.view]));
                ui.label(crate::i18n::widget(RichText::new(wot_insights::tr_format!("{} Gefechte · {} Endergebnisse · {} offen",total.count,total.known_results,total.count-total.known_results)).color(MUTED)));
            });
            ui.weak(crate::i18n::rich("Zufallsgefechte aus deinen gespeicherten Replays – keine vollständige Kontostatistik."));
            ui.add_space(8.);
            if filtered.is_empty(){ui.label(crate::i18n::widget("Keine Gefechte für diese Filter. Zeitraum, Favoriten und Datenquelle prüfen."));return;}
            match self.preferences.view {
                0=>overview(ui,&filtered,&total,&mut self.selected),
                1=>tank_table(ui,&filtered,&mut self.preferences,&mut self.tank_sort),
                2=>battle_table(ui,&filtered,&mut self.preferences.columns,&mut self.battle_sort,&mut self.page,&mut self.selected),
                3=>{
                    ui.horizontal_wrapped(|ui|{
                        ui.selectable_value(&mut self.preferences.map_mode,0,crate::i18n::widget("Karte × Klasse"));
                        ui.selectable_value(&mut self.preferences.map_mode,1,crate::i18n::widget("Karten-Rangliste"));
                        ui.separator();ui.label(crate::i18n::widget("Mind. Endergebnisse"));ui.add(egui::DragValue::new(&mut self.preferences.minimum).range(1..=500));
                    });
                    if self.preferences.map_mode==0{crate::maps_view::heatmap(ui,&filtered,self.preferences.minimum,&mut self.preferences.map_metric);}
                    else{crate::maps_view::ranking(ui,&filtered,self.preferences.minimum);}
                }
                6=>finance_view(ui,&filtered,&total,&mut self.preferences,&mut self.finance_sort),
                _=>trend_view(ui,&filtered,&mut self.preferences.trend_metric),
            }
        });
        if let Some(key) = &self.selected {
            if let Some(battle) = self.battles.iter().find(|b| b.key == *key) {
                self.details.select(&self.database, battle);
                let mut open = true;
                egui::Window::new(crate::i18n::widget("Gefechtsdetails"))
                    .open(&mut open)
                    .resizable(true)
                    .default_width(1050.)
                    .default_height(650.)
                    .vscroll(true)
                    .show(ctx, |ui| {
                        self.details.show(ui, battle);
                        ui.add_space(12.);
                        egui::CollapsingHeader::new(crate::i18n::widget(
                            "Zusammenfassung & Credits-Abrechnung",
                        ))
                        .default_open(false)
                        .show(ui, |ui| battle_details(ui, battle));
                    });
                if !open {
                    self.selected = None;
                }
            } else {
                self.selected = None;
            }
        }
        if self.preferences.filter != previous.filter {
            self.page = 0;
        }
        if self.preferences != previous
            && let Err(e) = self.preferences.save(&self.settings_path)
        {
            self.message = wot_insights::tr_format!("Speichern fehlgeschlagen: {e:#}");
        }
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.poll();
        ctx.request_repaint_after(Duration::from_millis(500));
        self.render(ctx);
    }
}

mod preferences;
use preferences::{Columns, Preferences};
mod background;
mod settings;
mod trends;
use trends::*;
mod finance;
use finance::*;
mod battles;
use battles::*;
mod tanks;
use tanks::*;
mod overview;
use overview::*;
mod filters;
use filters::*;
#[cfg(test)]
mod tests;

mod info;
