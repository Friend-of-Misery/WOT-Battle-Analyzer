use eframe::egui;
use rusqlite::{Connection, OpenFlags, types::ValueRef};
use std::{
    path::Path,
    sync::mpsc,
    time::{Duration, Instant},
};
struct Query {
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
    truncated: bool,
}
type Reply = Result<Query, String>;
pub struct State {
    editor: String,
    player_search: String,
    result: Option<Reply>,
    receiver: Option<mpsc::Receiver<Reply>>,
    schema: Option<Result<Vec<(String, String, Query)>, String>>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            editor: "SELECT * FROM battles LIMIT 100;".into(),
            player_search: String::new(),
            result: None,
            receiver: None,
            schema: None,
        }
    }
}
fn connect(path: &Path) -> anyhow::Result<Connection> {
    let db = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
    )?;
    wot_insights::migrations::attach_readonly_archive(&db, path)?;
    db.busy_timeout(Duration::from_secs(2))?;
    db.execute_batch("PRAGMA query_only=ON;")?;
    let start = Instant::now();
    db.progress_handler(1000, Some(move || start.elapsed() > Duration::from_secs(5)));
    Ok(db)
}
fn query(db: &Connection, sql: &str) -> anyhow::Result<Query> {
    let mut stmt = db.prepare(sql)?;
    anyhow::ensure!(
        stmt.readonly() && stmt.column_count() > 0,
        "Nur lesende Abfragen mit Ergebnisspalten sind erlaubt."
    );
    let columns = stmt
        .column_names()
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let mut cursor = stmt.query([])?;
    let mut rows = Vec::new();
    let mut truncated = false;
    while let Some(row) = cursor.next()? {
        if rows.len() == 1000 {
            truncated = true;
            break;
        }
        let mut cells = Vec::new();
        for i in 0..columns.len() {
            cells.push(match row.get_ref(i)? {
                ValueRef::Null => "NULL".into(),
                ValueRef::Integer(v) => v.to_string(),
                ValueRef::Real(v) => v.to_string(),
                ValueRef::Text(v) => {
                    let text = String::from_utf8_lossy(v);
                    let mut cell: String = text.chars().take(2000).collect();
                    if text.chars().count() > 2000 {
                        cell.push_str(" … [gekürzt]");
                    }
                    cell
                }
                ValueRef::Blob(v) => {
                    wot_insights::tr_format!("[BLOB: {} Bytes]", v.len())
                }
            });
        }
        rows.push(cells);
    }
    Ok(Query {
        columns,
        rows,
        truncated,
    })
}
fn schema(path: &Path) -> anyhow::Result<Vec<(String, String, Query)>> {
    let db = connect(path)?;
    let attached: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM pragma_database_list WHERE name='archive_store')",
        [],
        |r| r.get(0),
    )?;
    let mut sql="SELECT 'main',name,coalesce(sql,'') FROM main.sqlite_schema WHERE type IN ('table','view')".to_owned();
    if attached {
        sql.push_str(" UNION ALL SELECT 'archive_store',name,coalesce(sql,'') FROM archive_store.sqlite_schema WHERE type IN ('table','view')");
    }
    sql.push_str(" ORDER BY 1,2");
    let mut stmt = db.prepare(&sql)?;
    let entries = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    let mut tables = Vec::new();
    for entry in entries {
        let (namespace, name, ddl) = entry?;
        let escaped = name.replace('\'', "''");
        let columns = query(
            &db,
            &format!(
                "SELECT cid AS Position,name AS Spalte,type AS Datentyp,[notnull] AS Pflichtfeld,dflt_value AS Standardwert,pk AS Primaerschluessel,hidden AS Versteckt FROM pragma_table_xinfo('{escaped}','{namespace}')"
            ),
        )?;
        tables.push((
            if namespace == "main" {
                name
            } else {
                format!("{namespace}.{name}")
            },
            ddl,
            columns,
        ));
    }
    Ok(tables)
}
fn grid(ui: &mut egui::Ui, id: impl std::hash::Hash, result: &Query) {
    egui::ScrollArea::both()
        .id_salt(&id)
        .max_height(350.)
        .show(ui, |ui| {
            egui::Grid::new(id).striped(true).show(ui, |ui| {
                for column in &result.columns {
                    ui.strong(column);
                }
                ui.end_row();
                for row in &result.rows {
                    for cell in row {
                        ui.add(egui::Label::new(cell).wrap_mode(egui::TextWrapMode::Extend));
                    }
                    ui.end_row();
                }
            });
        });
}
impl State {
    pub fn show(&mut self, ui: &mut egui::Ui, path: &Path) {
        if let Some(rx) = &self.receiver {
            match rx.try_recv() {
                Ok(result) => {
                    self.result = Some(result);
                    self.receiver = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.result = Some(Err("Abfrage wurde unterbrochen.".into()));
                    self.receiver = None;
                }
                _ => {
                    ui.ctx().request_repaint_after(Duration::from_millis(100));
                }
            }
        }
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading(crate::i18n::rich("SQL-Abfragen"));
            ui.horizontal_wrapped(|ui| {
                ui.label(crate::i18n::widget("Spieler / Alias:"));
                ui.text_edit_singleline(&mut self.player_search);
                if ui.add_enabled(!self.player_search.trim().is_empty(), egui::Button::new(crate::i18n::widget("Suchabfrage übernehmen"))).clicked() {
                    let name = self.player_search.trim().replace('\'', "''");
                    self.editor = format!("SELECT datum, karte, echter_name, angezeigter_name, alias, panzer, stufe, zuordnung, spieler_id\nFROM v_spieler_gefechte\nWHERE instr(lower(coalesce(echter_name,'')), lower('{name}')) > 0\n   OR instr(lower(coalesce(angezeigter_name,'')), lower('{name}')) > 0\n   OR instr(lower(coalesce(alias,'')), lower('{name}')) > 0\nORDER BY datum DESC;");
                }
            });
            ui.horizontal_wrapped(|ui| {
                for (label, view) in [("Spieler & Gefechte", "v_spieler_gefechte"), ("Eingehender Schaden", "v_schaden_eingehend"), ("Ausgehender Schaden", "v_schaden_ausgehend")] {
                    if ui.button(crate::i18n::widget(label)).clicked() { self.editor = format!("SELECT * FROM {view}\nORDER BY datum DESC LIMIT 100;"); }
                }
            });
            ui.weak(crate::i18n::rich("Schaden: Summen je Gefecht und Gegenfahrzeug. Munitionstyp und eingehende Durchschläge/Explosionstreffer sind nicht belegt und bleiben NULL. Explosionstreffer allein belegen keinen HE-Munitionstyp."));
            ui.weak(wot_insights::tr_format!("Datenbank: {}", path.display()));
            ui.label(crate::i18n::widget("Lesezugriff · erste Anweisung pro Ausführung · maximal 1.000 Zeilen · Zeitlimit 5 Sekunden. Ansichtsfilter gelten hier nicht."));
            ui.add(egui::TextEdit::multiline(&mut self.editor).code_editor().desired_rows(6).desired_width(f32::INFINITY));
            if ui.add_enabled(self.receiver.is_none(), egui::Button::new(crate::i18n::widget("Abfrage ausführen"))).clicked() {
                let sql = self.editor.clone();
                let path = path.to_path_buf();
                let (tx, rx) = mpsc::channel();
                self.receiver = Some(rx);
                std::thread::spawn(move || {
                    let result = connect(&path).and_then(|db| query(&db, &sql)).map_err(|e| wot_insights::tr_format!("{e:#}"));
                    let _ = tx.send(result);
                });
                ui.ctx().request_repaint();
            }
            if self.receiver.is_some() { ui.spinner(); }
            if let Some(result) = &self.result {
                match result {
                    Ok(result) => {
                        ui.label(wot_insights::tr_format!("{} Zeilen{}", result.rows.len(), if result.truncated { " (auf 1.000 begrenzt)" } else { "" }));
                        grid(ui, "sql_results", result);
                    },
                    Err(error) => { ui.colored_label(egui::Color32::LIGHT_RED, error); }
                }
            }
            ui.separator();
            ui.heading(crate::i18n::rich("Tabellen & Spaltendefinitionen"));
            let refresh = ui.button(crate::i18n::widget("Schema aktualisieren")).clicked();
            if self.schema.is_none() || refresh {
                self.schema = Some(schema(path).map_err(|e| wot_insights::tr_format!("{e:#}")));
            }
            if let Some(result) = &self.schema {
                match result {
                    Ok(tables) => for (name, ddl, columns) in tables {
                        egui::CollapsingHeader::new(name).show(ui, |ui| {
                            if ui.button(crate::i18n::widget("SELECT-Vorlage übernehmen")).clicked() {
                                self.editor = format!("SELECT * FROM \"{}\" LIMIT 100;", name.split('.').map(|part|part.replace('"',"\"\"")).collect::<Vec<_>>().join("\".\""));
                            }
                            grid(ui, (name, "schema"), columns);
                            ui.monospace(ddl);
                        });
                    },
                    Err(error) => { ui.colored_label(egui::Color32::LIGHT_RED, error); }
                }
            }
        });
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn queries_and_schema() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");
        let db = Connection::open(&path).unwrap();
        db.execute_batch("CREATE TABLE sample (id INTEGER PRIMARY KEY, name TEXT NOT NULL DEFAULT 'test', data BLOB); INSERT INTO sample VALUES (1,'hello',X'0102');").unwrap();
        let reader = connect(&path).unwrap();
        let result = query(&reader, "SELECT *, NULL FROM sample").unwrap();
        assert_eq!(result.rows[0], ["1", "hello", "[BLOB: 2 Bytes]", "NULL"]);
        assert!(query(&reader, "DELETE FROM sample").is_err());
        assert_eq!(
            query(&reader, "SELECT 1; DELETE FROM sample;")
                .unwrap()
                .rows[0][0],
            "1"
        );
        assert!(query(&reader, "SELECT * FROM missing").is_err());
        let result = query(&reader, "WITH RECURSIVE n(x) AS (VALUES(1) UNION ALL SELECT x+1 FROM n WHERE x<1002) SELECT x FROM n").unwrap();
        assert_eq!(result.rows.len(), 1000);
        assert!(result.truncated);
        let tables = schema(&path).unwrap();
        assert_eq!(tables[0].0, "sample");
        assert_eq!(tables[0].2.rows[1][1], "name");
        assert_eq!(tables[0].2.rows[1][3], "1");
        assert_eq!(tables[0].2.rows[0][5], "1");
    }
}

#[cfg(test)]
mod archive_tests {
    use super::*;
    #[test]
    fn sql_browser_attaches_archive_read_only_and_lists_its_schema() {
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().join("Test mit Leerzeichen ü");
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("stats.db");
        let writer = wot_insights::migrations::open(&path).unwrap();
        writer
            .execute(
                "INSERT INTO archive_store.replay_blobs VALUES ('sample','now',X'010203')",
                [],
            )
            .unwrap();
        let reader = connect(&path).unwrap();
        assert_eq!(
            query(
                &reader,
                "SELECT length(payload) FROM archive_store.replay_blobs"
            )
            .unwrap()
            .rows[0][0],
            "3"
        );
        assert!(
            reader
                .execute("DELETE FROM archive_store.replay_blobs", [])
                .is_err()
        );
        assert!(
            schema(&path)
                .unwrap()
                .iter()
                .any(|(name, _, _)| name == "archive_store.replay_blobs")
        );
    }
}
