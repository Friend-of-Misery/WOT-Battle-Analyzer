use super::*;
impl App {
    pub(super) fn settings(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui,|ui|{
            ui.heading(crate::i18n::rich("Darstellung"));
            ui.horizontal_wrapped(|ui|{
                let changed=ui.add(egui::Slider::new(&mut self.preferences.font_size,14.0..=22.0).step_by(1.0).text(crate::i18n::text("Schriftgröße"))).changed();
                if ui.button(crate::i18n::widget("Zurücksetzen")).clicked(){self.preferences.font_size=17.;typography(ui.ctx(),17.);}
                else if changed{typography(ui.ctx(),self.preferences.font_size);}
            });
            ui.separator();
            ui.heading(crate::i18n::rich("Sprache"));
            egui::ComboBox::from_id_salt("ui_language").selected_text(crate::i18n::LANGUAGES.iter().find(|(code,_)|*code==self.preferences.language).map(|(_,name)|*name).unwrap_or("Deutsch")).show_ui(ui,|ui|{
                for (code,name) in crate::i18n::LANGUAGES {if ui.selectable_value(&mut self.preferences.language,code.to_string(),*name).changed(){crate::i18n::set_language(code);}}
            });
            ui.separator();
            ui.heading(crate::i18n::rich("Datenquelle & Erfassung"));
            ui.checkbox(&mut self.preferences.sql_enabled,crate::i18n::widget("SQL-Reiter für eigene Abfragen anzeigen"));
            ui.label(crate::i18n::widget("WoT-Installationsordner"));ui.text_edit_singleline(&mut self.preferences.game_folder);
            ui.label(crate::i18n::widget("Replay-Ordner"));ui.text_edit_singleline(&mut self.preferences.replay_folder);
            ui.checkbox(&mut self.preferences.automatic,crate::i18n::widget("Neue Replays automatisch alle 30 Sekunden einlesen"));
            if ui.add_enabled(self.receiver.is_none(),egui::Button::new(crate::i18n::widget("Datenquelle prüfen und einlesen"))).clicked(){self.scan();}
            ui.separator();ui.label(wot_insights::tr_format!("Datenbank: {}",self.database.display()));ui.label(wot_insights::tr_format!("Einstellungen: {}",self.settings_path.display()));
            ui.label(crate::i18n::widget("Favoriten, Filter, Ansicht und sichtbare Spalten werden automatisch gespeichert."));
            ui.add_space(10.);ui.heading(crate::i18n::rich("Was die Werte bedeuten"));
            for note in [
                "Die Daten stammen aus lokalen Replays; Panzerklassen und Stufen aus dem installierten Client.",
                "Fehlende Endergebnisse werden nicht als Niederlage oder 0 Schaden gezählt. In Listen bleiben sie als offen sichtbar.",
                "Letzte Session: deine jüngste Folge von Zufallsgefechten, getrennt durch eine Pause von mehr als 60 Minuten. Die Session bleibt beim Wechsel von Panzer oder Klasse gleich.",
                "Grüne Winrate: mindestens 50 %. In der Kartenmatrix beziehen sich die Farben auf dein eigenes Klassenmittel im aktuellen Filter.",
                "Assist = Aufklärung + Kette + Betäubung. Verdienst zeigt Brutto, Ausgaben und Netto. Schaden/Erhalten ist das Verhältnis der Summen aus Gefechten mit beiden Werten.",
                "Credits netto = credits − autoRepairCost − autoLoadCost[0] − autoEquipCost[0]. Fehlende Abzüge bleiben unbekannt. Aufgaben/Ereignisse sind bereits im Brutto enthalten. Vergleichswerte ohne Aufgaben oder Reserven sind bereinigte Werte, kein zusätzlicher Gewinn.",
                "Ausgaben sind die im Replay ausgewiesenen automatischen Abzüge. Käufe außerhalb des Gefechts und historische Anschaffungskosten von Lagerbeständen sind nicht vollständig erfasst. Gold und Anleihen werden nicht in Credits umgerechnet.",
                "Verläufe zeigen Tageswerte, keine kumulierte Kontostatistik. Panzerlisten gelten für den ausgewählten Zeitraum.",
                "Noch nicht enthalten: WN8, Dossier-/Ergebnis-Cache-Import, konfigurierbare Widgets und 3D-Trefferanalyse. Vorzeitig verlassene Gefechte können ohne Ergebnis bleiben.",
                "Spieldateien werden nicht verändert. Replay-Dateien werden nur über Replay-Archiv nach vollständiger Archivierung und ausdrücklicher Auswahl gelöscht. Kein XVM und keine Netzwerkabfragen zur Laufzeit."]{ui.label(crate::i18n::widget(note));}
            if !self.errors.is_empty(){ui.add_space(10.);ui.heading(wot_insights::tr_format!("{} Dateien übersprungen",self.errors.len()));for e in self.errors.iter().take(30){ui.colored_label(RED,egui::RichText::from(e).strong());}}
        });
    }
    pub(super) fn replay_archive(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading(crate::i18n::rich("Replay-Archiv & Bereinigung"));
            ui.label(crate::i18n::widget("Beim Import werden sämtliche JSON-Blöcke einschließlich unbekannter Felder und der vollständige binäre Gefechtsverlauf verlustfrei in der Datenbank archiviert."));
            ui.weak(crate::i18n::rich("Das Archiv bewahrt alle Daten. Die Statistikansichten werten bisher nur einen Teil davon aus. Der binäre Verlauf wird noch nicht vollständig dekodiert."));
            ui.label(crate::i18n::widget("Replays älter als 30 Tage löschen"));
            ui.label(wot_insights::tr_format!("Replay-Ordner: {}", self.preferences.replay_folder));
            ui.weak(crate::i18n::rich("Das Gefechtsdatum im Replay entscheidet. Unterordner werden berücksichtigt; Verknüpfungen sowie temp.wotreplay und replay_last_battle.wotreplay werden ausgelassen."));
            ui.weak(crate::i18n::rich("Die Originaldateien werden endgültig gelöscht, ihre Daten bleiben vollständig im Archiv. Da der Gefechtsverlauf bereits komprimiert ist, spart die Archivierung insgesamt wenig Speicherplatz."));
            if ui.add_enabled(self.receiver.is_none() && self.cleanup_receiver.is_none(), egui::Button::new(crate::i18n::widget("Replays älter als 30 Tage suchen"))).clicked() {
                self.cleanup_confirmed = false;
                self.cleanup_candidates.clear();
                self.cleanup_folder = self.preferences.replay_folder.clone();
                match wot_insights::archive::preview(std::path::Path::new(&self.cleanup_folder)) {
                    Ok(candidates) => {
                        self.cleanup_candidates = candidates;
                        self.cleanup_message = wot_insights::tr_format!("{} Replay-Dateien gefunden.", self.cleanup_candidates.len());
                    }
                    Err(error) => self.cleanup_message = wot_insights::tr_format!("Suche fehlgeschlagen: {error:#}"),
                }
            }
            ui.label(crate::i18n::widget(&self.cleanup_message));
            if !self.cleanup_candidates.is_empty() {
                let size: u64 = self.cleanup_candidates.iter().map(|c| c.size).sum();
                ui.label(wot_insights::tr_format!("{} Dateien · {:.1} MB Originaldateien (Archiv benötigt zusätzlichen Platz in der Datenbank)", self.cleanup_candidates.len(), size as f64 / 1_000_000.));
                egui::ScrollArea::vertical().id_salt("cleanup_preview").max_height(220.).show(ui, |ui| {
                    for candidate in &self.cleanup_candidates {
                        ui.label(wot_insights::tr_format!("{} · {:.2} MB · {}", candidate.date, candidate.size as f64 / 1_000_000., candidate.path.display()));
                    }
                });
                ui.checkbox(&mut self.cleanup_confirmed,crate::i18n::widget( "Die aufgelisteten Originaldateien nach geprüfter Archivierung endgültig löschen"));
                let enabled = self.cleanup_confirmed && self.receiver.is_none() && self.cleanup_folder == self.preferences.replay_folder;
                if ui.add_enabled(enabled, egui::Button::new(crate::i18n::widget("Jetzt archivieren und Originaldateien löschen"))).clicked() {
                    let (sender, receiver) = mpsc::channel();
                    self.cleanup_receiver = Some(receiver);
                    let database = self.database.clone();
                    let folder = PathBuf::from(&self.cleanup_folder);
                    let candidates = std::mem::take(&mut self.cleanup_candidates);
                    self.cleanup_message = "Archivierung und Bereinigung laufen … App bitte geöffnet lassen.".into();
                    std::thread::spawn(move || {
                        let _ = sender.send(wot_insights::archive::delete_archived(&database, &folder, &candidates));
                    });
                    self.cleanup_confirmed = false;
                }
            }
        });
    }
}
