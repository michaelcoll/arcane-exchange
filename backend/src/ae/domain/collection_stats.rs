use crate::domain::price::Price;

pub struct BinderInfo {
    pub name: String,
    pub card_count: u64,
}

pub struct CollectionStats {
    pub total_cards: u64,
    pub unique_cards: u64,
    pub price_trend_min: Price,
    pub price_trend_max: Price,
    pub binders: Vec<BinderInfo>,
}
