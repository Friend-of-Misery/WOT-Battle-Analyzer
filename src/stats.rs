use crate::replay::Battle;
use std::collections::BTreeMap;

#[derive(Default, Debug, Clone)]
pub struct Summary {
    pub count: usize,
    pub known_results: usize,
    pub wins: usize,
    pub damage: i64,
    pub damage_count: usize,
    pub assist: i64,
    pub assist_count: usize,
    pub base_xp: i64,
    pub xp_count: usize,
    pub kills: i64,
    pub kills_count: usize,
    pub credits: i64,
    pub credits_count: usize,
    pub paired_damage: i64,
    pub paired_received: i64,
    pub economy: crate::economy::EconomySummary,
    pub accuracy: crate::accuracy::AccuracySummary,
}
impl Summary {
    pub fn add(&mut self, b: &Battle) {
        self.count += 1;
        if let Some(r) = &b.result {
            self.known_results += 1;
            self.wins += usize::from(r == "win");
        }
        // End-result metrics only: incomplete replays must not bias the averages.
        if b.result.is_none() {
            return;
        }
        if let Some(economy) = &b.economy {
            self.economy.add(economy);
        }
        self.accuracy.add(b);
        if let Some(v) = b.damage {
            self.damage += v;
            self.damage_count += 1;
        }
        if let Some(v) = b.assist {
            self.assist += v;
            self.assist_count += 1;
        }
        if let Some(v) = b.base_xp {
            self.base_xp += v;
            self.xp_count += 1;
        }
        if let Some(v) = b.kills {
            self.kills += v;
            self.kills_count += 1;
        }
        if let Some(v) = b.credits {
            self.credits += v;
            self.credits_count += 1;
        }
        if let (Some(damage), Some(received)) = (b.damage, b.received) {
            self.paired_damage += damage;
            self.paired_received += received;
        }
    }
    pub fn winrate(&self) -> Option<f64> {
        ratio(self.wins as f64 * 100., self.known_results)
    }
    pub fn avg_damage(&self) -> Option<f64> {
        ratio(self.damage as f64, self.damage_count)
    }
    pub fn avg_assist(&self) -> Option<f64> {
        ratio(self.assist as f64, self.assist_count)
    }
    pub fn avg_xp(&self) -> Option<f64> {
        ratio(self.base_xp as f64, self.xp_count)
    }
    pub fn avg_kills(&self) -> Option<f64> {
        ratio(self.kills as f64, self.kills_count)
    }
    pub fn avg_credits(&self) -> Option<f64> {
        ratio(self.credits as f64, self.credits_count)
    }
    pub fn damage_ratio(&self) -> Option<f64> {
        (self.paired_received > 0).then(|| self.paired_damage as f64 / self.paired_received as f64)
    }
}
fn ratio(value: f64, count: usize) -> Option<f64> {
    (count > 0).then(|| value / count as f64)
}

pub fn summarize<'a>(battles: impl IntoIterator<Item = &'a Battle>) -> Summary {
    let mut s = Summary::default();
    for b in battles {
        s.add(b);
    }
    s
}
pub fn maps_by_class<'a>(
    battles: impl IntoIterator<Item = &'a Battle>,
) -> BTreeMap<(String, String), Summary> {
    let mut rows = BTreeMap::<_, Summary>::new();
    for b in battles {
        rows.entry((b.map.clone(), b.class.clone()))
            .or_default()
            .add(b);
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::from_json;
    #[test]
    fn unknown_results_do_not_change_winrate_or_average() {
        let h = serde_json::json!({"playerName":"A","playerID":1,"playerVehicle":"T","dateTime":"08.11.2026 12:00:00","mapName":"M","mapDisplayName":"M","battleType":1});
        let mut win = from_json(&h, None, &Default::default()).unwrap();
        win.result = Some("win".into());
        win.damage = Some(1000);
        let unknown = from_json(&h, None, &Default::default()).unwrap();
        let s = summarize([&win, &unknown]);
        assert_eq!(s.count, 2);
        assert_eq!(s.known_results, 1);
        assert_eq!(s.winrate(), Some(100.));
        assert_eq!(s.avg_damage(), Some(1000.));
    }
    #[test]
    fn damage_ratio_uses_only_pairs_and_does_not_invent_infinite_values() {
        let h = serde_json::json!({"playerName":"A","playerID":1,"playerVehicle":"T","dateTime":"08.11.2026 12:00:00","mapName":"M","mapDisplayName":"M","battleType":1});
        let mut a = from_json(&h, None, &Default::default()).unwrap();
        a.result = Some("win".into());
        a.damage = Some(1000);
        a.received = Some(500);
        let mut missing = a.clone();
        missing.damage = Some(9000);
        missing.received = None;
        let s = summarize([&a, &missing]);
        assert_eq!(s.damage_ratio(), Some(2.));
        a.received = Some(0);
        assert_eq!(summarize([&a]).damage_ratio(), None);
    }
}
