//! A scan session retains the store and only returns changed battle records.
use crate::{
    replay::Battle,
    store::{ImportReport, Store},
    vehicles,
};
use anyhow::Result;
use std::path::{Path, PathBuf};
#[derive(Default)]
pub struct Worker {
    database: Option<PathBuf>,
    store: Option<Store>,
    loaded: bool,
}
pub struct Loaded {
    pub battles: Vec<Battle>,
    pub replace_all: bool,
    pub imported: usize,
    pub unchanged: usize,
    pub errors: Vec<String>,
    pub projections_changed: bool,
}
impl Worker {
    pub fn scan(
        &mut self,
        database: &Path,
        game: &Path,
        folder: &Path,
        import: bool,
    ) -> Result<Loaded> {
        if self.database.as_deref() != Some(database) {
            self.store = Some(Store::open(database)?);
            self.database = Some(database.to_owned());
            self.loaded = false;
        }
        let store = self.store.as_mut().expect("store initialized");
        let report = if import {
            match vehicles::load_cached(game) {
                Ok(catalog) => match store.import(folder, &catalog) {
                    Ok(report) => report,
                    Err(e) => ImportReport {
                        errors: vec![format!("Import: {e:#}")],
                        ..Default::default()
                    },
                },
                Err(e) => ImportReport {
                    errors: vec![format!("Fahrzeugkatalog: {e:#}")],
                    ..Default::default()
                },
            }
        } else {
            ImportReport::default()
        };
        let replace_all = !self.loaded;
        let battles = if replace_all {
            store.battles()?
        } else {
            report.updated
        };
        self.loaded = true;
        Ok(Loaded {
            battles,
            replace_all,
            imported: report.imported,
            unchanged: report.unchanged,
            errors: report.errors,
            projections_changed: report.projections_changed,
        })
    }
}
