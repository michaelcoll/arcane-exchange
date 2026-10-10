use crate::domain::card::CardId;
use crate::domain::card_image::CardImage;

/// The showcase (« Vitrine ») holds at most this many cards.
pub const SHOWCASE_SIZE: u32 = 30;

/// A card of the showcase (« Vitrine »): one of the most expensive cards held in the platform's
/// collections, shown publicly. It deliberately carries neither owner, quantity nor price.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowcaseCard {
    pub card_id: CardId,
    pub image: CardImage,
}
