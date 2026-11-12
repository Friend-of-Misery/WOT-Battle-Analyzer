use super::*;
pub(super) fn result_label(b: &Battle) -> &'static str {
    match b.result.as_deref() {
        Some("win") => "Sieg",
        Some("loss") => "Niederlage",
        Some("draw") => "Unentschieden",
        _ => "Offen",
    }
}
pub(super) fn battle_metric(b: &Battle, column: Sort) -> Option<f64> {
    match column {
        Sort::HitRate => return wot_insights::accuracy::percentage(b.hits, b.shots),
        Sort::PenetrationRate => return wot_insights::accuracy::percentage(b.penetrations, b.hits),
        Sort::PenetrationPerShot => {
            return wot_insights::accuracy::percentage(b.penetrations, b.shots);
        }
        _ => {}
    }
    match column {
        Sort::Damage => b.damage,
        Sort::Assist => b.assist,
        Sort::Kills => b.kills,
        Sort::Xp => b.base_xp,
        Sort::Credits => b.credits,
        Sort::Net => b.economy.as_ref().and_then(|e| e.net()),
        Sort::Shots => b.shots,
        Sort::Hits => b.hits,
        Sort::Penetrations => b.penetrations,
        _ => None,
    }
    .map(|v| v as f64)
}
pub(super) fn battle_table(
    ui: &mut egui::Ui,
    battles: &[&Battle],
    columns: &mut Columns,
    sorting: &mut Sorting,
    page: &mut usize,
    selected: &mut Option<String>,
) {
    let mut rows = battles.to_vec();
    rows.sort_by(|a, b| {
        let order = match sorting.column {
            Sort::Date => a.date.cmp(&b.date),
            Sort::Name => tank_name(&a.tank).cmp(&tank_name(&b.tank)),
            _ => battle_metric(a, sorting.column)
                .partial_cmp(&battle_metric(b, sorting.column))
                .unwrap_or(std::cmp::Ordering::Equal),
        };
        (if sorting.descending {
            order.reverse()
        } else {
            order
        })
        .then(a.key.cmp(&b.key))
    });
    let pages = rows.len().div_ceil(100).max(1);
    *page = (*page).min(pages - 1);
    ui.horizontal_wrapped(|ui| {
        ui.menu_button(crate::i18n::widget("Spalten"), |ui| {
            ui.checkbox(&mut columns.assist, crate::i18n::widget("Assist"));
            ui.checkbox(&mut columns.kills, crate::i18n::widget("Abschüsse"));
            ui.checkbox(&mut columns.xp, crate::i18n::widget("Basis-XP"));
            ui.checkbox(&mut columns.credits, crate::i18n::widget("Credits brutto"));
            ui.checkbox(&mut columns.net, crate::i18n::widget("Credits netto"));
            ui.checkbox(
                &mut columns.shot_details,
                crate::i18n::widget("Schüsse / Treffer / Durchschläge"),
            );
            ui.checkbox(
                &mut columns.received,
                crate::i18n::widget("Erhaltener Schaden"),
            );
        });
        if ui
            .add_enabled(
                *page > 0,
                egui::Button::new(crate::i18n::widget("‹ Zurück")),
            )
            .clicked()
        {
            *page -= 1;
        }
        ui.label(wot_insights::tr_format!(
            "Seite {} / {} · {} Gefechte",
            *page + 1,
            pages,
            rows.len()
        ));
        if ui
            .add_enabled(
                *page + 1 < pages,
                egui::Button::new(crate::i18n::widget("Weiter ›")),
            )
            .clicked()
        {
            *page += 1;
        }
        ui.weak(crate::i18n::rich("Gefechtsdatum anklicken für Details."));
    });
    egui::ScrollArea::both().show(ui, |ui| {
        egui::Grid::new("battle_table")
            .striped(true)
            .spacing([14., 7.])
            .show(ui, |ui| {
                sorting.heading(ui, "Datum", Sort::Date);
                sorting.heading(ui, "Panzer", Sort::Name);
                ui.strong(crate::i18n::rich("Karte"));
                ui.strong(crate::i18n::rich("Ergebnis"));
                sorting.heading(ui, "Schaden", Sort::Damage);
                if columns.assist {
                    sorting.heading(ui, "Assist", Sort::Assist);
                }
                if columns.kills {
                    sorting.heading(ui, "Abschüsse", Sort::Kills);
                }
                if columns.xp {
                    sorting.heading(ui, "Basis-XP", Sort::Xp);
                }
                if columns.credits {
                    sorting.heading(ui, "Credits brutto", Sort::Credits);
                }
                if columns.net {
                    sorting.heading(ui, "Netto", Sort::Net);
                }
                if columns.shot_details {
                    sorting.heading(ui, "Schüsse", Sort::Shots);
                    sorting.heading(ui, "Treffer", Sort::Hits);
                    sorting.heading(ui, "Durchschläge", Sort::Penetrations);
                    sorting.heading(ui, "Trefferquote", Sort::HitRate);
                    sorting.heading(ui, "Durchschlag/Treffer", Sort::PenetrationRate);
                }
                if columns.received {
                    ui.strong(crate::i18n::rich("Erhalten"));
                }
                ui.end_row();
                for b in rows.iter().skip(*page * 100).take(100) {
                    if ui
                        .selectable_label(
                            selected.as_ref() == Some(&b.key),
                            crate::i18n::widget(&b.date),
                        )
                        .clicked()
                    {
                        *selected = Some(b.key.clone());
                    }
                    ui.label(crate::i18n::widget(tank_name(&b.tank)))
                        .on_hover_text(crate::i18n::widget(&b.tank));
                    ui.label(crate::i18n::widget(&b.map));
                    ui.colored_label(
                        match b.result.as_deref() {
                            Some("win") => GREEN,
                            Some("loss") => RED,
                            _ => MUTED,
                        },
                        egui::RichText::from(crate::i18n::rich(result_label(b))).strong(),
                    );
                    ui.label(crate::i18n::widget(integer(b.damage)));
                    if columns.assist {
                        ui.label(crate::i18n::widget(integer(b.assist)));
                    }
                    if columns.kills {
                        ui.label(crate::i18n::widget(integer(b.kills)));
                    }
                    if columns.xp {
                        ui.label(crate::i18n::widget(integer(b.base_xp)));
                    }
                    if columns.credits {
                        ui.label(crate::i18n::widget(integer(b.credits)));
                    }
                    if columns.net {
                        money_label(
                            ui,
                            b.economy.as_ref().and_then(|e| e.net()).map(|n| n as f64),
                        );
                    }
                    if columns.shot_details {
                        ui.label(crate::i18n::widget(integer(b.shots)));
                        ui.label(crate::i18n::widget(integer(b.hits)));
                        ui.label(crate::i18n::widget(integer(b.penetrations)));
                        ui.label(crate::i18n::widget(value(
                            wot_insights::accuracy::percentage(b.hits, b.shots),
                            " %",
                        )));
                        ui.label(crate::i18n::widget(value(
                            wot_insights::accuracy::percentage(b.penetrations, b.hits),
                            " %",
                        )));
                    }
                    if columns.received {
                        ui.label(crate::i18n::widget(integer(b.received)));
                    }
                    ui.end_row();
                }
            });
    });
}
pub(super) fn battle_details(ui: &mut egui::Ui, b: &Battle) {
    ui.heading(crate::i18n::rich(tank_name(&b.tank)));
    ui.label(wot_insights::tr_format!(
        "{} · {} · {}",
        b.map,
        b.date,
        b.player
    ));
    ui.separator();
    egui::Grid::new("battle_details")
        .num_columns(2)
        .spacing([22., 7.])
        .show(ui, |ui| {
            for (label, text) in [
                ("Ergebnis", result_label(b).into()),
                ("Klasse", vehicles::class_label(&b.class).into()),
                (
                    "Nation",
                    vehicles::nation_label(vehicles::nation(&b.tank)).into(),
                ),
                ("Rolle", vehicles::role_label(&b.role).into()),
                ("Stufe", b.tier.to_string()),
                ("Schaden", integer(b.damage)),
                ("Assist", integer(b.assist)),
                ("Abschüsse", integer(b.kills)),
                ("Basis-XP", integer(b.base_xp)),
                ("Credits brutto", integer(b.credits)),
                ("Schüsse", integer(b.shots)),
                ("Direkte Gegnertreffer", integer(b.hits)),
                ("Durchschläge", integer(b.penetrations)),
                (
                    "Trefferquote (Treffer / Schüsse)",
                    value(wot_insights::accuracy::percentage(b.hits, b.shots), " %"),
                ),
                (
                    "Durchschläge / Treffer",
                    value(
                        wot_insights::accuracy::percentage(b.penetrations, b.hits),
                        " %",
                    ),
                ),
                (
                    "Durchschläge / Schüsse",
                    value(
                        wot_insights::accuracy::percentage(b.penetrations, b.shots),
                        " %",
                    ),
                ),
                ("Sprengwirkungstreffer (separat)", integer(b.explosion_hits)),
                ("Erhaltener Schaden", integer(b.received)),
            ] {
                ui.weak(crate::i18n::rich(label));
                ui.label(crate::i18n::widget(text));
                ui.end_row();
            }
        });
    if b.result.is_none() {
        ui.add_space(8.);
        ui.colored_label(
            egui::Color32::YELLOW,
            egui::RichText::from(crate::i18n::rich(
                "Endergebnis fehlt. Dieses Gefecht zählt nicht in Winrate und Durchschnitte.",
            ))
            .strong(),
        );
    }
    ui.add_space(8.);
    if let Some(e) = &b.economy {
        egui::CollapsingHeader::new(crate::i18n::widget("Credits-Abrechnung")).default_open(true).show(ui,|ui|{
            egui::Grid::new("credit_invoice").num_columns(2).spacing([18.,5.]).show(ui,|ui|{
                for (label,amount) in [("Basisverdienst laut Replay",e.base),("Premiumkonto-Bonus",e.premium),("Persönliche Reserven",e.personal_reserve),("Clan-/Kampfreserven",e.clan_reserve),("Aufgaben / Ereignisse",e.missions),("Sonstige Boni / Korrekturen",e.other),("Brutto insgesamt",e.gross)]{
                    ui.label(crate::i18n::widget(label));ui.label(crate::i18n::widget(money(amount.map(|n|n as f64))));ui.end_row();
                }
                ui.separator();ui.separator();ui.end_row();
                for (label,cost) in [("Reparatur",e.repair),("Munition",e.ammunition),("Verbrauchsmaterial",e.consumables),("Ausgaben insgesamt",e.expenses())]{ui.label(crate::i18n::widget(label));ui.label(crate::i18n::widget(money(cost.map(|n|-n as f64))));ui.end_row();}
                ui.separator();ui.separator();ui.end_row();
                for (label,n) in [("Netto",e.net()),("Netto ohne Aufgaben / Ereignisse",e.without_missions()),("Netto ohne Aufgaben und Reserven",e.without_missions_and_reserves())]{ui.strong(crate::i18n::rich(label));money_label(ui,n.map(|n|n as f64));ui.end_row();}
            });
            if e.net().is_none(){ui.colored_label(egui::Color32::YELLOW,egui::RichText::from(crate::i18n::rich("Abrechnung unvollständig: fehlende Kosten werden nicht mit 0 angesetzt.")).strong());}
            if e.gold_expenses.is_some_and(|n|n>0)||e.bond_expenses.is_some_and(|n|n>0){ui.label(wot_insights::tr_format!("Separat: {} Gold, {} Anleihen ausgegeben; nicht in Credits umgerechnet.",integer(e.gold_expenses),integer(e.bond_expenses)));}
            ui.weak(crate::i18n::rich("Aufgaben und Boni sind bereits im Brutto enthalten. Abzüge: im Replay gemeldete automatische Reparatur und Wiederauffüllung. Spätere manuelle Käufe und Lagerbestände sind keine vollständige Geldbeutelbilanz."));
        });
    } else {
        ui.weak(crate::i18n::rich(
            "Für die Credits-Abrechnung dieses Gefechts fehlen Replay-Daten.",
        ));
    }
    ui.weak(crate::i18n::rich("Basisverdienst kann Fahrzeugfaktoren und Gefechtsauszeichnungen enthalten. Nicht erklärte Differenzen bleiben als sonstige Boni/Korrekturen sichtbar."));
}
