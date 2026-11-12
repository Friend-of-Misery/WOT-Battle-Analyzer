//! Credit accounting from the replay's battle-results fields.
//! Costs follow the client's TOTAL_CREDITS record: repair + autoLoadCost[0] + autoEquipCost[0].
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Economy {
    pub gross: Option<i64>,
    pub base: Option<i64>,
    pub premium: Option<i64>,
    pub personal_reserve: Option<i64>,
    pub clan_reserve: Option<i64>,
    pub missions: Option<i64>,
    pub other: Option<i64>,
    pub repair: Option<i64>,
    pub ammunition: Option<i64>,
    pub consumables: Option<i64>,
    pub gold_expenses: Option<i64>,
    pub bond_expenses: Option<i64>,
}
fn amount(s: &Value, key: &str) -> Option<i64> {
    s.get(key)?.as_i64()
}
fn cost(s: &Value, key: &str, index: usize) -> Option<i64> {
    s.get(key)?
        .as_array()?
        .get(index)?
        .as_i64()
        .filter(|n| *n >= 0)
}
pub fn parse(s: &Value) -> Economy {
    let base = amount(s, "originalCredits");
    let premium = base
        .zip(amount(s, "subtotalCredits"))
        .zip(amount(s, "appliedPremiumCreditsFactor100"))
        .and_then(|((base, subtotal), factor)| {
            let expected = base
                .checked_mul(factor)?
                .checked_add(50)?
                .checked_div(100)?;
            (factor > 0 && expected == subtotal).then(|| subtotal - base)
        });
    let mut economy = Economy {
        gross: amount(s, "credits"),
        base,
        premium,
        personal_reserve: amount(s, "boosterCredits"),
        clan_reserve: amount(s, "orderCredits"),
        missions: amount(s, "eventCredits"),
        other: None,
        repair: amount(s, "autoRepairCost").filter(|n| *n >= 0),
        ammunition: cost(s, "autoLoadCost", 0),
        consumables: cost(s, "autoEquipCost", 0),
        gold_expenses: cost(s, "autoLoadCost", 1)
            .zip(cost(s, "autoEquipCost", 1))
            .and_then(|(a, b)| a.checked_add(b)),
        bond_expenses: cost(s, "autoEquipCost", 2),
    };
    economy.other = economy
        .gross
        .zip(economy.base)
        .zip(economy.premium)
        .zip(economy.personal_reserve)
        .zip(economy.clan_reserve)
        .zip(economy.missions)
        .and_then(|(((((gross, base), premium), personal), clan), missions)| {
            gross
                .checked_sub(base)?
                .checked_sub(premium)?
                .checked_sub(personal)?
                .checked_sub(clan)?
                .checked_sub(missions)
        });
    economy
}
impl Economy {
    pub fn expenses(&self) -> Option<i64> {
        self.repair?
            .checked_add(self.ammunition?)?
            .checked_add(self.consumables?)
    }
    pub fn net(&self) -> Option<i64> {
        self.gross?.checked_sub(self.expenses()?)
    }
    pub fn without_missions(&self) -> Option<i64> {
        self.net()?.checked_sub(self.missions?)
    }
    pub fn without_missions_and_reserves(&self) -> Option<i64> {
        self.without_missions()?
            .checked_sub(self.personal_reserve?)?
            .checked_sub(self.clan_reserve?)
    }
}
#[derive(Clone, Debug, Default)]
pub struct MoneyMetric {
    pub sum: i64,
    pub count: usize,
}
impl MoneyMetric {
    fn add(&mut self, value: Option<i64>) {
        if let Some(value) = value {
            self.sum += value;
            self.count += 1;
        }
    }
    pub fn average(&self) -> Option<f64> {
        (self.count > 0).then(|| self.sum as f64 / self.count as f64)
    }
    pub fn total(&self) -> Option<i64> {
        (self.count > 0).then_some(self.sum)
    }
}
#[derive(Clone, Debug, Default)]
pub struct EconomySummary {
    pub gross: MoneyMetric,
    pub expenses: MoneyMetric,
    pub net: MoneyMetric,
    pub without_missions: MoneyMetric,
    pub without_reserves: MoneyMetric,
    pub repair: MoneyMetric,
    pub ammunition: MoneyMetric,
    pub consumables: MoneyMetric,
    pub missions: MoneyMetric,
    pub personal_reserve: MoneyMetric,
    pub clan_reserve: MoneyMetric,
    pub base: MoneyMetric,
    pub premium: MoneyMetric,
    pub other: MoneyMetric,
}
impl EconomySummary {
    pub fn add(&mut self, e: &Economy) {
        // Compare the same cohort: incomplete costs must not inflate income or reduce average costs.
        if e.net().is_none() {
            return;
        }
        self.net.add(e.net());
        self.gross.add(e.gross);
        self.expenses.add(e.expenses());
        self.without_missions.add(e.without_missions());
        self.without_reserves.add(e.without_missions_and_reserves());
        self.repair.add(e.repair);
        self.ammunition.add(e.ammunition);
        self.consumables.add(e.consumables);
        self.missions.add(e.missions);
        self.personal_reserve.add(e.personal_reserve);
        self.clan_reserve.add(e.clan_reserve);
        self.base.add(e.base);
        self.premium.add(e.premium);
        self.other.add(e.other);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quest_is_already_in_gross_and_costs_are_not_double_counted() {
        let e = parse(
            &serde_json::json!({"credits":100390,"originalCredits":22395,"subtotalCredits":33593,"appliedPremiumCreditsFactor100":150,"boosterCredits":16797,"orderCredits":0,"eventCredits":50000,"autoRepairCost":15128,"autoLoadCost":[16842,0],"autoEquipCost":[20000,0,0]}),
        );
        assert_eq!(e.premium, Some(11198));
        assert_eq!(e.other, Some(0));
        assert_eq!(e.expenses(), Some(51970));
        assert_eq!(e.net(), Some(48420));
        assert_eq!(e.without_missions(), Some(-1580));
        assert_eq!(e.without_missions_and_reserves(), Some(-18377));
    }
    #[test]
    fn missing_cost_is_unknown_and_zero_cost_is_valid() {
        let mut e =
            parse(&serde_json::json!({"credits":1000,"autoRepairCost":0,"autoLoadCost":[0,0]}));
        assert_eq!(e.net(), None);
        e.consumables = Some(0);
        assert_eq!(e.net(), Some(1000));
        assert_eq!(e.without_missions(), None);
    }
    #[test]
    fn currencies_stay_separate_and_missing_records_do_not_bias_averages() {
        let e = parse(
            &serde_json::json!({"credits":1000,"autoRepairCost":100,"autoLoadCost":[300,2],"autoEquipCost":[200,3,10]}),
        );
        assert_eq!(e.net(), Some(400));
        assert_eq!(e.gold_expenses, Some(5));
        assert_eq!(e.bond_expenses, Some(10));
        let mut summary = EconomySummary::default();
        summary.add(&e);
        summary.add(&Economy {
            gross: Some(100000),
            ..Default::default()
        });
        assert_eq!(summary.net.count, 1);
        assert_eq!(summary.net.average(), Some(400.));
        assert_eq!(summary.gross.average(), Some(1000.));
    }
}
