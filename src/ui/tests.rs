use super::*;
use wot_insights::store::Store;
fn sample() -> Battle {
    let mut b=wot_insights::replay::from_json(&serde_json::json!({"playerName":"Test","playerID":1,"playerVehicle":"usa-A07_T20","dateTime":"08.11.2026 12:00:00","mapName":"map","mapDisplayName":"Karte","battleType":1}),None,&Default::default()).unwrap();
    b.result = Some("win".into());
    b.damage = Some(1200);
    b.assist = Some(300);
    b.received = Some(800);
    b.kills = Some(2);
    b.class = "mediumTank".into();
    b.tier = 7;
    b.shots = Some(10);
    b.hits = Some(6);
    b.penetrations = Some(4);
    b.explosion_hits = Some(2);
    b.economy = Some(wot_insights::economy::parse(
        &serde_json::json!({"credits":2000,"originalCredits":1000,"subtotalCredits":1500,"appliedPremiumCreditsFactor100":150,"boosterCredits":500,"orderCredits":0,"eventCredits":0,"autoRepairCost":100,"autoLoadCost":[200,0],"autoEquipCost":[300,0,0]}),
    ));
    b
}
#[test]
fn preferences_roundtrip_keeps_favorites_columns_and_filters() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("settings.json");
    let mut p = Preferences::default();
    p.favorites.insert("usa-A07_T20".into());
    p.columns.credits = true;
    p.filter.days = -1;
    p.language = "pl".into();
    p.font_size = 19.;
    p.view = 8;
    p.sql_enabled = true;
    p.save(&path).unwrap();
    assert_eq!(Preferences::load(&path).unwrap(), p);
    p.filter.days = 7;
    p.save(&path).unwrap();
    assert_eq!(Preferences::load(&path).unwrap().filter.days, 7);
}
#[test]
fn every_view_renders_with_data_at_supported_window_sizes() {
    let temp = tempfile::tempdir().unwrap();
    Store::open(&temp.path().join("stats.db")).unwrap();
    for size in [Vec2::new(1000., 620.), Vec2::new(1280., 820.)] {
        for (code, _) in crate::i18n::LANGUAGES {
            for view in 0..12 {
                let ctx = egui::Context::default();
                configure(&ctx);
                let mut app = App {
                    preferences: Preferences {
                        view,
                        language: code.to_string(),
                        font_size: if size.x > 1000. { 22. } else { 17. },
                        automatic: false,
                        sql_enabled: view == 8,
                        finance_minimum: 1,
                        finance_mode: usize::from(size.x > 1000.),
                        tank_mode: usize::from(size.x > 1000.),
                        ..Default::default()
                    },
                    settings_path: temp.path().join("settings.json"),
                    database: temp.path().join("stats.db"),
                    battles: vec![sample()],
                    worker: Default::default(),
                    receiver: None,
                    last_scan: Instant::now(),
                    last_success: None,
                    message: "Testdaten".into(),
                    import_failed: false,
                    errors: Vec::new(),
                    tank_sort: Sorting {
                        column: Sort::Battles,
                        descending: true,
                    },
                    battle_sort: Sorting {
                        column: Sort::Date,
                        descending: true,
                    },
                    finance_sort: Sorting {
                        column: Sort::CleanNet,
                        descending: true,
                    },
                    page: 0,
                    selected: (view == 2).then(|| sample().key),
                    details: Default::default(),
                    cleanup_candidates: Vec::new(),
                    cleanup_folder: String::new(),
                    cleanup_confirmed: false,
                    cleanup_message: String::new(),
                    cleanup_receiver: None,
                    sql: Default::default(),
                    damage_view: Default::default(),
                    player_view: Default::default(),
                };
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                        ..Default::default()
                    },
                    |ctx| app.render(ctx),
                );
                assert!(!output.shapes.is_empty(), "view {view}");
                assert!(
                    output.shapes.iter().all(|s| s.clip_rect.is_finite()),
                    "view {view}"
                );
            }
        }
    }
}

#[test]
fn old_settings_default_to_german_and_reject_unknown_languages() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("settings.json");
    std::fs::write(&path, r#"{}"#).unwrap();
    assert_eq!(Preferences::load(&path).unwrap().language, "de");
    std::fs::write(&path, r#"{"language":"unknown"}"#).unwrap();
    assert_eq!(Preferences::load(&path).unwrap().language, "de");
}
