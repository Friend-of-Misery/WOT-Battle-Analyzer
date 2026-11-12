//! Shared filtering and aggregations for all views; no UI or network dependencies.
use crate::{replay::Battle, stats::Summary, vehicles};
use chrono::{Duration, NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Filter {
    pub player: String,
    pub class: String,
    pub tank: String,
    pub tank_search: String,
    pub tier: u8,
    pub nations: BTreeSet<String>,
    pub classes: BTreeSet<String>,
    pub tiers: BTreeSet<u8>,
    pub roles: BTreeSet<String>,
    pub map: String,
    /// 0: all, -1: latest session (gap > 60 minutes), otherwise calendar days.
    pub days: i64,
    pub favorites_only: bool,
}

pub fn matches_tank(tank: &str, query: &str) -> bool {
    let normalize = |text: &str| {
        text.chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect::<String>()
    };
    normalize(tank).contains(&normalize(query))
}

pub fn matches_vehicle(battle: &Battle, filter: &Filter) -> bool {
    (filter.class.is_empty() || battle.class == filter.class)
        && (filter.tier == 0 || battle.tier == filter.tier)
        && (filter.classes.is_empty() || filter.classes.contains(&battle.class))
        && (filter.tiers.is_empty() || filter.tiers.contains(&battle.tier))
        && (filter.nations.is_empty() || filter.nations.contains(vehicles::nation(&battle.tank)))
        && (filter.roles.is_empty() || filter.roles.contains(vehicles::role_group(&battle.role)))
        && matches_tank(&battle.tank, &filter.tank_search)
}

pub fn latest_session_start(battles: &[Battle], player: &str) -> Option<String> {
    let mut dates: Vec<_> = battles
        .iter()
        .filter(|b| b.mode == 1 && (player.is_empty() || b.player == player))
        .filter_map(|b| NaiveDateTime::parse_from_str(&b.date, "%Y-%m-%d %H:%M:%S").ok())
        .collect();
    dates.sort_unstable_by(|a, b| b.cmp(a));
    let mut start = *dates.first()?;
    for date in dates.iter().skip(1) {
        if start.signed_duration_since(*date) > Duration::minutes(60) {
            break;
        }
        start = *date;
    }
    Some(start.format("%Y-%m-%d %H:%M:%S").to_string())
}

pub fn filter<'a>(
    battles: &'a [Battle],
    filter: &Filter,
    favorites: &BTreeSet<String>,
    today: NaiveDate,
) -> Vec<&'a Battle> {
    let cutoff = match filter.days {
        -1 => latest_session_start(battles, &filter.player).unwrap_or_default(),
        days if days > 0 => (today - Duration::days(days.min(36500) - 1))
            .format("%Y-%m-%d")
            .to_string(),
        _ => String::new(),
    };
    let map = filter.map.to_lowercase();
    let mut rows: Vec<_> = battles
        .iter()
        .filter(|b| {
            b.mode == 1
                && (filter.player.is_empty() || b.player == filter.player)
                && matches_vehicle(b, filter)
                && (filter.tank.is_empty() || b.tank == filter.tank)
                && (map.is_empty() || b.map.to_lowercase().contains(&map))
                && (cutoff.is_empty() || b.date >= cutoff)
                && (!filter.favorites_only || favorites.contains(&b.tank))
        })
        .collect();
    rows.sort_unstable_by(|a, b| b.date.cmp(&a.date).then(a.key.cmp(&b.key)));
    rows
}

#[derive(Clone, Debug)]
pub struct TankRow {
    pub tank: String,
    pub class: String,
    pub tier: u8,
    pub role: String,
    pub summary: Summary,
}

pub fn tanks(battles: &[&Battle]) -> Vec<TankRow> {
    let mut rows = BTreeMap::<_, Summary>::new();
    for b in battles {
        rows.entry((b.tank.clone(), b.class.clone(), b.tier, b.role.clone()))
            .or_default()
            .add(b);
    }
    rows.into_iter()
        .map(|((tank, class, tier, role), summary)| TankRow {
            tank,
            class,
            tier,
            role,
            summary,
        })
        .collect()
}
pub fn days(battles: &[&Battle]) -> Vec<(String, Summary)> {
    let mut days = BTreeMap::<String, Summary>::new();
    for b in battles {
        days.entry(b.date[..10].to_owned()).or_default().add(b);
    }
    days.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::from_json;
    fn battle(date: &str, tank: &str, player: &str) -> Battle {
        from_json(&serde_json::json!({"playerName":player,"playerID":1,"playerVehicle":tank,"dateTime":date,"mapName":"map","mapDisplayName":"Karte","battleType":1}),None,&Default::default()).unwrap()
    }
    #[test]
    fn session_uses_gaps_and_player_not_current_tank_filter() {
        let battles = vec![
            battle("08.11.2026 15:00:00", "A", "P"),
            battle("08.11.2026 16:10:00", "A", "P"),
            battle("08.11.2026 16:50:00", "B", "P"),
            battle("08.11.2026 17:01:00", "A", "Q"),
        ];
        assert_eq!(
            latest_session_start(&battles, "P").as_deref(),
            Some("2026-11-08 16:10:00")
        );
        let f = Filter {
            player: "P".into(),
            tank: "A".into(),
            days: -1,
            ..Default::default()
        };
        assert_eq!(
            filter(
                &battles,
                &f,
                &BTreeSet::new(),
                NaiveDate::from_ymd_opt(2026, 11, 8).unwrap()
            )
            .len(),
            1
        );
    }
    #[test]
    fn period_and_favorites_apply_to_every_view() {
        let battles = vec![
            battle("07.11.2026 23:59:59", "A", "P"),
            battle("08.11.2026 00:00:00", "A", "P"),
            battle("08.11.2026 12:00:00", "B", "P"),
        ];
        let f = Filter {
            days: 1,
            favorites_only: true,
            ..Default::default()
        };
        let rows = filter(
            &battles,
            &f,
            &BTreeSet::from(["A".into()]),
            NaiveDate::from_ymd_opt(2026, 11, 8).unwrap(),
        );
        assert_eq!(rows.len(), 1);
        assert_eq!(tanks(&rows)[0].summary.count, 1);
        assert_eq!(days(&rows)[0].1.count, 1);
    }
    #[test]
    fn vehicle_filters_use_or_within_and_between_groups() {
        let mut german = battle("08.11.2026 12:00:00", "germany-G01_Test", "P");
        german.class = "heavyTank".into();
        german.tier = 8;
        german.role = "role_HT_assault".into();
        let mut american = german.clone();
        american.tank = "usa-A01_Test".into();
        american.tier = 10;
        let mut support = german.clone();
        support.role = "role_HT_support".into();
        let mut russian = german.clone();
        russian.tank = "ussr-R01_Test".into();
        let mut low_tier = german.clone();
        low_tier.tier = 6;
        let mut unknown = german.clone();
        unknown.role.clear();
        let f = Filter {
            nations: BTreeSet::from(["germany".into(), "usa".into()]),
            classes: BTreeSet::from(["heavyTank".into()]),
            tiers: BTreeSet::from([8, 9, 10]),
            roles: BTreeSet::from(["assault".into()]),
            ..Default::default()
        };
        let battles = vec![german, american, support, russian, low_tier, unknown];
        let rows = filter(
            &battles,
            &f,
            &BTreeSet::new(),
            NaiveDate::from_ymd_opt(2026, 11, 8).unwrap(),
        );
        assert_eq!(rows.len(), 2);
        assert_eq!(tanks(&rows).len(), 2);
        let f = Filter {
            roles: BTreeSet::from(["unknown".into()]),
            ..Default::default()
        };
        assert_eq!(
            filter(
                &battles,
                &f,
                &BTreeSet::new(),
                NaiveDate::from_ymd_opt(2026, 11, 8).unwrap()
            )
            .len(),
            1
        );
    }
}
