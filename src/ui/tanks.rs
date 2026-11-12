use super::*;
pub(super) fn tank_metric(row: &TankRow, column: Sort) -> Option<f64> {
    match column {
        Sort::Battles => Some(row.summary.count as f64),
        Sort::Winrate => row.summary.winrate(),
        Sort::Damage => row.summary.avg_damage(),
        Sort::Assist => row.summary.avg_assist(),
        Sort::Kills => row.summary.avg_kills(),
        Sort::Xp => row.summary.avg_xp(),
        Sort::Credits => row.summary.avg_credits(),
        Sort::Net => row.summary.economy.net.average(),
        Sort::CleanNet => row.summary.economy.without_missions.average(),
        Sort::UnboostedNet => row.summary.economy.without_reserves.average(),
        Sort::TotalNet => row.summary.economy.net.total().map(|n| n as f64),
        Sort::Expenses => row.summary.economy.expenses.average(),
        Sort::Shots => {
            (row.summary.accuracy.count > 0).then_some(row.summary.accuracy.shots as f64)
        }
        Sort::Hits => (row.summary.accuracy.count > 0).then_some(row.summary.accuracy.hits as f64),
        Sort::Penetrations => {
            (row.summary.accuracy.count > 0).then_some(row.summary.accuracy.penetrations as f64)
        }
        Sort::HitRate => row.summary.accuracy.hit_rate(),
        Sort::PenetrationRate => row.summary.accuracy.penetration_rate(),
        Sort::PenetrationPerShot => row.summary.accuracy.penetration_per_shot(),
        _ => None,
    }
}
pub(super) fn tank_table(
    ui: &mut egui::Ui,
    battles: &[&Battle],
    prefs: &mut Preferences,
    sorting: &mut Sorting,
) {
    let previous_mode = prefs.tank_mode;
    ui.horizontal(|ui| {
        ui.selectable_value(
            &mut prefs.tank_mode,
            0,
            crate::i18n::widget("Leistung & Verdienst"),
        );
        ui.selectable_value(&mut prefs.tank_mode, 1, crate::i18n::widget("Schussdaten"));
    });
    if previous_mode != prefs.tank_mode {
        sorting.column = if prefs.tank_mode == 1 {
            Sort::HitRate
        } else {
            Sort::Battles
        };
        sorting.descending = true;
    }
    if prefs.tank_mode == 1 {
        accuracy_tanks(ui, battles, prefs, sorting);
        return;
    }
    ui.weak(crate::i18n::rich("★ markiert einen Favoriten. Ein Klick auf den Panzernamen öffnet seine Gefechte. Spaltenköpfe sortieren."));
    let mut rows = analysis::tanks(battles);
    rows.sort_by(|a, b| {
        let order = if sorting.column == Sort::Name {
            tank_name(&a.tank).cmp(&tank_name(&b.tank))
        } else {
            tank_metric(a, sorting.column)
                .partial_cmp(&tank_metric(b, sorting.column))
                .unwrap_or(std::cmp::Ordering::Equal)
        };
        (if sorting.descending {
            order.reverse()
        } else {
            order
        })
        .then(a.tank.cmp(&b.tank))
    });
    egui::ScrollArea::both().show(ui, |ui| {
        egui::Grid::new("tanks")
            .striped(true)
            .spacing([15., 7.])
            .show(ui, |ui| {
                ui.label(crate::i18n::widget("★"));
                sorting.heading(ui, "Panzer", Sort::Name);
                sorting.heading(ui, "Ø Netto", Sort::Net);
                sorting.heading(ui, "Ø ohne Aufgaben", Sort::CleanNet);
                ui.strong(crate::i18n::rich("Klasse"));
                ui.strong(crate::i18n::rich("Nation"));
                ui.strong(crate::i18n::rich("Rolle"));
                ui.strong(crate::i18n::rich("Stufe"));
                sorting.heading(ui, "Gefechte", Sort::Battles);
                ui.strong(crate::i18n::rich("Ergebnisse"));
                sorting.heading(ui, "Winrate", Sort::Winrate);
                sorting.heading(ui, "Ø Schaden", Sort::Damage);
                sorting.heading(ui, "Ø Assist", Sort::Assist);
                sorting.heading(ui, "Ø Abschüsse", Sort::Kills);
                sorting.heading(ui, "Ø Basis-XP", Sort::Xp);
                ui.end_row();
                for row in rows {
                    let favorite = prefs.favorites.contains(&row.tank);
                    if ui
                        .selectable_label(
                            favorite,
                            crate::i18n::widget(if favorite { "★" } else { "☆" }),
                        )
                        .clicked()
                    {
                        if favorite {
                            prefs.favorites.remove(&row.tank);
                        } else {
                            prefs.favorites.insert(row.tank.clone());
                        }
                    }
                    if ui
                        .selectable_label(false, crate::i18n::widget(tank_name(&row.tank)))
                        .on_hover_text(crate::i18n::widget(&row.tank))
                        .clicked()
                    {
                        prefs.filter.tank = row.tank.clone();
                        prefs.view = 2;
                    }
                    money_label(ui, row.summary.economy.net.average());
                    money_label(ui, row.summary.economy.without_missions.average());
                    ui.label(crate::i18n::widget(vehicles::class_label(&row.class)));
                    ui.label(crate::i18n::widget(vehicles::nation_label(
                        vehicles::nation(&row.tank),
                    )));
                    ui.label(crate::i18n::widget(vehicles::role_label(&row.role)));
                    ui.label(crate::i18n::widget(row.tier.to_string()));
                    ui.label(crate::i18n::widget(row.summary.count.to_string()));
                    ui.label(crate::i18n::widget(row.summary.known_results.to_string()));
                    ui.colored_label(
                        wr_color(row.summary.winrate()),
                        egui::RichText::from(crate::i18n::rich(value(row.summary.winrate(), " %")))
                            .strong(),
                    );
                    for n in [
                        row.summary.avg_damage(),
                        row.summary.avg_assist(),
                        row.summary.avg_kills(),
                        row.summary.avg_xp(),
                    ] {
                        ui.label(crate::i18n::widget(value(n, "")));
                    }
                    ui.end_row();
                }
            });
    });
}
pub(super) fn accuracy_tanks(
    ui: &mut egui::Ui,
    battles: &[&Battle],
    prefs: &mut Preferences,
    sorting: &mut Sorting,
) {
    ui.weak(crate::i18n::rich("Direkte Gegnertreffer gemäß WoT. Summen und Durchschnitte nutzen nur Endergebnisse mit allen drei Zählern."));
    ui.weak(crate::i18n::rich("Trefferquote = Treffer / Schüsse. Durchschläge/Treffer und Durchschläge/Schüsse werden getrennt gezeigt. Kein Mittelwert einzelner Gefechtsquoten."));
    let mut rows = analysis::tanks(battles);
    rows.sort_by(|a, b| {
        let order = if sorting.column == Sort::Name {
            tank_name(&a.tank).cmp(&tank_name(&b.tank))
        } else {
            tank_metric(a, sorting.column)
                .partial_cmp(&tank_metric(b, sorting.column))
                .unwrap_or(std::cmp::Ordering::Equal)
        };
        (if sorting.descending {
            order.reverse()
        } else {
            order
        })
        .then(a.tank.cmp(&b.tank))
    });
    egui::ScrollArea::both()
        .id_salt("tank_accuracy")
        .show(ui, |ui| {
            egui::Grid::new("tank_accuracy_table")
                .striped(true)
                .spacing([16., 7.])
                .show(ui, |ui| {
                    ui.label(crate::i18n::widget("★"));
                    sorting.heading(ui, "Panzer", Sort::Name);
                    ui.strong(crate::i18n::rich("Ausgewertet"));
                    sorting.heading(ui, "Schüsse Σ", Sort::Shots);
                    sorting.heading(ui, "Treffer Σ", Sort::Hits);
                    sorting.heading(ui, "Durchschläge Σ", Sort::Penetrations);
                    sorting.heading(ui, "Trefferquote", Sort::HitRate);
                    sorting.heading(ui, "Durchschläge/Treffer", Sort::PenetrationRate);
                    sorting.heading(ui, "Durchschläge/Schüsse", Sort::PenetrationPerShot);
                    for label in ["Ø Schüsse", "Ø Treffer", "Ø Durchschläge"] {
                        ui.strong(crate::i18n::rich(label));
                    }
                    ui.end_row();
                    for row in rows {
                        let favorite = prefs.favorites.contains(&row.tank);
                        if ui
                            .selectable_label(
                                favorite,
                                crate::i18n::widget(if favorite { "★" } else { "☆" }),
                            )
                            .clicked()
                        {
                            if favorite {
                                prefs.favorites.remove(&row.tank);
                            } else {
                                prefs.favorites.insert(row.tank.clone());
                            }
                        }
                        if ui
                            .selectable_label(false, crate::i18n::widget(tank_name(&row.tank)))
                            .on_hover_text(crate::i18n::widget("Gefechte öffnen"))
                            .clicked()
                        {
                            prefs.filter.tank = row.tank.clone();
                            prefs.view = 2;
                        }
                        let a = &row.summary.accuracy;
                        ui.label(wot_insights::tr_format!(
                            "{} / {}",
                            a.count,
                            row.summary.known_results
                        ))
                        .on_hover_text(crate::i18n::widget(
                            "Vollständige Schussdaten / bekannte Endergebnisse",
                        ));
                        for n in [a.shots, a.hits, a.penetrations] {
                            ui.label(crate::i18n::widget(integer((a.count > 0).then_some(n))));
                        }
                        for rate in [a.hit_rate(), a.penetration_rate(), a.penetration_per_shot()] {
                            ui.label(crate::i18n::widget(value(rate, " %")));
                        }
                        for average in [
                            a.average_shots(),
                            a.average_hits(),
                            a.average_penetrations(),
                        ] {
                            ui.label(crate::i18n::widget(value(average, "")));
                        }
                        ui.end_row();
                    }
                });
        });
    ui.weak(crate::i18n::rich("Sprengwirkungstreffer sind separat in den Gefechtsdetails zu sehen. Fehlende Werte und Quoten ohne Nenner bleiben „–“. Mehrfachtreffer können Zählerverhältnisse über 100 % erzeugen."));
}
