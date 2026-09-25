use crate::application::error::AppError;
use crate::domain::card::CardInfo;
use crate::domain::card_image::CardImages;
use crate::domain::language_code::LanguageCode;
use crate::domain::price::FullPriceGuide;
use crate::domain::set_name::SetCode;
use async_trait::async_trait;
use chrono::NaiveDate;
use std::fmt::{Display, Formatter};
use uuid::Uuid;

#[cfg(test)]
use mockall::automock;

#[async_trait]
#[cfg_attr(test, automock)]
pub trait CardMarketCaller: Send + Sync {
    async fn get_price_guides(&self) -> Result<(NaiveDate, Vec<FullPriceGuide>), AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait EdhRecCaller: Send + Sync {
    async fn get_card_info(&self, card_name: String) -> Result<CardInfo, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait ScryfallCaller: Send + Sync {
    async fn get_card_market_id(&self, id: Uuid) -> Result<Option<u32>, AppError>;

    /// The card's `normal` images (488 × 680), converted to WebP, or `None` when Scryfall does not know the
    /// card or has no image for it. A technical failure (timeout, 5xx, rate limit) is an error.
    async fn get_card_images(&self, id: Uuid) -> Result<Option<CardImages>, AppError>;
}

/// A card as Gatherer shows it in one language.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GathererCard {
    /// Id of the front image, the card's `the_gatherer_id`.
    pub gatherer_id: String,
    /// The images as served by Gatherer (WebP), whatever their size.
    pub images: CardImages,
}

/// What Gatherer answered for a card in one language.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GathererLookup {
    Found(GathererCard),
    /// Gatherer does not have the card, or not all its faces: the next source is tried.
    NotFound(GathererMiss),
}

/// Why Gatherer does not have a card, as logged when a card falls back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GathererMiss {
    /// 404 on the card page.
    NoPage,
    /// The page has no `og:image`.
    NoImageOnPage,
    /// 404 on the front image.
    NoImage,
    /// A double-faced card whose page gives no back image.
    NoBackOnPage,
    /// 404 on the back image.
    NoBack,
}

impl Display for GathererMiss {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            GathererMiss::NoPage => "page not found",
            GathererMiss::NoImageOnPage => "no image on the page",
            GathererMiss::NoImage => "image not found",
            GathererMiss::NoBackOnPage => "no back on the page",
            GathererMiss::NoBack => "back image not found",
        })
    }
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait GathererCaller: Send + Sync {
    /// The card's page and images in `language_code`, or why Gatherer does not have them. A
    /// technical failure (timeout, 5xx, rate limit) is an error.
    async fn get_card(
        &self,
        set_code: SetCode,
        collector_number: String,
        language_code: LanguageCode,
        name: String,
    ) -> Result<GathererLookup, AppError>;

    /// Only the Gatherer id of the card in `language_code` (its page, no image download), or
    /// `None` when Gatherer has no page or no image for it.
    async fn get_gatherer_id(
        &self,
        set_code: SetCode,
        collector_number: String,
        language_code: LanguageCode,
        name: String,
    ) -> Result<Option<String>, AppError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_gatherer_miss_reads_as_a_log_reason() {
        let reasons = [
            GathererMiss::NoPage,
            GathererMiss::NoImageOnPage,
            GathererMiss::NoImage,
            GathererMiss::NoBackOnPage,
            GathererMiss::NoBack,
        ]
        .map(|miss| miss.to_string());

        assert_eq!(
            reasons,
            [
                "page not found",
                "no image on the page",
                "image not found",
                "no back on the page",
                "back image not found",
            ]
        );
    }
}
