use super::*;
pub(super) fn money(amount: Option<f64>) -> String {
    let Some(amount) = amount else {
        return "–".into();
    };
    let raw = wot_insights::tr_format!("{:.0}", amount.abs());
    let grouped = raw
        .chars()
        .rev()
        .enumerate()
        .flat_map(|(i, c)| {
            if i > 0 && i % 3 == 0 {
                vec!['.', c]
            } else {
                vec![c]
            }
        })
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    wot_insights::tr_format!("{}{grouped}", if amount < 0. { "−" } else { "" })
}
pub(super) fn money_label(ui: &mut egui::Ui, amount: Option<f64>) {
    ui.colored_label(
        match amount {
            Some(n) if n >= 0. => GREEN,
            Some(_) => RED,
            None => MUTED,
        },
        egui::RichText::from(crate::i18n::rich(money(amount))).strong(),
    );
}
pub(super) fn profit_metric(
    s: &wot_insights::economy::EconomySummary,
    basis: usize,
) -> &wot_insights::economy::MoneyMetric {
    match basis {
        0 => &s.net,
        1 => &s.without_missions,
        _ => &s.without_reserves,
    }
}
pub(super) fn finance_amount(ui: &mut egui::Ui, amount: Option<f64>, large: bool) {
    let color = match amount {
        Some(n) if n < 0. => Color32::from_rgb(180, 25, 25),
        Some(_) => Color32::BLACK,
        None => Color32::from_gray(90),
    };
    ui.label(crate::i18n::widget(
        RichText::new(crate::i18n::text(money(amount)))
            .color(color)
            .size(if large {
                25.
            } else {
                ui.style().text_styles[&egui::TextStyle::Body].size
            })
            .strong(),
    ));
}
pub(super) fn finance_component(
    ui: &mut egui::Ui,
    label: &str,
    metric: &wot_insights::economy::MoneyMetric,
    expense: bool,
) {
    ui.horizontal(|ui| {
        ui.label(crate::i18n::widget(label));
        finance_amount(
            ui,
            metric
                .total()
                .map(|n| if expense { -(n as f64) } else { n as f64 }),
            false,
        );
        if metric.count > 0 {
            ui.label(crate::i18n::widget(
                RichText::new(wot_insights::tr_format!(
                    "({} Gefechte)",
                    money(Some(metric.count as f64))
                ))
                .small()
                .color(Color32::from_gray(85)),
            ));
        }
    });
}
pub(super) fn finance_view(
    ui: &mut egui::Ui,
    battles: &[&Battle],
    total: &Summary,
    prefs: &mut Preferences,
    sorting: &mut Sorting,
) {
    let e = &total.economy;
    ui.label(wot_insights::tr_format!("Alle Beträge in Credits · {} einberechnete Gefechte · {} Endergebnisse im aktuellen Filter", money(Some(e.net.count as f64)), money(Some(total.known_results as f64))));
    egui::Frame::new()
        .fill(Color32::from_rgb(245, 246, 248))
        .stroke(Stroke::new(1_f32, Color32::from_gray(190)))
        .corner_radius(6.)
        .inner_margin(12.)
        .show(ui, |ui| {
            ui.scope(|ui| {
                ui.visuals_mut().override_text_color = Some(Color32::BLACK);
                let detail_width = (ui.available_width() - 185. - 150. - 56.).max(260.);
                egui::Grid::new("finance_summary")
                    .num_columns(3)
                    .spacing([28., 14.])
                    .show(ui, |ui| {
                        ui.strong(crate::i18n::rich("Gesamt"));
                        ui.strong(crate::i18n::rich("Ø pro Gefecht"));
                        ui.strong(crate::i18n::rich("Enthaltene Boni / Kosten · gesamt"));
                        ui.end_row();
                        for (row, label, metric) in [
                            (0, "Einnahmen", &e.gross),
                            (1, "Ausgaben", &e.expenses),
                            (2, "Bilanz", &e.net),
                        ] {
                            let sign = if row == 1 { -1. } else { 1. };
                            ui.vertical(|ui| {
                                ui.set_min_width(185.);
                                ui.strong(crate::i18n::rich(label));
                                finance_amount(ui, metric.total().map(|n| n as f64 * sign), true);
                            });
                            ui.vertical(|ui| {
                                ui.set_min_width(150.);
                                ui.label(crate::i18n::widget(label));
                                finance_amount(ui, metric.average().map(|n| n * sign), true);
                            });
                            ui.vertical(|ui| {
                                ui.set_width(detail_width);
                                match row {
                                0 => {
                                    finance_component(ui, "Persönliche Booster", &e.personal_reserve, false);
                                    finance_component(ui, "Clan-/Kampfreserven", &e.clan_reserve, false);
                                    finance_component(ui, "Tagesaufgaben / Missionen / Ereignisse", &e.missions, false);
                                    finance_component(ui, "Premiumbonus", &e.premium, false);
                                }
                                1 => {
                                    finance_component(ui, "Munition", &e.ammunition, true);
                                    finance_component(ui, "Fahrzeugreparatur", &e.repair, true);
                                    finance_component(ui, "Verbrauchsmaterial", &e.consumables, true);
                                    ui.label(crate::i18n::widget(RichText::new(crate::i18n::text("Food, Medikit, Reparaturset, Feuerlöscher: Kosten nur als Gesamtsumme bekannt.")).small())).on_hover_text(crate::i18n::widget("Food = Verpflegung für die Besatzung; Medikit = Erste-Hilfe-Kasten; Reparaturset = Reparatur von Modulen im Gefecht. Fahrzeugreparatur ist die separate Reparatur nach dem Gefecht. autoEquipCost enthält keine Kosten je Einzelartikel."));
                                }
                                _ => {
                                    ui.label(crate::i18n::widget("Einnahmen − Ausgaben"));
                                    ui.label(crate::i18n::widget("Boni sind bereits in den Einnahmen enthalten."));
                                }
                                }
                            });
                            ui.end_row();
                        }
                    });
            });
        });
    ui.weak(crate::i18n::rich("Boni können eine abweichende Zahl bekannter Gefechte haben. Aufgabencredits werden vom Replay gemeinsam gemeldet; Tagesaufgaben und andere Missionen sind hier nicht getrennt."));
    let old_basis = prefs.finance_basis;
    ui.horizontal_wrapped(|ui| {
        ui.selectable_value(
            &mut prefs.finance_mode,
            0,
            crate::i18n::widget("Panzer-Rangliste"),
        );
        ui.selectable_value(
            &mut prefs.finance_mode,
            1,
            crate::i18n::widget("Einnahmen & Kosten"),
        );
        ui.separator();
        ui.label(crate::i18n::widget("Vergleichen nach"));
        egui::ComboBox::from_id_salt("finance_basis")
            .selected_text(crate::i18n::widget(
                ["Ø Netto", "Ø ohne Aufgaben", "Ø ohne Aufgaben/Reserven"][prefs.finance_basis],
            ))
            .show_ui(ui, |ui| {
                for (basis, label) in ["Ø Netto", "Ø ohne Aufgaben", "Ø ohne Aufgaben/Reserven"]
                    .iter()
                    .enumerate()
                {
                    ui.selectable_value(
                        &mut prefs.finance_basis,
                        basis,
                        crate::i18n::widget(*label),
                    );
                }
            });
        ui.label(crate::i18n::widget("Mind. Abrechnungen"));
        ui.add(egui::DragValue::new(&mut prefs.finance_minimum).range(1..=500));
    });
    if prefs.finance_basis != old_basis {
        sorting.column = [Sort::Net, Sort::CleanNet, Sort::UnboostedNet][prefs.finance_basis];
        sorting.descending = true;
    }
    let mut rows = analysis::tanks(battles);
    rows.retain(|r| {
        profit_metric(&r.summary.economy, prefs.finance_basis).count >= prefs.finance_minimum
    });
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
    if rows.is_empty() {
        ui.label(crate::i18n::widget("Noch kein Panzer erreicht die Mindestzahl vollständiger Abrechnungen. Minimum senken oder Filter erweitern."));
        return;
    }
    egui::ScrollArea::both().id_salt("finance_rows").show(ui,|ui|{egui::Grid::new("finance_table").striped(true).spacing([16.,7.]).show(ui,|ui|{
        sorting.heading(ui,"Panzer",Sort::Name);ui.strong(crate::i18n::rich("Abrechnungen"));
        sorting.heading(ui,"Ø Netto",Sort::Net);sorting.heading(ui,"Ø ohne Aufgaben",Sort::CleanNet);sorting.heading(ui,"Ø ohne Aufgaben/Reserven",Sort::UnboostedNet);
        if prefs.finance_mode==0{
            sorting.heading(ui,"Netto gesamt",Sort::TotalNet);sorting.heading(ui,"Ø Ausgaben",Sort::Expenses);ui.strong(crate::i18n::rich("Ø Munition"));ui.strong(crate::i18n::rich("Ø Reparatur"));ui.strong(crate::i18n::rich("Ø Verbrauch"));
        }else{for h in ["Ø Basis","Ø Premium","Ø persönliche Reserven","Ø Clanreserven","Ø Aufgaben","Ø Sonstiges","Ø Brutto","Ø Munition","Ø Reparatur","Ø Verbrauch"]{ui.strong(crate::i18n::rich(h));}}
        ui.end_row();
        for row in rows{
            let e=&row.summary.economy;
            if ui.selectable_label(false,crate::i18n::widget(tank_name(&row.tank))).on_hover_text(crate::i18n::widget("Gefechte und einzelne Abrechnungen öffnen")).clicked(){prefs.filter.tank=row.tank.clone();prefs.view=2;}
            ui.label(wot_insights::tr_format!("{} / {}",profit_metric(e,prefs.finance_basis).count,row.summary.known_results)).on_hover_text(crate::i18n::widget("Für die gewählte Vergleichskennzahl bekannte Abrechnungen / Endergebnisse"));
            money_label(ui,e.net.average());money_label(ui,e.without_missions.average());money_label(ui,e.without_reserves.average());
            if prefs.finance_mode==0{money_label(ui,e.net.total().map(|n|n as f64));for n in [e.expenses.average(),e.ammunition.average(),e.repair.average(),e.consumables.average()]{ui.label(crate::i18n::widget(money(n)));}}
            else{for metric in [&e.base,&e.premium,&e.personal_reserve,&e.clan_reserve,&e.missions,&e.other,&e.gross,&e.ammunition,&e.repair,&e.consumables]{ui.label(crate::i18n::widget(money(metric.average()))).on_hover_text(wot_insights::tr_format!("{} bekannte Werte",metric.count));}}
            ui.end_row();
        }
    });});
}
