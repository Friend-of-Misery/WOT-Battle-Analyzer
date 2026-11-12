use eframe::egui;
use rusqlite::{Connection, OpenFlags, params};
use std::{
    path::Path,
    sync::mpsc,
    time::{Duration, Instant},
};

const PAGE: usize = 100;
#[derive(Clone, Default)]
struct Filter {
    search: String,
    account: Option<i64>,
    relation: String,
    outgoing: bool,
    page: usize,
}
struct Entry {
    key: String,
    account: Option<i64>,
    cells: Vec<String>,
}
struct Report {
    rows: Vec<Entry>,
    headings: Vec<&'static str>,
    count: usize,
    battles: usize,
    together: usize,
    against: usize,
}
#[derive(Default)]
pub struct State {
    filter: Filter,
    result: Option<Result<Report, String>>,
    receiver: Option<mpsc::Receiver<Result<Report, String>>>,
    initialized: bool,
}
const MATCH: &str = "(?1='' OR (?2 IS NOT NULL AND t.spieler_id=?2) OR (?2 IS NULL AND (t.spieler_id IN (SELECT x.spieler_id FROM gefecht_teilnehmer x LEFT JOIN spieler p ON p.account_id=x.spieler_id WHERE instr(lower(coalesce(x.echter_name,'')),lower(?1))>0 OR instr(lower(coalesce(x.angezeigter_name,'')),lower(?1))>0 OR instr(lower(coalesce(x.alias,'')),lower(?1))>0 OR instr(lower(coalesce(p.echter_name,'')),lower(?1))>0 OR cast(x.spieler_id AS TEXT)=?1) OR instr(lower(coalesce(t.angezeigter_name,'')),lower(?1))>0))) AND (?2 IS NULL OR t.spieler_id=?2) AND (?3='' OR t.zuordnung=?3)";
fn load(db: &Connection, filter: &Filter, damage: bool) -> anyhow::Result<Report> {
    let from = if damage {
        wot_insights::tr_format!(
            "v_spieler_gefechte t JOIN schaden_{} s USING(gefechts_id,fahrzeug_id)",
            if filter.outgoing {
                "ausgehend"
            } else {
                "eingehend"
            }
        )
    } else {
        "v_spieler_gefechte t".into()
    };
    let condition = format!("{MATCH} AND t.zuordnung<>'selbst'");
    let mut report=db.query_row(&format!("SELECT count(*),count(DISTINCT t.gefechts_id),count(DISTINCT CASE WHEN t.zuordnung='Mitspieler' THEN t.gefechts_id END),count(DISTINCT CASE WHEN t.zuordnung='Gegner' THEN t.gefechts_id END) FROM {from} WHERE {condition}"), params![filter.search,filter.account,filter.relation],|r| Ok(Report{rows:Vec::new(),headings:vec!["Datum / Details","Karte","Spieler","Account-ID","Alias / angezeigt","Team","Panzer","Stufe"],count:r.get(0)?,battles:r.get(1)?,together:r.get(2)?,against:r.get(3)?}))?;
    let stats = if damage {
        report.headings.extend([
            if filter.outgoing {
                "Schaden an ihm"
            } else {
                "Schaden von ihm"
            },
            "Direkte Treffer",
            "Durchschläge",
            "Explosionstreffer",
            "Munition",
            "Gesamtschaden Spieler",
        ]);
        "s.schaden,s.direkte_treffer,s.durchschlagstreffer,s.explosionstreffer,s.munitionstyp,t.gesamt_schaden"
    } else {
        report.headings.extend([
            "Gesamtschaden",
            "Schüsse",
            "Treffer",
            "Durchschläge",
            "Assist Aufklärung",
            "Assist Kette",
            "Assist Betäubung",
        ]);
        "t.gesamt_schaden,t.schuesse,t.treffer,t.durchschlaege,t.assist_aufklaerung,t.assist_kette,t.assist_betaeubung"
    };
    let mut stmt=db.prepare(&format!("SELECT t.gefechts_id,t.spieler_id,t.datum,t.karte,coalesce(t.echter_name,t.angezeigter_name,'unbekannt'),coalesce(t.alias,t.angezeigter_name,'–'),t.zuordnung,t.panzer,t.stufe,{stats} FROM {from} WHERE {condition} ORDER BY t.datum DESC,t.gefechts_id,t.fahrzeug_id{} LIMIT ?4 OFFSET ?5",if damage {",s.eigenes_fahrzeug,s.leben_index"} else {""}))?;
    let count = stmt.column_count();
    let rows = stmt.query_map(
        params![
            filter.search,
            filter.account,
            filter.relation,
            PAGE as i64,
            (filter.page * PAGE) as i64
        ],
        |r| {
            let mut cells = Vec::new();
            for i in 2..count {
                cells.push(match r.get_ref(i)? {
                    rusqlite::types::ValueRef::Null => "–".into(),
                    rusqlite::types::ValueRef::Integer(v) => v.to_string(),
                    rusqlite::types::ValueRef::Text(v) => String::from_utf8_lossy(v).into_owned(),
                    _ => "–".into(),
                });
            }
            Ok(Entry {
                key: r.get(0)?,
                account: r.get(1)?,
                cells,
            })
        },
    )?;
    report.rows = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(report)
}
impl State {
    pub fn invalidate(&mut self) {
        self.initialized = false;
        self.receiver = None;
    }
    fn start(&mut self, path: &Path, damage: bool) {
        let filter = self.filter.clone();
        let path = path.to_path_buf();
        let (tx, rx) = mpsc::channel();
        self.receiver = Some(rx);
        self.result = None;
        self.initialized = true;
        std::thread::spawn(move || {
            let result = (|| -> anyhow::Result<Report> {
                let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
                db.busy_timeout(Duration::from_secs(2))?;
                let start = Instant::now();
                db.progress_handler(
                    1000,
                    Some(move || start.elapsed() > Duration::from_secs(10)),
                );
                load(&db, &filter, damage)
            })()
            .map_err(|e| wot_insights::tr_format!("{e:#}"));
            let _ = tx.send(result);
        });
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        path: &Path,
        damage: bool,
        selected: &mut Option<String>,
    ) {
        if let Some(rx) = &self.receiver {
            match rx.try_recv() {
                Ok(result) => {
                    self.result = Some(result);
                    self.receiver = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.result = Some(Err("Abfrage unterbrochen. Bitte aktualisieren.".into()));
                    self.receiver = None;
                }
                _ => ui.ctx().request_repaint_after(Duration::from_millis(100)),
            }
        }
        let mut run = false;
        ui.heading(crate::i18n::rich(if damage {
            "Schaden"
        } else {
            "Spielersuche"
        }));
        if damage {
            ui.horizontal(|ui| {
                run |= ui
                    .selectable_value(
                        &mut self.filter.outgoing,
                        false,
                        crate::i18n::widget("Eingehender Schaden"),
                    )
                    .changed();
                run |= ui
                    .selectable_value(
                        &mut self.filter.outgoing,
                        true,
                        crate::i18n::widget("Ausgehender Schaden"),
                    )
                    .changed();
            });
            ui.weak(crate::i18n::rich("Summen je Gefecht und Gegenfahrzeug. Unbelegte Werte: –. Munitionstyp und eingehende Durchschläge/Explosionstreffer fehlen in den Ergebnisdetails. Explosionstreffer belegen allein keine HE-Munition."));
        } else {
            ui.label(crate::i18n::widget("Nach Accountname, Alias oder Account-ID suchen. Die Account-ID verknüpft auch Begegnungen unter anderen Namen."));
            ui.weak(crate::i18n::rich("Gesamtschaden, Schüsse, Treffer, Durchschläge und Assist stammen aus dem Abschlussbericht des jeweiligen Spielers. Fehlende Werte: –."));
        }
        ui.horizontal_wrapped(|ui| {
            ui.label(crate::i18n::widget("Spieler / Alias / ID"));
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.filter.search)
                    .hint_text(crate::i18n::widget("Name oder Account-ID …")),
            );
            if response.changed() {
                self.filter.account = None;
            }
            if ui.button(crate::i18n::widget("Suchen")).clicked()
                || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
            {
                run = true;
            }
            if ui.button(crate::i18n::widget("Zurücksetzen")).clicked() {
                self.filter = Filter {
                    outgoing: self.filter.outgoing,
                    ..Default::default()
                };
                self.result = None;
                self.initialized = false;
                run = damage;
            }
            if ui.button(crate::i18n::widget("Aktualisieren")).clicked() {
                run = true;
            }
        });
        ui.horizontal_wrapped(|ui| {
            for (label, relation) in [
                ("Alle Begegnungen", ""),
                ("Zusammen", "Mitspieler"),
                ("Gegeneinander", "Gegner"),
            ] {
                run |= ui
                    .selectable_value(
                        &mut self.filter.relation,
                        relation.to_owned(),
                        crate::i18n::widget(label),
                    )
                    .changed();
            }
            if let Some(id) = self.filter.account {
                ui.label(wot_insights::tr_format!("Account: {id}"));
            }
        });
        if run {
            self.filter.page = 0;
        }
        if (run || (!self.initialized && self.receiver.is_none()))
            && (damage || !self.filter.search.trim().is_empty() || self.filter.account.is_some())
        {
            self.start(path, damage);
        }
        if self.receiver.is_some() {
            ui.spinner();
            ui.label(crate::i18n::widget("Lade Gefechte …"));
        }
        let mut account = None;
        let mut page = None;
        if let Some(result) = &self.result {
            match result {
                Err(e) => {
                    ui.colored_label(egui::Color32::LIGHT_RED, egui::RichText::from(e).strong());
                    ui.label(crate::i18n::widget(
                        "Nach dem Update einmal Jetzt einlesen ausführen.",
                    ));
                }
                Ok(report) => {
                    ui.label(wot_insights::tr_format!(
                        "{} Einträge · {} Gefechte · {} zusammen · {} gegeneinander",
                        report.count,
                        report.battles,
                        report.together,
                        report.against
                    ));
                    ui.weak(crate::i18n::rich("Ein Klick auf das Datum öffnet die Gefechtsdetails. Ein Klick auf einen Spieler zeigt nur diesen Account. Die Filter der anderen Reiter gelten hier nicht."));
                    if report.rows.is_empty() {
                        ui.label(crate::i18n::widget("Keine passenden Einträge. Gegebenenfalls zuerst Jetzt einlesen ausführen."));
                    }
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                self.filter.page > 0,
                                egui::Button::new(crate::i18n::widget("← Zurück")),
                            )
                            .clicked()
                        {
                            page = Some(self.filter.page - 1);
                        }
                        ui.label(wot_insights::tr_format!(
                            "Seite {} / {}",
                            self.filter.page + 1,
                            report.count.div_ceil(PAGE).max(1)
                        ));
                        if ui
                            .add_enabled(
                                (self.filter.page + 1) * PAGE < report.count,
                                egui::Button::new(crate::i18n::widget("Weiter →")),
                            )
                            .clicked()
                        {
                            page = Some(self.filter.page + 1);
                        }
                    });
                    egui::ScrollArea::both().id_salt(("interactions",damage)).show(ui,|ui|{
                    egui::Grid::new(("interaction_rows",damage)).striped(true).show(ui,|ui|{
                        for h in &report.headings {ui.strong(crate::i18n::rich(*h));}ui.end_row();
                        for row in &report.rows {
                            for (i,cell) in row.cells.iter().enumerate(){
                                if i==0 {if ui.link(cell).clicked(){*selected=Some(row.key.clone());}}
                                else if i==2 {
                                    if ui.link(cell).on_hover_text(crate::i18n::widget(row.account.map(|v|wot_insights::tr_format!("Account-ID: {v} · Alle Begegnungen anzeigen")).unwrap_or_else(||"Account-ID unbekannt".into()))).clicked(){account=row.account;}
                                    if let Some(id)=row.account {ui.label(crate::i18n::widget(id.to_string()));} else {ui.label(crate::i18n::widget("–"));}
                                } else if i==4 {ui.label(crate::i18n::text(cell));} else if i==5 {ui.label(crate::ui::tank_name(cell));}
                                else {ui.add(egui::Label::new(cell).wrap_mode(egui::TextWrapMode::Extend));}
                            }
                            ui.end_row();
                        }
                    });
                });
                }
            }
        } else if !damage && self.receiver.is_none() {
            ui.label(crate::i18n::widget(
                "Gib einen Spielernamen, Alias oder eine Account-ID ein.",
            ));
        }
        if let Some(id) = account {
            self.filter.account = Some(id);
            self.filter.search = id.to_string();
            self.filter.page = 0;
            self.start(path, damage);
        }
        if let Some(page) = page {
            self.filter.page = page;
            self.start(path, damage);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        wot_insights::damage::initialize(&db).unwrap();
        db.execute_batch("INSERT INTO players VALUES (1,'Me'),(2,'LatestName'),(3,'Other');
            INSERT INTO battles(battle_id,date,map,map_id,own_player_id,results_available) VALUES ('a','2026-11-10','Map A','map_a',1,1),('b','2026-11-11','Map B','map_b',1,1);
            INSERT INTO battle_participants(battle_id,vehicle_id,player_id,real_name,display_name,alias,team,total_damage,shots,hits,penetrations,spotting_assist,tracking_assist,stun_assist) VALUES
            ('a',10,1,'Me','Me',NULL,1,100,1,1,1,0,0,0),
            ('a',20,2,'OldName','SecretAlias','SecretAlias',1,2000,12,10,8,600,250,0),
            ('b',11,1,'Me','Me',NULL,1,100,1,1,1,0,0,0),
            ('b',21,2,'NewName','DifferentAlias','DifferentAlias',2,3000,15,11,9,0,400,100),
            ('b',31,3,'Other','Other',NULL,2,NULL,NULL,NULL,NULL,NULL,NULL,NULL);
            INSERT INTO incoming_damage VALUES ('b','own',21,0,500,2,NULL,NULL,NULL,200,1);
            INSERT INTO outgoing_damage VALUES ('b','own',21,0,300,1,1,0,NULL);") .unwrap();
        db
    }
    #[test]
    fn search_follows_account_across_aliases_and_distinguishes_teams() {
        let db = fixture();
        let mut f = Filter {
            search: "SecretAlias".into(),
            ..Default::default()
        };
        let report = load(&db, &f, false).unwrap();
        assert_eq!(
            (
                report.count,
                report.battles,
                report.together,
                report.against
            ),
            (2, 2, 1, 1)
        );
        assert_eq!(report.rows[0].cells[2], "NewName");
        assert_eq!(
            &report.rows[0].cells[7..],
            &["3000", "15", "11", "9", "0", "400", "100"]
        );
        assert_eq!(report.headings.len(), report.rows[0].cells.len() + 1);
        f.relation = "Mitspieler".into();
        let report = load(&db, &f, false).unwrap();
        assert_eq!(report.count, 1);
        assert_eq!(report.rows[0].key, "a");
        f.relation = "Gegner".into();
        assert_eq!(load(&db, &f, false).unwrap().rows[0].key, "b");
        f.relation.clear();
        f.search = "LatestName".into();
        assert_eq!(load(&db, &f, false).unwrap().count, 2);
        f.search = "2".into();
        f.account = Some(2);
        assert_eq!(load(&db, &f, false).unwrap().count, 2);
        f.account = None;
        f.search = "' OR 1=1 --".into();
        assert_eq!(load(&db, &f, false).unwrap().count, 0);
    }
    #[test]
    fn damage_direction_totals_unknowns_and_pagination() {
        let db = fixture();
        let mut f = Filter::default();
        let incoming = load(&db, &f, true).unwrap();
        assert_eq!(incoming.count, 1);
        assert_eq!(
            &incoming.rows[0].cells[7..],
            &["500", "2", "–", "–", "–", "3000"]
        );
        f.outgoing = true;
        let outgoing = load(&db, &f, true).unwrap();
        assert_eq!(
            &outgoing.rows[0].cells[7..],
            &["300", "1", "1", "0", "–", "3000"]
        );
        f.search = "Other".into();
        let report = load(&db, &f, false).unwrap();
        assert!(report.rows[0].cells[7..].iter().all(|v| v == "–"));
        for i in 100..205 {
            db.execute("INSERT INTO battle_participants(battle_id,vehicle_id,player_id,team,real_name) VALUES ('b',?1,3,2,'Other')",[i]).unwrap();
        }
        f.search.clear();
        let first = load(&db, &f, false).unwrap();
        assert_eq!(first.rows.len(), PAGE);
        assert_eq!(first.count, 108);
        f.page = 1;
        let next = load(&db, &f, false).unwrap();
        assert_eq!(next.rows.len(), 8);
    }
    #[test]
    fn populated_views_render_at_supported_sizes() {
        let db = fixture();
        for size in [egui::vec2(1000., 620.), egui::vec2(1280., 820.)] {
            for damage in [false, true] {
                let ctx = egui::Context::default();
                crate::ui::configure(&ctx);
                let mut state = State {
                    initialized: true,
                    result: Some(Ok(load(&db, &Filter::default(), damage).unwrap())),
                    ..Default::default()
                };
                let mut selected = None;
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            state.show(ui, Path::new("unused.db"), damage, &mut selected)
                        });
                    },
                );
                assert!(!output.shapes.is_empty());
                assert!(output.shapes.iter().all(|s| s.clip_rect.is_finite()));
            }
        }
    }
}
