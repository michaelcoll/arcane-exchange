use crate::domain::stats::Stats;
use serde::Serialize;
use ts_rs::TS;
use utoipa::ToSchema;

/// Global platform statistics, public.
#[derive(Serialize, Debug, TS, ToSchema)]
#[serde(rename = "Stats")]
#[ts(export)]
pub struct StatsResponse {
    /// Number of Cards known to the platform.
    pub card_number: u32,
    /// Number of Cardmarket prices recorded.
    pub card_price_number: u32,
    /// Database size, in MB.
    pub db_size_mb: u16,
    /// ISO 8601 date string (YYYY-MM-DD) of the most recent Cardmarket price — prices are dated
    /// to the day. `null` until a price has been imported.
    pub last_price_date: Option<String>,
    /// Copies offered for trade across the platform: the sum of every player's proposed
    /// quantities, after collection visibility, trading binders and rarity filters.
    pub proposed_copy_number: u32,
}

impl From<Stats> for StatsResponse {
    fn from(stats: Stats) -> Self {
        Self {
            card_number: stats.card_number,
            card_price_number: stats.card_price_number,
            db_size_mb: stats.db_size_mb,
            last_price_date: stats.last_price_date.map(|d| d.to_string()),
            proposed_copy_number: stats.proposed_copy_number,
        }
    }
}
