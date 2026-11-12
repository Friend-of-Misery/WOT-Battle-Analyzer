use super::*;
impl App {
    pub(super) fn scan(&mut self) {
        self.scan_with_import(true);
    }
    pub(super) fn scan_with_import(&mut self, import: bool) {
        if self.receiver.is_some() || self.cleanup_receiver.is_some() {
            return;
        }
        let (sender, receiver) = mpsc::channel();
        self.receiver = Some(receiver);
        self.message = "Replay-Ordner wird geprüft …".into();
        let (game, folder, db) = (
            PathBuf::from(&self.preferences.game_folder),
            PathBuf::from(&self.preferences.replay_folder),
            self.database.clone(),
        );
        let worker = std::sync::Arc::clone(&self.worker);
        std::thread::spawn(move || {
            let result = worker
                .lock()
                .map_err(|_| anyhow::anyhow!("Scan-Sitzung unterbrochen"))
                .and_then(|mut worker| worker.scan(&db, &game, &folder, import));
            let _ = sender.send(result);
        });
    }

    pub(super) fn poll(&mut self) {
        match self.cleanup_receiver.as_ref().map(|r| r.try_recv()) {
            Some(Ok(result)) => {
                self.cleanup_receiver = None;
                self.cleanup_message = match result {
                    Ok((deleted, bytes, errors)) => wot_insights::tr_format!(
                        "{deleted} Originaldateien gelöscht ({:.1} MB). Alle Daten bleiben archiviert. {} Dateien wegen Fehlern beibehalten.{}",
                        bytes as f64 / 1_000_000.,
                        errors.len(),
                        errors
                            .iter()
                            .take(10)
                            .map(|e| format!("\n{e}"))
                            .collect::<String>()
                    ),
                    Err(error) => wot_insights::tr_format!("Bereinigung fehlgeschlagen: {error:#}"),
                };
            }
            Some(Err(mpsc::TryRecvError::Disconnected)) => {
                self.cleanup_receiver = None;
                self.cleanup_message =
                    "Bereinigung unterbrochen. Nicht gelöschte Dateien bleiben erhalten.".into();
            }
            _ => {}
        }
        let reply = self.receiver.as_ref().map(|r| r.try_recv());
        match reply {
            Some(Ok(result)) => {
                self.receiver = None;
                self.last_scan = Instant::now();
                match result {
                    Ok(loaded) => {
                        let changed = loaded.replace_all
                            || !loaded.battles.is_empty()
                            || loaded.projections_changed;
                        if loaded.replace_all {
                            self.battles = loaded.battles;
                        } else if !loaded.battles.is_empty() {
                            let mut positions: std::collections::HashMap<_, _> = self
                                .battles
                                .iter()
                                .enumerate()
                                .map(|(i, b)| (b.key.clone(), i))
                                .collect();
                            for battle in loaded.battles {
                                if let Some(&i) = positions.get(&battle.key) {
                                    self.battles[i] = battle;
                                } else {
                                    positions.insert(battle.key.clone(), self.battles.len());
                                    self.battles.push(battle);
                                }
                            }
                            self.battles
                                .sort_by(|a, b| b.date.cmp(&a.date).then(a.key.cmp(&b.key)));
                        }
                        if changed {
                            self.damage_view.invalidate();
                            self.player_view.invalidate();
                        }
                        self.message = wot_insights::tr_format!(
                            "{} aktualisiert · {} unverändert · {} übersprungen",
                            loaded.imported,
                            loaded.unchanged,
                            loaded.errors.len()
                        );
                        self.errors = loaded.errors;
                        self.import_failed = false;
                        self.last_success = Some(Local::now().format("%H:%M:%S").to_string());
                        if self.preferences.filter.player.is_empty() {
                            self.preferences.filter.player = self
                                .battles
                                .first()
                                .map(|b| b.player.clone())
                                .unwrap_or_default();
                        }
                    }
                    Err(e) => {
                        self.message = wot_insights::tr_format!("Import fehlgeschlagen: {e:#}");
                        self.import_failed = true;
                    }
                }
            }
            Some(Err(mpsc::TryRecvError::Disconnected)) => {
                self.receiver = None;
                self.last_scan = Instant::now();
                self.import_failed = true;
                self.message =
                    "Import wurde unterbrochen. Erneut einlesen oder Datenquelle prüfen.".into();
            }
            _ => {}
        }
        if self.preferences.automatic
            && self.receiver.is_none()
            && self.last_scan.elapsed() > Duration::from_secs(30)
        {
            self.scan();
        }
    }
}
