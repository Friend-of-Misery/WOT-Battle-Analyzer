use super::*;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct Columns {
    pub(super) assist: bool,
    pub(super) kills: bool,
    pub(super) xp: bool,
    pub(super) credits: bool,
    pub(super) net: bool,
    pub(super) shot_details: bool,
    pub(super) received: bool,
}
impl Default for Columns {
    fn default() -> Self {
        Self {
            assist: true,
            kills: true,
            xp: true,
            credits: false,
            net: true,
            shot_details: true,
            received: false,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct Preferences {
    pub(super) language: String,
    pub(super) font_size: f32,
    pub(super) filter: Filter,
    pub(super) favorites: BTreeSet<String>,
    pub(super) columns: Columns,
    pub(super) replay_folder: String,
    pub(super) game_folder: String,
    pub(super) automatic: bool,
    pub(super) sql_enabled: bool,
    pub(super) view: usize,
    pub(super) minimum: usize,
    pub(super) map_mode: usize,
    pub(super) map_metric: usize,
    pub(super) trend_metric: usize,
    pub(super) finance_basis: usize,
    pub(super) finance_minimum: usize,
    pub(super) finance_mode: usize,
    pub(super) tank_mode: usize,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            language: "de".into(),
            font_size: 17.,
            filter: Filter::default(),
            favorites: BTreeSet::new(),
            columns: Columns::default(),
            replay_folder: String::new(),
            game_folder: String::new(),
            automatic: true,
            sql_enabled: false,
            view: 0,
            minimum: 10,
            map_mode: 0,
            map_metric: 0,
            trend_metric: 0,
            finance_basis: 1,
            finance_minimum: 5,
            finance_mode: 0,
            tank_mode: 0,
        }
    }
}
impl Preferences {
    pub(super) fn load(path: &std::path::Path) -> Result<Self> {
        let mut prefs: Self = serde_json::from_slice(&std::fs::read(path)?)?;
        if !crate::i18n::valid_language(&prefs.language) {
            prefs.language = "de".into();
        }
        prefs.font_size = if prefs.font_size.is_finite() {
            prefs.font_size.clamp(14., 22.)
        } else {
            17.
        };
        prefs.view = prefs.view.min(11);
        if prefs.view == 8 && !prefs.sql_enabled {
            prefs.view = 5;
        }
        prefs.finance_basis = prefs.finance_basis.min(2);
        prefs.finance_minimum = prefs.finance_minimum.clamp(1, 500);
        prefs.finance_mode = prefs.finance_mode.min(1);
        prefs.tank_mode = prefs.tank_mode.min(1);
        prefs.minimum = prefs.minimum.clamp(1, 500);
        prefs.trend_metric = prefs.trend_metric.min(4);
        prefs.map_mode = prefs.map_mode.min(1);
        prefs.map_metric = prefs.map_metric.min(2);
        if !prefs.filter.class.is_empty() {
            prefs
                .filter
                .classes
                .insert(std::mem::take(&mut prefs.filter.class));
        }
        if prefs.filter.tier != 0 {
            prefs
                .filter
                .tiers
                .insert(std::mem::take(&mut prefs.filter.tier));
        }
        Ok(prefs)
    }
    pub(super) fn save(&self, path: &std::path::Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let temp = path.with_extension("json.tmp");
        std::fs::write(&temp, serde_json::to_vec_pretty(self)?)?;
        std::fs::rename(temp, path).context("Einstellungen konnten nicht gespeichert werden")
    }
}
