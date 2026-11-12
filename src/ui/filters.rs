use super::*;
pub(super) fn filters(ui: &mut egui::Ui, battles: &[Battle], prefs: &mut Preferences) {
    ui.horizontal_wrapped(|ui| {
        ui.label(crate::i18n::widget("Spieler"));
        let players: BTreeSet<_> = battles.iter().map(|b| b.player.clone()).collect();
        egui::ComboBox::from_id_salt("player")
            .width(155.)
            .selected_text(crate::i18n::widget(if prefs.filter.player.is_empty() {
                "Alle"
            } else {
                &prefs.filter.player
            }))
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut prefs.filter.player,
                    String::new(),
                    crate::i18n::widget("Alle"),
                );
                for p in players {
                    ui.selectable_value(&mut prefs.filter.player, p.clone(), p);
                }
            });
        egui::ComboBox::from_id_salt("period")
            .width(150.)
            .selected_text(crate::i18n::widget(period_label(prefs.filter.days)))
            .show_ui(ui, |ui| {
                for n in [-1, 1, 7, 30, 90, 0] {
                    ui.selectable_value(
                        &mut prefs.filter.days,
                        n,
                        crate::i18n::widget(period_label(n)),
                    );
                }
            });
        vehicle_filter_button(ui, &mut prefs.filter);
        ui.checkbox(
            &mut prefs.filter.favorites_only,
            crate::i18n::widget("★ Favoriten"),
        );
    });
    ui.horizontal_wrapped(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut prefs.filter.tank_search)
                .hint_text(crate::i18n::widget("Panzer suchen …"))
                .desired_width(160.),
        ).on_hover_text(crate::i18n::widget("Zum Beispiel T20, Strv 103 oder AMX. Groß-/Kleinschreibung und Leerzeichen spielen keine Rolle."));
        let tanks: BTreeSet<_> = battles
            .iter()
            .filter(|b| {
                b.mode == 1 && analysis::matches_vehicle(b, &prefs.filter)
                    && (prefs.filter.player.is_empty() || b.player == prefs.filter.player)
                    && (!prefs.filter.favorites_only || prefs.favorites.contains(&b.tank))
            })
            .map(|b| b.tank.clone())
            .collect();
        if !prefs.filter.tank.is_empty() && !tanks.contains(&prefs.filter.tank) {
            prefs.filter.tank.clear();
        }
        egui::ComboBox::from_id_salt("tank")
            .width(245.)
            .selected_text(crate::i18n::widget(if prefs.filter.tank.is_empty() {
                if prefs.filter.tank_search.trim().is_empty() { "Alle Panzer".into() } else { "Alle Suchtreffer".into() }
            } else {
                tank_name(&prefs.filter.tank)
            }))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut prefs.filter.tank, String::new(),crate::i18n::widget( "Alle Panzer"));
                if tanks.is_empty() { ui.weak(crate::i18n::rich("Keine passenden Panzer")); }
                for t in tanks {
                    let name = tank_name(&t);
                    ui.selectable_value(&mut prefs.filter.tank, t, name);
                }
            });
        ui.add(
            egui::TextEdit::singleline(&mut prefs.filter.map)
                .hint_text(crate::i18n::widget("Karte suchen …"))
                .desired_width(180.),
        );
        if ui.small_button("Filter zurücksetzen").clicked() {
            let player = prefs.filter.player.clone();
            prefs.filter = Filter {
                player,
                ..Default::default()
            };
        }
        ui.weak(crate::i18n::rich("Alle Ansichten verwenden dieselben Filter."));
    });
}
pub(super) fn vehicle_filter_button(ui: &mut egui::Ui, filter: &mut Filter) {
    let active = [
        !filter.nations.is_empty(),
        !filter.classes.is_empty(),
        !filter.tiers.is_empty(),
        !filter.roles.is_empty(),
    ]
    .into_iter()
    .filter(|active| *active)
    .count();
    let label = if active == 0 {
        "Panzerfilter".to_owned()
    } else {
        wot_insights::tr_format!("Panzerfilter ({active})")
    };
    ui.menu_button(crate::i18n::widget(label),|ui|{
        ui.set_width(660.);
        ui.strong(crate::i18n::rich("Panzer eingrenzen"));
        ui.weak(crate::i18n::rich("Mehrfachauswahl möglich. Innerhalb einer Gruppe gilt „oder“, zwischen Gruppen „und“."));
        ui.separator();
        egui::ScrollArea::vertical().id_salt("vehicle_filters").max_height(360.).show(ui,|ui|{
            ui.columns(4,|columns|{
                columns[0].strong(crate::i18n::rich("Nation"));
                if columns[0].selectable_label(filter.nations.is_empty(),crate::i18n::widget("Alle Nationen")).clicked(){filter.nations.clear();}
                for (nation,label) in vehicles::NATIONS{set_checkbox(&mut columns[0],&mut filter.nations,nation,label);}
                columns[1].strong(crate::i18n::rich("Panzerklasse"));
                if columns[1].selectable_label(filter.classes.is_empty(),crate::i18n::widget("Alle Klassen")).clicked(){filter.classes.clear();}
                for (class,label) in [("lightTank","Leicht"),("mediumTank","Mittel"),("heavyTank","Schwer"),("AT-SPG","Jagdpanzer"),("SPG","Artillerie"),("unknown","Unbekannt")]{set_checkbox(&mut columns[1],&mut filter.classes,class,label);}
                columns[2].strong(crate::i18n::rich("Tierstufe"));
                if columns[2].selectable_label(filter.tiers.is_empty(),crate::i18n::widget("Alle Stufen")).clicked(){filter.tiers.clear();}
                for tier in 1..=11u8{
                    let mut selected=filter.tiers.contains(&tier);
                    if columns[2].checkbox(&mut selected,wot_insights::tr_format!("{}  ({tier})",["I","II","III","IV","V","VI","VII","VIII","IX","X","XI"][tier as usize-1])).changed(){if selected{filter.tiers.insert(tier);}else{filter.tiers.remove(&tier);}}
                }
                columns[3].strong(crate::i18n::rich("Fahrzeugrolle"));
                if columns[3].selectable_label(filter.roles.is_empty(),crate::i18n::widget("Alle Rollen")).clicked(){filter.roles.clear();}
                for (role,label) in vehicles::ROLE_GROUPS{set_checkbox(&mut columns[3],&mut filter.roles,role,label);}
            });
        });
        ui.separator();
        if ui.button(crate::i18n::widget("Panzerfilter leeren")).clicked(){filter.nations.clear();filter.classes.clear();filter.tiers.clear();filter.roles.clear();filter.class.clear();filter.tier=0;}
        ui.weak(crate::i18n::rich("Rollen stammen aus dem aktuellen WoT-Client. Panzer ohne Rollenangabe: „Nicht zugeordnet“."));
    }).response.on_hover_text(crate::i18n::widget("Nation, Klasse, Tierstufe und Fahrzeugrolle auswählen"));
    if active > 0 {
        let groups = [
            filter
                .nations
                .iter()
                .map(|s| vehicles::nation_label(s))
                .collect::<Vec<_>>()
                .join(", "),
            filter
                .classes
                .iter()
                .map(|s| vehicles::class_label(s))
                .collect::<Vec<_>>()
                .join(", "),
            filter
                .tiers
                .iter()
                .map(|tier| wot_insights::tr_format!("Tier {tier}"))
                .collect::<Vec<_>>()
                .join(", "),
            filter
                .roles
                .iter()
                .map(|role| {
                    vehicles::ROLE_GROUPS
                        .iter()
                        .find(|(code, _)| *code == role)
                        .map(|(_, label)| *label)
                        .unwrap_or("Unbekannt")
                })
                .collect::<Vec<_>>()
                .join(", "),
        ];
        ui.label(crate::i18n::widget(
            RichText::new(crate::i18n::text(
                groups
                    .into_iter()
                    .filter(|g| !g.is_empty())
                    .collect::<Vec<_>>()
                    .join(" · "),
            ))
            .small()
            .color(MUTED),
        ));
    }
}
pub(super) fn set_checkbox(ui: &mut egui::Ui, set: &mut BTreeSet<String>, key: &str, label: &str) {
    let mut selected = set.contains(key);
    if ui
        .checkbox(&mut selected, crate::i18n::widget(label))
        .changed()
    {
        if selected {
            set.insert(key.into());
        } else {
            set.remove(key);
        }
    }
}
pub(super) fn period_label(days: i64) -> &'static str {
    match days {
        -1 => "Letzte Session",
        1 => "Heute",
        7 => "Letzte 7 Tage",
        30 => "Letzte 30 Tage",
        90 => "Letzte 90 Tage",
        _ => "Gesamter Zeitraum",
    }
}
