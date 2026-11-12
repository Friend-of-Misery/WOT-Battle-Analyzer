use crate::ui::value;
use eframe::egui;
use std::collections::{BTreeMap, BTreeSet};
use wot_insights::{
    replay::Battle,
    stats::{self, Summary},
    vehicles,
};

pub fn heatmap(ui: &mut egui::Ui, battles: &[&Battle], minimum: usize, metric: &mut usize) {
    ui.horizontal(|ui| {
        ui.label(crate::i18n::widget("Vergleichswert"));
        for (i, label) in ["Winrate", "Ø Schaden", "Ø Assist"].iter().enumerate() {
            ui.selectable_value(metric, i, crate::i18n::widget(*label));
        }
    });
    ui.weak(crate::i18n::rich(
        "Zellen zeigen Wert und n = bekannte Endergebnisse. Weniger Daten als das Minimum: grau.",
    ));
    let groups = stats::maps_by_class(battles.iter().copied());
    let maps: BTreeSet<_> = groups.keys().map(|(m, _)| m.clone()).collect();
    let classes = ["lightTank", "mediumTank", "heavyTank", "AT-SPG", "SPG"];
    let class_baselines: BTreeMap<_, _> = classes
        .iter()
        .map(|c| {
            (
                *c,
                stats::summarize(battles.iter().copied().filter(|b| b.class == *c)),
            )
        })
        .collect();
    egui::ScrollArea::both().show(ui, |ui| {
        egui::Grid::new("heatmap")
            .min_col_width(145.)
            .spacing([8., 6.])
            .show(ui, |ui| {
                ui.strong(crate::i18n::rich("Karte"));
                for c in classes {
                    ui.strong(crate::i18n::rich(vehicles::class_label(c)));
                }
                ui.end_row();
                for map in maps {
                    ui.label(crate::i18n::widget(&map));
                    for class in classes {
                        let s = groups.get(&(map.clone(), class.to_owned()));
                        if let Some(s) = s {
                            let v = metric_value(s, *metric);
                            let base = metric_value(&class_baselines[class], *metric);
                            let color = if s.known_results < minimum || v.is_none() {
                                egui::Color32::from_gray(42)
                            } else if v.zip(base).is_some_and(|(a, b)| a >= b) {
                                egui::Color32::from_rgb(26, 83, 69)
                            } else {
                                egui::Color32::from_rgb(103, 53, 42)
                            };
                            egui::Frame::new()
                                .fill(color)
                                .inner_margin(8.)
                                .corner_radius(5.)
                                .show(ui, |ui| {
                                    ui.set_min_width(125.);
                                    ui.label(crate::i18n::widget(
                                        egui::RichText::new(wot_insights::tr_format!(
                                            "{}  ·  n={}",
                                            value(v, if *metric == 0 { "%" } else { "" }),
                                            s.known_results
                                        ))
                                        .color(egui::Color32::WHITE)
                                        .strong(),
                                    ));
                                })
                                .response
                                .on_hover_text(wot_insights::tr_format!(
                                    "{} Replays, {} ohne Endergebnis. Klassenmittel im Filter: {}",
                                    s.count,
                                    s.count - s.known_results,
                                    value(base, if *metric == 0 { "%" } else { "" })
                                ));
                        } else {
                            ui.weak(crate::i18n::rich("–"));
                        }
                    }
                    ui.end_row();
                }
            });
    });
    ui.weak(crate::i18n::rich("Grün: mindestens dein Klassenmittel im Filter. Orange: darunter. Keine Bewertung der statistischen Signifikanz."));
}
fn metric_value(s: &Summary, metric: usize) -> Option<f64> {
    match metric {
        0 => s.winrate(),
        1 => s.avg_damage(),
        _ => s.avg_assist(),
    }
}
pub fn ranking(ui: &mut egui::Ui, battles: &[&Battle], minimum: usize) {
    let mut groups = BTreeMap::<String, Summary>::new();
    for b in battles {
        groups.entry(b.map.clone()).or_default().add(b);
    }
    let mut rows: Vec<_> = groups
        .into_iter()
        .filter(|(_, s)| s.known_results >= minimum)
        .collect();
    rows.sort_by(|a, b| {
        b.1.winrate()
            .partial_cmp(&a.1.winrate())
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.1.known_results.cmp(&a.1.known_results))
    });
    ui.weak(crate::i18n::rich("Nach Winrate sortiert. Wähle eine Klasse oder einen Panzer, um deine passenden Karten zu finden."));
    if rows.is_empty() {
        ui.label(crate::i18n::widget(
            "Noch keine Karte erreicht die Mindestzahl. Minimum senken oder Filter erweitern.",
        ));
    }
    egui::ScrollArea::both().show(ui, |ui| {
        egui::Grid::new("ranking")
            .striped(true)
            .min_col_width(130.)
            .show(ui, |ui| {
                for h in [
                    "Karte",
                    "Endergebnisse",
                    "Winrate",
                    "Ø Schaden",
                    "Ø Assist",
                    "Ø Basis-XP",
                ] {
                    ui.strong(crate::i18n::rich(h));
                }
                ui.end_row();
                for (map, s) in rows {
                    ui.label(crate::i18n::widget(map));
                    ui.strong(crate::i18n::rich(s.known_results.to_string()));
                    ui.strong(crate::i18n::rich(value(s.winrate(), "%")));
                    ui.strong(crate::i18n::rich(value(s.avg_damage(), "")));
                    ui.strong(crate::i18n::rich(value(s.avg_assist(), "")));
                    ui.strong(crate::i18n::rich(value(s.avg_xp(), "")));
                    ui.end_row();
                }
            });
    });
}
