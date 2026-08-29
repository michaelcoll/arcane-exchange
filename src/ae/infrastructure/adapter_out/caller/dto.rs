use crate::domain::price::{FullPriceGuide, Price, PriceGuide};
use chrono::{DateTime, Utc};

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardmarketPriceGuides {
    pub created_at: DateTime<Utc>,
    pub price_guides: Vec<CardmarketPriceGuide>,
}

#[derive(Debug, serde::Deserialize)]
pub struct CardmarketPriceGuide {
    #[serde(rename(deserialize = "idProduct"))]
    pub id_product: u32,
    pub avg: Option<f32>,
    pub low: Option<f32>,
    pub trend: Option<f32>,
    #[serde(rename(deserialize = "avg-foil"))]
    pub avg_foil: Option<f32>,
    #[serde(rename(deserialize = "low-foil"))]
    pub low_foil: Option<f32>,
    #[serde(rename(deserialize = "trend-foil"))]
    pub trend_foil: Option<f32>,
}

impl From<Option<f32>> for Price {
    fn from(value: Option<f32>) -> Self {
        value
            .map(|v| (v * 100.0).round() as u32)
            .map(Price::from_cents)
            .unwrap_or_else(Price::empty)
    }
}

impl From<CardmarketPriceGuide> for FullPriceGuide {
    fn from(value: CardmarketPriceGuide) -> Self {
        FullPriceGuide {
            id_product: value.id_product,
            normal: PriceGuide::new(value.low, value.trend, value.avg),
            foil: PriceGuide::new(value.low_foil, value.trend_foil, value.avg_foil),
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct EdhRecCardInfo {
    #[serde(rename(deserialize = "pageProps"))]
    pub page_props: EdhRecProps,
}

#[derive(Debug, serde::Deserialize)]
pub struct EdhRecProps {
    pub data: EdhRecData,
}

#[derive(Debug, serde::Deserialize)]
pub struct EdhRecData {
    pub container: EdhRecContainer,
}

#[derive(Debug, serde::Deserialize)]
pub struct EdhRecContainer {
    pub json_dict: EdhRecJsonDict,
}

#[derive(Debug, serde::Deserialize)]
pub struct EdhRecJsonDict {
    pub card: EdhRecCard,
}

#[derive(Debug, serde::Deserialize)]
pub struct EdhRecCard {
    pub inclusion: i32,
    pub potential_decks: i32,
}

#[derive(Debug, serde::Deserialize)]
pub struct ScryfallCardInfo {
    pub cardmarket_id: Option<i32>,
}
