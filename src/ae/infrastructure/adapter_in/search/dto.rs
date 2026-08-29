pub(crate) use crate::infrastructure::adapter_in::collection::dto::{
    RarityCodeParam, SortByParam, SortDirParam, default_page_size,
};
use serde::Deserialize;
use ts_rs::TS;

#[derive(Deserialize, TS)]
#[ts(export, export_to = "SearchParams.ts")]
pub(crate) struct SearchParams {
    #[serde(default)]
    pub(crate) page: u32,
    #[serde(default = "default_page_size")]
    pub(crate) page_size: u32,
    #[serde(default)]
    pub(crate) sort_by: SortByParam,
    #[serde(default)]
    pub(crate) sort_dir: SortDirParam,
    #[ts(optional)]
    pub(crate) q: Option<String>,
    /// Rarity codes, repeated for multiple values (e.g. `?rarity=C&rarity=U`)
    #[serde(default)]
    pub(crate) rarity: Vec<RarityCodeParam>,
    /// Comma-separated set codes
    #[ts(optional)]
    pub(crate) sets: Option<String>,
    /// Minimum trend price in cents
    #[ts(optional)]
    pub(crate) price_min: Option<u32>,
    /// Maximum trend price in cents
    #[ts(optional)]
    pub(crate) price_max: Option<u32>,
    /// Exact username of the owner to filter by (case-insensitive, no partial match)
    #[ts(optional)]
    pub(crate) player_username: Option<String>,
}

impl Default for SearchParams {
    fn default() -> Self {
        Self {
            page: 0,
            page_size: default_page_size(),
            sort_by: SortByParam::default(),
            sort_dir: SortDirParam::default(),
            q: None,
            rarity: Vec::new(),
            sets: None,
            price_min: None,
            price_max: None,
            player_username: None,
        }
    }
}
