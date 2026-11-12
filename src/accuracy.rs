use crate::replay::Battle;

pub fn percentage(numerator: Option<i64>, denominator: Option<i64>) -> Option<f64> {
    numerator
        .zip(denominator)
        .filter(|(n, d)| *n >= 0 && *d > 0)
        .map(|(n, d)| n as f64 / d as f64 * 100.)
}
#[derive(Clone, Debug, Default)]
pub struct AccuracySummary {
    pub count: usize,
    pub shots: i64,
    pub hits: i64,
    pub penetrations: i64,
}
impl AccuracySummary {
    pub fn add(&mut self, b: &Battle) {
        if b.result.is_none() {
            return;
        }
        if let (Some(shots), Some(hits), Some(penetrations)) = (b.shots, b.hits, b.penetrations)
            && shots >= 0
            && hits >= 0
            && penetrations >= 0
        {
            self.count += 1;
            self.shots += shots;
            self.hits += hits;
            self.penetrations += penetrations;
        }
    }
    pub fn hit_rate(&self) -> Option<f64> {
        percentage(Some(self.hits), Some(self.shots))
    }
    pub fn penetration_rate(&self) -> Option<f64> {
        percentage(Some(self.penetrations), Some(self.hits))
    }
    pub fn penetration_per_shot(&self) -> Option<f64> {
        percentage(Some(self.penetrations), Some(self.shots))
    }
    pub fn average_shots(&self) -> Option<f64> {
        (self.count > 0).then(|| self.shots as f64 / self.count as f64)
    }
    pub fn average_hits(&self) -> Option<f64> {
        (self.count > 0).then(|| self.hits as f64 / self.count as f64)
    }
    pub fn average_penetrations(&self) -> Option<f64> {
        (self.count > 0).then(|| self.penetrations as f64 / self.count as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn battle(shots: i64, hits: i64, pens: i64) -> Battle {
        let mut b=crate::replay::from_json(&serde_json::json!({"playerName":"P","playerID":1,"playerVehicle":"T","dateTime":"08.11.2026 12:00:00","mapName":"M","mapDisplayName":"M","battleType":1}),None,&Default::default()).unwrap();
        b.result = Some("win".into());
        b.shots = Some(shots);
        b.hits = Some(hits);
        b.penetrations = Some(pens);
        b
    }
    #[test]
    fn rates_are_weighted_by_shots_and_hits_and_unknown_records_are_excluded() {
        let mut summary = AccuracySummary::default();
        summary.add(&battle(2, 1, 1));
        summary.add(&battle(100, 25, 10));
        let mut missing = battle(1000, 500, 200);
        missing.penetrations = None;
        summary.add(&missing);
        assert_eq!(summary.count, 2);
        assert_eq!(summary.shots, 102);
        assert_eq!(summary.hits, 26);
        assert_eq!(summary.penetrations, 11);
        assert!((summary.hit_rate().unwrap() - 26. / 102. * 100.).abs() < 1e-10);
        assert!((summary.penetration_rate().unwrap() - 11. / 26. * 100.).abs() < 1e-10);
    }
    #[test]
    fn zero_shots_are_known_but_do_not_make_zero_percent_and_multi_hits_are_not_capped() {
        let mut summary = AccuracySummary::default();
        summary.add(&battle(0, 0, 0));
        assert_eq!(summary.average_shots(), Some(0.));
        assert_eq!(summary.hit_rate(), None);
        assert_eq!(summary.penetration_rate(), None);
        assert_eq!(percentage(Some(3), Some(2)), Some(150.));
        assert_eq!(percentage(None, Some(2)), None);
    }
}
