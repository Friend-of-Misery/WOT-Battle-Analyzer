use anyhow::Result;
use eframe::egui::{self, RichText};
use serde_json::Value;
use std::{path::Path, sync::mpsc};
use wot_insights::{
    replay::Battle,
    replay_data::{self, Document},
    store::Store,
};

#[derive(Default)]
pub struct State {
    key: String,
    receiver: Option<mpsc::Receiver<Result<Document>>>,
    document: Option<Document>,
    error: Option<String>,
    search: String,
    binary_page: usize,
    tab: usize,
    hide_explanations: bool,
    technical_names: bool,
}

impl State {
    pub fn select(&mut self, database: &Path, battle: &Battle) {
        if self.key != battle.key {
            *self = Self {
                key: battle.key.clone(),
                ..Default::default()
            };
            let (sender, receiver) = mpsc::channel();
            self.receiver = Some(receiver);
            let database = database.to_owned();
            let battle = battle.clone();
            std::thread::spawn(move || {
                let _ =
                    sender.send(Store::open(&database).and_then(|s| s.battle_document(&battle)));
            });
        }
        match self.receiver.as_ref().map(|r| r.try_recv()) {
            Some(Ok(result)) => {
                self.receiver = None;
                match result {
                    Ok(document) => self.document = Some(document),
                    Err(error) => self.error = Some(wot_insights::tr_format!("{error:#}")),
                }
            }
            Some(Err(mpsc::TryRecvError::Disconnected)) => {
                self.receiver = None;
                self.error =
                    Some("Laden unterbrochen. Details schließen und erneut öffnen.".into());
            }
            _ => {}
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, battle: &Battle) {
        ui.heading(crate::i18n::rich(crate::ui::tank_name(&battle.tank)));
        ui.horizontal_wrapped(|ui| {
            ui.label(crate::i18n::widget(&battle.map));
            ui.separator();
            ui.label(crate::i18n::widget(&battle.date));
            ui.separator();
            ui.label(crate::i18n::widget(&battle.player));
            ui.separator();
            let (result, color) = match battle.result.as_deref() {
                Some("win") => ("Sieg", crate::ui::GREEN),
                Some("loss") => ("Niederlage", crate::ui::RED),
                Some("draw") => ("Unentschieden", ui.visuals().text_color()),
                _ => ("Endergebnis offen", ui.visuals().weak_text_color()),
            };
            ui.colored_label(
                color,
                egui::RichText::from(crate::i18n::rich(
                    RichText::new(crate::i18n::text(result)).strong(),
                ))
                .strong(),
            );
        });
        ui.add_space(12.);
        ui.horizontal_wrapped(|ui| {
            for (index, label) in [
                "Übersicht",
                "Eigene Werte",
                "Gefecht & Teilnehmer",
                "Originaldaten",
            ]
            .iter()
            .enumerate()
            {
                ui.selectable_value(&mut self.tab, index, crate::i18n::widget(*label));
            }
        });
        ui.separator();
        if self.tab == 0 {
            overview(ui, battle);
            return;
        }
        if self.receiver.is_some() {
            ui.spinner();
            ui.label(crate::i18n::widget(
                "Vollständige Daten werden aus Replay oder Archiv geladen …",
            ));
            return;
        }
        if let Some(error) = &self.error {
            ui.label(error);
            if ui.button(crate::i18n::widget("Erneut laden")).clicked() {
                self.key.clear();
                self.error = None;
            }
            return;
        }
        let Some(document) = &self.document else {
            return;
        };
        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.search)
                    .hint_text(crate::i18n::widget("Feldname, Wert oder Bedeutung …"))
                    .desired_width(330.),
            );
            if ui.button(crate::i18n::widget("Suche leeren")).clicked() {
                self.search.clear();
            }
            if self.tab == 3 && ui.button(crate::i18n::widget("JSON kopieren")).clicked() {
                ui.ctx()
                    .copy_text(serde_json::to_string_pretty(&document.blocks).unwrap_or_default());
            }
        });
        ui.horizontal_wrapped(|ui| {
            let mut explanations = !self.hide_explanations;
            ui.checkbox(
                &mut explanations,
                crate::i18n::widget("Erklärungen anzeigen"),
            );
            self.hide_explanations = !explanations;
            ui.checkbox(
                &mut self.technical_names,
                crate::i18n::widget("Technische Feldnamen"),
            );
        });
        let search = self.search.trim().to_lowercase();
        let options = Options {
            explanations: !self.hide_explanations,
            technical: self.technical_names || self.tab == 3,
        };
        ui.add_space(8.);
        if self.tab == 1 && search.is_empty() {
            own_groups(ui, document, options);
            return;
        }
        if self.tab == 1
            && let Some(result) = document.blocks.get(1)
        {
            let result = if result.is_array() {
                &result[0]
            } else {
                result
            };
            if let Some(personal) = result.get("personal") {
                egui::CollapsingHeader::new(crate::i18n::widget(
                    "Eigene Ergebnisdaten – alle Werte erklärt",
                ))
                .id_salt("own_explained_results")
                .default_open(true)
                .show(ui, |ui| {
                    tree(ui, "personal", personal, "own", &search, true, options)
                });
            }
        }
        for (index, block) in document.blocks.iter().enumerate() {
            if self.tab == 1 {
                continue;
            }
            let name = match index {
                0 => "Gefechtsbeginn, Karte, Server und Teilnehmer".to_owned(),
                1 => "Vollständige Endergebnisse – eigene Daten, Teams und alle Teilnehmer"
                    .to_owned(),
                _ => wot_insights::tr_format!("Zusätzlicher JSON-Block {index}"),
            };
            let header = egui::CollapsingHeader::new(name).id_salt(("json_block", index));
            let header = if search.is_empty() {
                header
            } else {
                header.open(Some(true))
            };
            header.show(ui, |ui| {
                tree(
                    ui,
                    &wot_insights::tr_format!("JSON-Block {index}"),
                    block,
                    &wot_insights::tr_format!("block/{index}"),
                    &search,
                    true,
                    options,
                )
            });
        }
        if self.tab != 3 {
            return;
        }
        egui::CollapsingHeader::new(crate::i18n::widget("Dateiinformationen")).show(ui, |ui| {
            ui.label(wot_insights::tr_format!("Quelle: {}", document.source));
            ui.label(wot_insights::tr_format!("{} JSON-Blöcke · {} Bytes Originaldatei · {} Bytes binärer Rest", document.blocks.len(), grouped(&document.original_size.to_string()), grouped(&document.stream.len().to_string())));
            ui.weak(crate::i18n::rich("Unbekannte Felder, leere Listen und Nullwerte bleiben vollständig erhalten. Noch nicht geprüfte Bedeutungen sind gekennzeichnet."));
        });
        egui::CollapsingHeader::new(crate::i18n::widget("Binärer Gefechtsverlauf – Rohdaten, noch nicht vollständig entschlüsselt"))
            .id_salt("binary_stream").show(ui, |ui| {
                ui.label(crate::i18n::widget("Dieser Bereich enthält den unveränderten binären Rest der Replaydatei, einschließlich seiner Format-/Kompressionsdaten. Positionen, Kamera, Chat und Ereignisse werden daraus bisher nicht vollständig dekodiert. Die Bytes sind vollständig erhalten; eine inhaltliche Erklärung jedes Pakets ist derzeit noch nicht möglich."));
                let pages = document.stream.len().div_ceil(256).max(1);
                ui.horizontal(|ui| {
                    if ui.button(crate::i18n::widget("Zurück")).clicked() { self.binary_page = self.binary_page.saturating_sub(1); }
                    ui.label(crate::i18n::widget("Seite (ab 0)"));
                    ui.add(egui::DragValue::new(&mut self.binary_page).range(0..=pages-1));
                    if ui.button(crate::i18n::widget("Weiter")).clicked() { self.binary_page = (self.binary_page + 1).min(pages-1); }
                    ui.label(wot_insights::tr_format!("von {} Seiten", grouped(&pages.to_string())));
                });
                self.binary_page = self.binary_page.min(pages-1);
                let start = (self.binary_page * 256).min(document.stream.len());
                let end = (start + 256).min(document.stream.len());
                for (index, chunk) in document.stream[start..end].chunks(16).enumerate() {
                    ui.monospace(format!("{:08X}  {}", start + index * 16, chunk.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" ")));
                }
            });
    }
}

#[derive(Clone, Copy)]
struct Options {
    explanations: bool,
    technical: bool,
}

fn overview(ui: &mut egui::Ui, battle: &Battle) {
    let metrics = [
        ("Schaden", battle.damage),
        ("Assist", battle.assist),
        ("Abschüsse", battle.kills),
        ("Basis-XP", battle.base_xp),
    ];
    ui.columns(4, |columns| {
        for (column, (label, value)) in columns.iter_mut().zip(metrics) {
            egui::Frame::group(column.style())
                .inner_margin(12.)
                .show(column, |ui| {
                    ui.set_min_width((ui.available_width() - 8.).max(60.));
                    ui.weak(crate::i18n::rich(label));
                    ui.label(crate::i18n::widget(
                        RichText::new(crate::i18n::text(
                            value
                                .map(|n| grouped(&n.to_string()))
                                .unwrap_or_else(|| "–".into()),
                        ))
                        .size(26.)
                        .strong(),
                    ));
                });
        }
    });
    ui.add_space(14.);
    ui.columns(2, |columns| {
        egui::Frame::group(columns[0].style())
            .inner_margin(12.)
            .show(&mut columns[0], |ui| {
                ui.strong(crate::i18n::rich("Treffer & Überleben"));
                ui.add_space(8.);
                for (label, value) in [
                    ("Schüsse", battle.shots),
                    ("Direkte Treffer", battle.hits),
                    ("Durchschläge", battle.penetrations),
                    ("Erhaltener Schaden", battle.received),
                ] {
                    ui.horizontal(|ui| {
                        ui.label(crate::i18n::widget(label));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.strong(crate::i18n::rich(
                                value
                                    .map(|n| grouped(&n.to_string()))
                                    .unwrap_or_else(|| "–".into()),
                            ));
                        });
                    });
                }
                if let Some(percentage) =
                    wot_insights::accuracy::percentage(battle.hits, battle.shots)
                {
                    ui.add_space(8.);
                    ui.add(
                        egui::ProgressBar::new((percentage / 100.).clamp(0., 1.) as f32)
                            .text(wot_insights::tr_format!("Trefferquote {percentage:.1} %")),
                    );
                }
            });
        egui::Frame::group(columns[1].style())
            .inner_margin(12.)
            .show(&mut columns[1], |ui| {
                ui.strong(crate::i18n::rich("Credits-Bilanz"));
                ui.add_space(8.);
                if let Some(economy) = &battle.economy {
                    for (label, value) in [
                        ("Einnahmen", economy.gross),
                        ("Ausgaben", economy.expenses().map(|n| -n)),
                        ("Bilanz", economy.net()),
                    ] {
                        ui.horizontal(|ui| {
                            ui.label(crate::i18n::widget(label));
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let color = if value.is_some_and(|n| n < 0) {
                                        crate::ui::RED
                                    } else {
                                        ui.visuals().text_color()
                                    };
                                    ui.label(crate::i18n::widget(
                                        RichText::new(crate::i18n::text(
                                            value
                                                .map(|n| grouped(&n.to_string()))
                                                .unwrap_or_else(|| "–".into()),
                                        ))
                                        .strong()
                                        .color(color)
                                        .size(if label == "Bilanz" { 22. } else { 16. }),
                                    ));
                                },
                            );
                        });
                    }
                    ui.add_space(8.);
                    ui.weak(crate::i18n::rich(
                        "Boni sind in den Einnahmen bereits enthalten.",
                    ));
                } else {
                    ui.weak(crate::i18n::rich(
                        "Für dieses Gefecht liegt noch keine Credits-Abrechnung vor.",
                    ));
                }
            });
    });
    ui.add_space(14.);
    ui.strong(crate::i18n::rich(
        "Alle weiteren Werte findest du unter „Eigene Werte“.",
    ));
    ui.weak(crate::i18n::rich("Dort sind Schaden, Verdienst, Erfahrung und Fortschritt thematisch gruppiert. Teilnehmerdaten und vollständige Originaldaten haben eigene Bereiche."));
}

fn category(key: &str) -> usize {
    let key = key.to_lowercase();
    if key.contains("credit")
        || key.contains("cost")
        || key.contains("gold")
        || key.contains("crystal")
        || key.contains("coin")
        || key == "repair"
        || key == "currencies"
    {
        0
    } else if key.contains("damage")
        || key.contains("hit")
        || key.contains("shot")
        || key.contains("pierc")
        || key.contains("kill")
        || key.contains("stun")
        || key.contains("spotted")
    {
        1
    } else if key.contains("health")
        || key.contains("death")
        || key.contains("lifetime")
        || key.contains("mileage")
        || key.contains("suicide")
        || key.contains("leave")
        || key.contains("watched")
        || key.contains("respawn")
        || key.contains("rollout")
    {
        2
    } else if key.contains("xp") || key.contains("premium") || key.contains("wotplus") {
        3
    } else if key.contains("quest")
        || key.contains("battlepass")
        || key.contains("achievement")
        || key.contains("mastery")
        || key.contains("marks")
        || key.contains("rating")
        || key.contains("progress")
        || key.starts_with("bp")
        || key.contains("dossier")
        || key.contains("reward")
        || key.contains("rank")
    {
        4
    } else if key.contains("capture")
        || key.contains("flag")
        || key.contains("resource")
        || key.contains("defend")
        || key.contains("winpoint")
        || key.contains("destructible")
    {
        5
    } else {
        6
    }
}

fn own_groups(ui: &mut egui::Ui, document: &Document, options: Options) {
    let Some(result) = document.blocks.get(1) else {
        ui.weak(crate::i18n::rich("Dieses Replay enthält noch kein Endergebnis. Informationen zum Gefechtsbeginn findest du unter „Gefecht & Teilnehmer“."));
        return;
    };
    let result = if result.is_array() {
        &result[0]
    } else {
        result
    };
    let Some(personal) = result.get("personal").and_then(Value::as_object) else {
        ui.weak(crate::i18n::rich(
            "Keine persönlichen Ergebnisdaten in diesem Replay vorhanden.",
        ));
        return;
    };
    for (dataset, values) in personal {
        let Some(fields) = values.as_object() else {
            tree(
                ui,
                dataset,
                values,
                &wot_insights::tr_format!("own/{dataset}"),
                "",
                false,
                options,
            );
            continue;
        };
        if personal.len() > 1 {
            ui.add_space(8.);
            ui.strong(crate::i18n::rich(if dataset == "avatar" {
                "Spielerbezogene Werte".into()
            } else {
                wot_insights::tr_format!("Fahrzeugwerte · Datensatz {dataset}")
            }));
        }
        for (group, title) in [
            "Einnahmen, Boni & Kosten",
            "Schaden, Treffer & Aufklärung",
            "Überleben & Bewegung",
            "Erfahrung & Kontoboni",
            "Auszeichnungen, Aufgaben & Fortschritt",
            "Basen & besondere Ziele",
            "Weitere Angaben",
        ]
        .iter()
        .enumerate()
        {
            let entries: Vec<_> = fields
                .iter()
                .filter(|(key, _)| category(key) == group)
                .collect();
            if entries.is_empty() {
                continue;
            }
            ui.add_space(6.);
            egui::Frame::group(ui.style())
                .inner_margin(10.)
                .show(ui, |ui| {
                    egui::CollapsingHeader::new(crate::i18n::widget(
                        RichText::new(wot_insights::tr_format!(
                            "{title}   ·   {} Werte",
                            entries.len()
                        ))
                        .strong(),
                    ))
                    .id_salt(("group", dataset, group))
                    .default_open(group < 2 && dataset != "avatar")
                    .show(ui, |ui| {
                        for (key, value) in entries {
                            tree(
                                ui,
                                key,
                                value,
                                &wot_insights::tr_format!("own/{dataset}/{key}"),
                                "",
                                false,
                                options,
                            );
                        }
                    });
                });
        }
    }
}

fn label(key: &str) -> &str {
    match key {
        "damageDealt" => "Verursachter Schaden",
        "damageReceived" => "Erhaltener Schaden",
        "damageBlockedByArmor" => "Durch Panzerung geblockt",
        "damageAssistedRadio" => "Assist durch Aufklärung",
        "damageAssistedTrack" => "Assist durch Ketten",
        "damageAssistedStun" => "Assist durch Betäubung",
        "shots" => "Abgegebene Schüsse",
        "directEnemyHits" => "Direkte Gegnertreffer",
        "piercingEnemyHits" => "Durchschläge bei Gegnern",
        "explosionHits" => "Sprengwirkungstreffer",
        "kills" => "Abschüsse",
        "spotted" => "Aufgeklärte Gegner",
        "health" => "Verbleibende Lebenspunkte",
        "maxHealth" => "Maximale Lebenspunkte",
        "lifeTime" => "Überlebenszeit",
        "mileage" => "Fahrstrecke (Originaleinheit)",
        "credits" => "Einnahmen brutto",
        "originalCredits" => "Basisverdienst",
        "subtotalCredits" => "Credits-Zwischensumme",
        "factualCredits" => "Credits vor Aufgabenbonus",
        "boosterCredits" => "Persönliche Booster",
        "orderCredits" => "Clan-/Kampfreserven",
        "eventCredits" => "Aufgaben & Ereignisse",
        "autoRepairCost" => "Fahrzeugreparatur",
        "autoLoadCost" => "Munitionsauffüllung",
        "autoEquipCost" => "Verbrauchsmaterial auffüllen",
        "repair" => "Interner Reparaturwert",
        "originalXP" => "Basis-XP",
        "xp" => "Erfahrung gesamt",
        "freeXP" => "Freie Erfahrung",
        "tmenXP" => "Besatzungserfahrung",
        "dailyXPFactor10" => "Tages-/Erstsiegfaktor",
        "capturePoints" => "Eroberungspunkte",
        "droppedCapturePoints" => "Zurückgesetzte Eroberungspunkte",
        "deathReason" => "Todesursache (Code)",
        "killerID" => "Zerstört durch (Kennung)",
        "isPrematureLeave" => "Vorzeitig verlassen",
        "watchedBattleToTheEnd" => "Bis zum Ende verfolgt",
        "achievements" => "Auszeichnungen",
        "inBattleAchievements" => "Auszeichnungen im Gefecht",
        "markOfMastery" => "Meisterschaftsabzeichen",
        "marksOnGun" => "Geschützmarkierungen",
        "movingAvgDamage" => "Gleitender Schadenswert",
        "battlePassPoints" => "Battle-Pass-Punkte",
        "questsProgress" => "Aufgabenfortschritt",
        "PM2Progress" => "Kampagnenfortschritt",
        "details" => "Begegnungen mit einzelnen Gegnern",
        "personal" => "Eigene Ergebnisdaten",
        "common" => "Allgemeines Gefechtsergebnis",
        "players" => "Spieler",
        "vehicles" => "Fahrzeuge",
        "avatar" => "Spielerbezogene Werte",
        "dateTime" => "Gefechtszeit",
        "mapDisplayName" => "Karte",
        "mapName" => "Kartenkennung",
        "serverName" => "Server",
        "playerName" => "Spielername",
        "playerVehicle" => "Fahrzeugkennung",
        "winnerTeam" => "Siegerteam",
        "duration" => "Gefechtsdauer",
        "team" => "Team",
        "gold" => "Gold",
        "crystal" => "Anleihen",
        "isPremium" => "Premiumstatus",
        _ => key,
    }
}

fn grouped(raw: &str) -> String {
    let (sign, digits) = raw.strip_prefix('-').map_or(("", raw), |s| ("−", s));
    if !digits.bytes().all(|b| b.is_ascii_digit()) {
        return raw.into();
    }
    let mut output = String::from(sign);
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            output.push('.');
        }
        output.push(ch);
    }
    output
}

fn display(value: &Value) -> String {
    match value {
        Value::Null => "Nicht übermittelt (null)".into(),
        Value::Bool(true) => "Ja (true)".into(),
        Value::Bool(false) => "Nein (false)".into(),
        Value::Number(number) => grouped(&number.to_string()),
        Value::String(text) if text.is_empty() => "Leere Zeichenfolge (\"\")".into(),
        Value::String(text) => text.clone(),
        _ => value.to_string(),
    }
}

fn matches(key: &str, value: &Value, search: &str) -> bool {
    if search.is_empty()
        || key.to_lowercase().contains(search)
        || label(key).to_lowercase().contains(search)
        || replay_data::explanation(key)
            .to_lowercase()
            .contains(search)
    {
        return true;
    }
    match value {
        Value::Object(fields) => fields
            .iter()
            .any(|(key, value)| matches(key, value, search)),
        Value::Array(values) => values.iter().any(|value| matches("", value, search)),
        _ => {
            display(value).to_lowercase().contains(search)
                || value.to_string().to_lowercase().contains(search)
        }
    }
}

fn tree(
    ui: &mut egui::Ui,
    key: &str,
    value: &Value,
    path: &str,
    search: &str,
    open: bool,
    options: Options,
) {
    if !matches(key, value, search) {
        return;
    }
    let children = match value {
        Value::Object(fields) => fields.len(),
        Value::Array(values) => values.len(),
        _ => 0,
    };
    if children > 0 {
        let header = egui::CollapsingHeader::new(wot_insights::tr_format!(
            "{} · {children} Einträge",
            label(key)
        ))
        .id_salt(path)
        .default_open(open);
        let header = if search.is_empty() {
            header
        } else {
            header.open(Some(true))
        };
        header.show(ui, |ui| {
            if options.explanations {
                ui.weak(crate::i18n::rich(replay_data::explanation(key)));
            }
            if options.technical && label(key) != key {
                ui.monospace(key);
            }
            let child_search = if !search.is_empty()
                && (key.to_lowercase().contains(search)
                    || replay_data::explanation(key)
                        .to_lowercase()
                        .contains(search))
            {
                ""
            } else {
                search
            };
            match value {
                Value::Object(fields) => {
                    for (child, value) in fields {
                        tree(
                            ui,
                            child,
                            value,
                            &wot_insights::tr_format!("{path}/{child}"),
                            child_search,
                            key == "personal",
                            options,
                        );
                    }
                }
                Value::Array(values) => {
                    for (index, value) in values.iter().enumerate() {
                        tree(
                            ui,
                            &wot_insights::tr_format!("[{index}]"),
                            value,
                            &wot_insights::tr_format!("{path}/{index}"),
                            child_search,
                            false,
                            options,
                        );
                    }
                }
                _ => {}
            }
        });
    } else {
        ui.push_id(path, |ui| {
            ui.add_space(5.);
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width((ui.available_width() * 0.42).clamp(160., 350.));
                    ui.label(crate::i18n::widget(
                        RichText::new(crate::i18n::text(label(key))).strong(),
                    ))
                    .on_hover_text(crate::i18n::widget(key));
                    if options.technical && label(key) != key {
                        ui.label(crate::i18n::widget(
                            RichText::new(crate::i18n::text(key))
                                .small()
                                .monospace()
                                .weak(),
                        ));
                    }
                });
                ui.add(
                    egui::Label::new(crate::i18n::widget(
                        RichText::new(crate::i18n::text(display(value))).strong(),
                    ))
                    .wrap()
                    .selectable(true),
                );
            });
            if options.explanations {
                ui.add(
                    egui::Label::new(crate::i18n::widget(
                        RichText::new(crate::i18n::text(replay_data::explanation(key)))
                            .small()
                            .weak(),
                    ))
                    .wrap(),
                );
            }
            ui.add_space(3.);
            ui.separator();
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn large_ids_remain_exact_and_search_reaches_nested_fields() {
        assert_eq!(
            display(&serde_json::json!(18446744073709551615u64)),
            "18.446.744.073.709.551.615"
        );
        assert_ne!(display(&Value::Null), display(&serde_json::json!(0)));
        assert!(matches(
            "personal",
            &serde_json::json!({"1":{"damageBlockedByArmor":1234}}),
            "panzerung"
        ));
        assert!(matches(
            "personal",
            &serde_json::json!({"future":{"deep":"seltener Wert"}}),
            "seltener wert"
        ));
    }
}
