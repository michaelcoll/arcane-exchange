use crate::domain::price::Price;
use crate::domain::set_name::SetName;

pub struct BinderInfo {
    pub name: String,
    pub card_count: u64,
}

pub struct CollectionStats {
    pub total_cards: u64,
    pub unique_cards: u64,
    pub price_trend_min: Price,
    pub price_trend_max: Price,
    pub sets: Vec<SetName>,
    pub binders: Vec<BinderInfo>,
}
