use crate::domain::stats::Stats;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, Debug, ToSchema)]
pub struct StatsResponse {
    pub card_number: u32,
    pub card_price_number: u32,
    pub db_size_mb: u16,
}

impl From<Stats> for StatsResponse {
    fn from(stats: Stats) -> Self {
        Self {
            card_number: stats.card_number,
            card_price_number: stats.card_price_number,
            db_size_mb: stats.db_size_mb,
        }
    }
}

#[derive(Serialize, Debug, ToSchema)]
pub struct EnqueueResponse {
    pub enqueued: usize,
}
