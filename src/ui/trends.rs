use super::*;
pub(super) fn trend_value(summary: &Summary, metric: usize) -> Option<f64> {
    match metric {
        0 => summary.winrate(),
        1 => summary.avg_damage(),
        2 => summary.avg_assist(),
        3 => summary.avg_xp(),
        _ => Some(summary.count as f64),
    }
}
pub(super) fn trend_view(ui: &mut egui::Ui, battles: &[&Battle], metric: &mut usize) {
    let labels = ["Winrate", "Ø Schaden", "Ø Assist", "Ø Basis-XP", "Gefechte"];
    ui.horizontal_wrapped(|ui| {
        ui.label(crate::i18n::widget("Tageswerte"));
        for (index, label) in labels.iter().enumerate() {
            ui.selectable_value(metric, index, crate::i18n::widget(*label));
        }
    });
    let days = analysis::days(battles);
    let points: Vec<_> = days
        .iter()
        .filter_map(|(day, s)| trend_value(s, *metric).map(|v| (day, v, s.known_results, s.count)))
        .collect();
    if points.is_empty() {
        ui.label(crate::i18n::widget(
            "Für diese Kennzahl fehlen Endergebnisse im gewählten Zeitraum.",
        ));
        return;
    }
    let ceiling = if *metric == 0 {
        100.
    } else {
        points.iter().map(|p| p.1).fold(0., f64::max).max(1.) * 1.12
    };
    let height = (ui.available_height() * 0.60).clamp(180., 360.);
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), height),
        egui::Sense::hover(),
    );
    let plot = egui::Rect::from_min_max(
        rect.min + Vec2::new(62., 16.),
        rect.max - Vec2::new(24., 42.),
    );
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 5., Color32::from_rgb(23, 26, 31));
    for i in 0..=4 {
        let fraction = i as f32 / 4.;
        let y = plot.bottom() - plot.height() * fraction;
        painter.line_segment(
            [egui::pos2(plot.left(), y), egui::pos2(plot.right(), y)],
            Stroke::new(1_f32, Color32::from_gray(52)),
        );
        painter.text(
            egui::pos2(plot.left() - 8., y),
            egui::Align2::RIGHT_CENTER,
            wot_insights::tr_format!(
                "{:.0}{}",
                ceiling * fraction as f64,
                if *metric == 0 { "%" } else { "" }
            ),
            egui::TextStyle::Small.resolve(ui.style()),
            MUTED,
        );
    }
    let positions: Vec<_> = points
        .iter()
        .enumerate()
        .map(|(i, p)| {
            egui::pos2(
                if points.len() == 1 {
                    plot.center().x
                } else {
                    plot.left() + plot.width() * i as f32 / (points.len() - 1) as f32
                },
                plot.bottom() - plot.height() * (p.1 / ceiling) as f32,
            )
        })
        .collect();
    for pair in positions.windows(2) {
        painter.line_segment([pair[0], pair[1]], Stroke::new(2_f32, ACCENT));
    }
    for position in &positions {
        painter.circle_filled(*position, 3., ACCENT);
    }
    let stride = points.len().div_ceil(6).max(1);
    for (index, p) in points
        .iter()
        .enumerate()
        .filter(|(index, _)| index % stride == 0)
    {
        painter.text(
            egui::pos2(positions[index].x, plot.bottom() + 16.),
            egui::Align2::CENTER_CENTER,
            &p.0[5..],
            egui::TextStyle::Small.resolve(ui.style()),
            MUTED,
        );
    }
    if let Some(pointer) = response.hover_pos()
        && let Some((index, position)) = positions.iter().enumerate().min_by(|a, b| {
            a.1.distance_sq(pointer)
                .total_cmp(&b.1.distance_sq(pointer))
        })
        && position.distance(pointer) < 24.
    {
        painter.circle_stroke(*position, 7., Stroke::new(2_f32, Color32::WHITE));
        let (date, v, known, count) = points[index];
        response.on_hover_ui(|ui| {
            ui.strong(date);
            ui.label(wot_insights::tr_format!(
                "{}: {}",
                labels[*metric],
                value(Some(v), if *metric == 0 { " %" } else { "" })
            ));
            ui.label(wot_insights::tr_format!(
                "{count} Gefechte · {known} Endergebnisse"
            ));
        });
    }
    ui.weak(crate::i18n::rich("Jeder Punkt ist ein Spieltag; Tage ohne Daten werden ausgelassen. Maus über einen Punkt für die Werte."));
    ui.add_space(8.);
    egui::ScrollArea::vertical()
        .id_salt("trend_days")
        .show(ui, |ui| {
            egui::Grid::new("trend_table")
                .striped(true)
                .spacing([24., 7.])
                .show(ui, |ui| {
                    for h in [
                        "Tag",
                        "Gefechte",
                        "Endergebnisse",
                        "Winrate",
                        "Ø Schaden",
                        "Ø Assist",
                        "Ø Basis-XP",
                    ] {
                        ui.strong(crate::i18n::rich(h));
                    }
                    ui.end_row();
                    for (day, s) in days.iter().rev() {
                        ui.label(day);
                        ui.label(crate::i18n::widget(s.count.to_string()));
                        ui.label(crate::i18n::widget(s.known_results.to_string()));
                        ui.colored_label(
                            wr_color(s.winrate()),
                            egui::RichText::from(crate::i18n::rich(value(s.winrate(), " %")))
                                .strong(),
                        );
                        ui.label(crate::i18n::widget(value(s.avg_damage(), "")));
                        ui.label(crate::i18n::widget(value(s.avg_assist(), "")));
                        ui.label(crate::i18n::widget(value(s.avg_xp(), "")));
                        ui.end_row();
                    }
                });
        });
}
