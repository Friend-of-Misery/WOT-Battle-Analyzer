use super::*;
pub(super) fn cards(ui: &mut egui::Ui, total: &Summary) {
    ui.horizontal_wrapped(|ui| {
        for (label, number) in [
            ("Gefechte", total.count.to_string()),
            ("Winrate", value(total.winrate(), " %")),
            ("Ø Schaden", value(total.avg_damage(), "")),
            ("Ø Assist", value(total.avg_assist(), "")),
            ("Ø Abschüsse", value(total.avg_kills(), "")),
            ("Ø Basis-XP", value(total.avg_xp(), "")),
        ] {
            egui::Frame::group(ui.style())
                .inner_margin(12.)
                .show(ui, |ui| {
                    ui.set_min_width(125.);
                    ui.label(crate::i18n::widget(
                        RichText::new(crate::i18n::text(label)).color(MUTED),
                    ));
                    ui.label(crate::i18n::widget(
                        RichText::new(crate::i18n::text(number)).size(26.).strong(),
                    ));
                });
        }
    });
}
pub(super) fn overview(
    ui: &mut egui::Ui,
    battles: &[&Battle],
    summary: &Summary,
    selected: &mut Option<String>,
) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        cards(ui, summary);
        let a = &summary.accuracy;
        ui.horizontal_wrapped(|ui| {
            ui.label(wot_insights::tr_format!(
                "Schussdaten aus {} Gefechten:",
                a.count
            ));
            ui.strong(wot_insights::tr_format!(
                "{} Schüsse · {} Treffer · {} Durchschläge",
                integer((a.count > 0).then_some(a.shots)),
                integer((a.count > 0).then_some(a.hits)),
                integer((a.count > 0).then_some(a.penetrations))
            ));
            ui.label(wot_insights::tr_format!(
                "Trefferquote {} · Durchschläge/Treffer {}",
                value(a.hit_rate(), " %"),
                value(a.penetration_rate(), " %")
            ));
        });
        ui.add_space(10.);
        ui.columns(3, |columns| {
            gauge(
                &mut columns[0],
                "Siegquote",
                summary.winrate(),
                (30., 70.),
                "%",
                wr_color(summary.winrate()),
            );
            gauge(
                &mut columns[1],
                "Schaden / erhalten",
                summary.damage_ratio(),
                (0., 2.),
                "",
                ACCENT,
            );
            gauge(
                &mut columns[2],
                "Abschüsse pro Gefecht",
                summary.avg_kills(),
                (0., 3.),
                "",
                GREEN,
            );
        });
        ui.add_space(12.);
        ui.columns(2, |columns| {
            let mut classes = BTreeMap::<String, usize>::new();
            let mut tiers = BTreeMap::<String, usize>::new();
            for b in battles {
                *classes
                    .entry(vehicles::class_label(&b.class).into())
                    .or_default() += 1;
                *tiers
                    .entry(wot_insights::tr_format!("Stufe {}", b.tier))
                    .or_default() += 1;
            }
            bars(&mut columns[0], "Gefechte nach Klasse", &classes, ACCENT);
            bars(&mut columns[1], "Gefechte nach Stufe", &tiers, GREEN);
        });
        ui.add_space(12.);
        ui.heading(crate::i18n::rich("Letzte Spieltage"));
        egui::Grid::new("recent_days")
            .striped(true)
            .spacing([22., 7.])
            .show(ui, |ui| {
                for h in [
                    "Tag",
                    "Gefechte",
                    "Endergebnisse",
                    "Winrate",
                    "Ø Schaden",
                    "Ø Assist",
                ] {
                    ui.strong(crate::i18n::rich(h));
                }
                ui.end_row();
                for (date, s) in analysis::days(battles).iter().rev().take(7) {
                    ui.label(date);
                    ui.label(crate::i18n::widget(s.count.to_string()));
                    ui.label(crate::i18n::widget(s.known_results.to_string()));
                    ui.colored_label(
                        wr_color(s.winrate()),
                        egui::RichText::from(crate::i18n::rich(value(s.winrate(), " %"))).strong(),
                    );
                    ui.label(crate::i18n::widget(value(s.avg_damage(), "")));
                    ui.label(crate::i18n::widget(value(s.avg_assist(), "")));
                    ui.end_row();
                }
            });
        ui.add_space(12.);
        ui.heading(crate::i18n::rich("Zuletzt gespielt"));
        for b in battles.iter().take(5) {
            if ui
                .selectable_label(
                    false,
                    crate::i18n::widget(wot_insights::tr_format!(
                        "{}   {}   {}   {}   {} Schaden",
                        b.date,
                        tank_name(&b.tank),
                        b.map,
                        result_label(b),
                        integer(b.damage)
                    )),
                )
                .clicked()
            {
                *selected = Some(b.key.clone());
            }
        }
    });
}
pub(super) fn gauge(
    ui: &mut egui::Ui,
    label: &str,
    number: Option<f64>,
    range: (f64, f64),
    suffix: &str,
    color: Color32,
) {
    let size = Vec2::new(ui.available_width().clamp(120., 380.), 172.);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let center = egui::pos2(rect.center().x, rect.bottom() - 35.);
    let radius = (rect.width() / 2. - 23.).clamp(35., 98.);
    let point = |a: f32, r: f32| center + Vec2::new(a.cos(), a.sin()) * r;
    for index in 0..40 {
        let a = std::f32::consts::PI + std::f32::consts::PI * index as f32 / 40.;
        let b = std::f32::consts::PI + std::f32::consts::PI * (index + 1) as f32 / 40.;
        painter.line_segment(
            [point(a, radius), point(b, radius)],
            Stroke::new(5_f32, Color32::from_gray(65)),
        );
    }
    for index in 0..=8 {
        let a = std::f32::consts::PI + std::f32::consts::PI * index as f32 / 8.;
        painter.line_segment(
            [point(a, radius - 4.), point(a, radius - 10.)],
            Stroke::new(1_f32, MUTED),
        );
    }
    if let Some(number) = number {
        let fraction = ((number - range.0) / (range.1 - range.0)).clamp(0., 1.) as f32;
        let a = std::f32::consts::PI + std::f32::consts::PI * fraction;
        painter.line_segment(
            [center, point(a, radius - 17.)],
            Stroke::new(2.5_f32, color),
        );
        painter.circle_filled(center, 4., color);
    }
    painter.text(
        center + Vec2::new(0., -26.),
        egui::Align2::CENTER_CENTER,
        value(number, suffix),
        egui::FontId::proportional(23.),
        color,
    );
    painter.text(
        center + Vec2::new(-radius, 17.),
        egui::Align2::CENTER_CENTER,
        wot_insights::tr_format!("{}", range.0),
        egui::TextStyle::Small.resolve(ui.style()),
        MUTED,
    );
    painter.text(
        center + Vec2::new(radius, 17.),
        egui::Align2::CENTER_CENTER,
        wot_insights::tr_format!("{}", range.1),
        egui::TextStyle::Small.resolve(ui.style()),
        MUTED,
    );
    painter.text(
        egui::pos2(rect.center().x, rect.bottom() - 5.),
        egui::Align2::CENTER_BOTTOM,
        label,
        egui::TextStyle::Body.resolve(ui.style()),
        Color32::WHITE,
    );
}
pub(super) fn bars(ui: &mut egui::Ui, title: &str, rows: &BTreeMap<String, usize>, color: Color32) {
    ui.strong(crate::i18n::rich(title));
    let maximum = rows.values().copied().max().unwrap_or(1).max(1) as f32;
    // Keep numeric tier ordering rather than lexicographic order (10 before 2).
    let mut rows: Vec<_> = rows.iter().collect();
    rows.sort_by_key(|(label, _)| {
        label
            .strip_prefix("Stufe ")
            .and_then(|s| s.parse::<u8>().ok())
            .unwrap_or(0)
    });
    for (label, count) in rows {
        ui.horizontal(|ui| {
            ui.add_sized([88., 18.], egui::Label::new(crate::i18n::widget(label)));
            ui.add(
                egui::ProgressBar::new(*count as f32 / maximum)
                    .fill(color)
                    .desired_width((ui.available_width() - 50.).max(50.))
                    .text(count.to_string()),
            );
        });
    }
}
