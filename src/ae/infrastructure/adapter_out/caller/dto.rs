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
    /// Set on a single-image card, split/adventure/flip cards included.
    pub image_uris: Option<ScryfallImageUris>,
    /// Each face carries its own images only on a card with two physical faces.
    #[serde(default)]
    pub card_faces: Vec<ScryfallCardFace>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ScryfallCardFace {
    pub image_uris: Option<ScryfallImageUris>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ScryfallImageUris {
    pub normal: Option<String>,
}

impl ScryfallCardInfo {
    /// The `normal` image URL of every face — front, then back for a double-faced card — or
    /// `None` when Scryfall has no image for the card.
    pub fn normal_image_urls(&self) -> Option<(String, Option<String>)> {
        if let Some(front) = self
            .image_uris
            .as_ref()
            .and_then(|uris| uris.normal.clone())
        {
            return Some((front, None));
        }
        let mut faces = self.card_faces.iter().map(|face| {
            face.image_uris
                .as_ref()
                .and_then(|uris| uris.normal.clone())
        });
        let front = faces.next().flatten()?;
        match faces.next() {
            None => Some((front, None)),
            Some(Some(back)) => Some((front, Some(back))),
            Some(None) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(faces: serde_json::Value) -> ScryfallCardInfo {
        serde_json::from_value(serde_json::json!({ "card_faces": faces })).unwrap()
    }

    #[test]
    fn a_single_face_with_its_own_image_has_no_back() {
        let card = card(serde_json::json!([{ "image_uris": { "normal": "front" } }]));

        assert_eq!(card.normal_image_urls(), Some(("front".to_string(), None)));
    }

    #[test]
    fn a_double_faced_card_without_a_back_image_has_no_image() {
        let card = card(serde_json::json!([
            { "image_uris": { "normal": "front" } },
            { "image_uris": null }
        ]));

        assert_eq!(card.normal_image_urls(), None);
    }
}
