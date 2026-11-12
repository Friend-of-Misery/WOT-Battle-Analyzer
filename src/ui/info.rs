use super::*;
impl App {
    pub(super) fn info(&self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().id_salt("info_page").show(ui, |ui| {
            ui.heading(wot_insights::brand::NAME);
            ui.label(wot_insights::tr_format!("Version {}", env!("CARGO_PKG_VERSION")));
            ui.add_space(10.);
            ui.horizontal_wrapped(|ui| {
                ui.strong(crate::i18n::rich("Ersteller"));
                ui.label(wot_insights::brand::AUTHOR);
            });
            ui.separator();
            ui.label(crate::i18n::widget("Lokale Auswertung deiner World-of-Tanks-Replays: Gefechtsstatistiken, Karten, Verdienst, Schaden und Begegnungen mit anderen Spielern."));
            ui.label(crate::i18n::widget("Deine Daten bleiben lokal gespeichert. Für die Replay-Auswertung sind weder Login noch Cloud erforderlich."));
            ui.add_space(12.);
            ui.heading("GitHub");
            if let Some(url) = wot_insights::brand::GITHUB_URL.filter(|url| !url.trim().is_empty()) {
                ui.hyperlink_to(crate::i18n::text("Quellcode, Updates und Rückmeldungen"), url);
            } else {
                ui.label(crate::i18n::widget("Wird nach Veröffentlichung ergänzt."));
            }
        });
    }
}
