use crate::domain::error::FunctionalError;
use crate::domain::language_code::LanguageCode;
use crate::domain::price::PriceGuide;
use crate::domain::rarity_code::RarityCode;
use crate::domain::set_name::{SetCode, SetName};
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CardId {
    pub set_code: SetCode,
    pub collector_number: String,
    pub language_code: LanguageCode,
    pub foil: bool,
}

impl CardId {
    pub fn try_new(
        set_code: impl Into<SetCode>,
        collector_number: impl Into<String>,
        language_code: LanguageCode,
        foil: bool,
    ) -> Result<Self, FunctionalError> {
        let collector_number = collector_number.into();
        if collector_number.chars().count() > 10 {
            return Err(FunctionalError::InvalidCollectorNumber(format!(
                "collector number must be 10 characters or less (got {})",
                collector_number
            )));
        }

        Ok(CardId {
            set_code: set_code.into(),
            collector_number,
            language_code,
            foil,
        })
    }

    pub fn new(
        set_code: impl Into<SetCode>,
        collector_number: impl Into<String>,
        language_code: LanguageCode,
        foil: bool,
    ) -> Self {
        Self::try_new(set_code, collector_number, language_code, foil)
            .expect("invalid collector number")
    }
}

impl Display for CardId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:>5} {} {} {}",
            self.collector_number,
            self.set_code,
            if self.foil { "⭑" } else { "·" },
            self.language_code,
        )
    }
}

/// A card's presence in a collection: fully visible to its owner (`Mine`),
/// reduced to just the seller's name for an individual sale listing (`Owned`,
/// used by `/card/offers`), or reduced to an aggregate owner count for public
/// search results (`Public`, used by `/search/card`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CollectionEntry {
    Mine {
        quantity: u8,
        /// Price in cents
        purchase_price: u32,
        added_at: chrono::DateTime<chrono::Utc>,
    },
    Owned {
        owner_username: String,
        quantity: u8,
        /// Price in cents, derived from the card's `trend` price guide.
        selling_price: Option<u32>,
    },
    Public {
        /// Number of distinct users owning this card.
        owner_count: u64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Card {
    pub id: CardId,
    pub set_name: SetName,
    pub name: String,
    pub rarity_code: RarityCode,
    pub collection_entry: CollectionEntry,
    pub scryfall_id: uuid::Uuid,
    pub cardmarket_id: Option<u32>,
    pub the_gatherer_id: Option<String>,
    pub price_guide: Option<PriceGuide>,
}

impl Card {
    #[allow(clippy::too_many_arguments)]
    #[allow(unused)]
    pub fn new(
        set_code: impl Into<SetCode>,
        set_name: impl Into<String>,
        collector_number: impl Into<String>,
        language_code: LanguageCode,
        foil: bool,
        name: impl Into<String>,
        rarity_code: RarityCode,
        quantity: u8,
        purchase_price: u32,
    ) -> Self {
        let set_code: SetCode = set_code.into();
        let set_name = SetName::new(set_code.clone(), set_name);
        Card {
            id: CardId::new(set_code, collector_number.into(), language_code, foil),
            set_name,
            name: name.into(),
            rarity_code,
            collection_entry: CollectionEntry::Mine {
                quantity,
                purchase_price,
                added_at: chrono::Utc::now(),
            },
            scryfall_id: uuid::Uuid::default(),
            cardmarket_id: None,
            the_gatherer_id: None,
            price_guide: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_full(
        set_code: impl Into<SetCode>,
        set_name: impl Into<String>,
        collector_number: impl Into<String>,
        language_code: LanguageCode,
        foil: bool,
        name: impl Into<String>,
        rarity_code: RarityCode,
        scryfall_id: uuid::Uuid,
        cardmarket_id: Option<u32>,
        the_gatherer_id: Option<String>,
        collection_entry: CollectionEntry,
    ) -> Self {
        let set_code: SetCode = set_code.into();
        let set_name = SetName::new(set_code.clone(), set_name);
        Card {
            id: CardId::new(set_code, collector_number.into(), language_code, foil),
            set_name,
            name: name.into(),
            rarity_code,
            collection_entry,
            scryfall_id,
            cardmarket_id,
            the_gatherer_id,
            price_guide: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardInfo {
    pub inclusion: u32,
    pub total_decks: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::language_code::LanguageCode;

    #[test]
    fn try_new_card_id_with_valid_collector_number_creates_instance() {
        let result = CardId::try_new("FDN", "1234567890", LanguageCode::EN, true);
        assert!(result.is_ok());
    }

    #[test]
    fn try_new_card_id_with_too_long_collector_number_returns_error() {
        let result = CardId::try_new("FDN", "12345678901", LanguageCode::EN, true);
        match result {
            Err(FunctionalError::InvalidCollectorNumber(msg)) => {
                assert!(msg.contains("collector number must be 10 characters or less"))
            }
            _ => panic!("Expected InvalidCollectorNumber variant"),
        }
    }

    #[test]
    #[should_panic(expected = "invalid collector number")]
    fn new_card_id_with_too_long_collector_number_panics() {
        CardId::new("FDN", "12345678901", LanguageCode::EN, true);
    }

    #[test]
    fn display_card_id_with_foil() {
        let card_id = CardId::new("FDN", "123", LanguageCode::EN, true);
        assert_eq!(card_id.to_string(), "  123 FDN ⭑ EN");
    }

    #[test]
    fn display_card_id_with_foil_and_collection_number_on_one_digit() {
        let card_id = CardId::new("FDN", "3", LanguageCode::EN, true);
        assert_eq!(card_id.to_string(), "    3 FDN ⭑ EN");
    }

    #[test]
    fn display_card_id_without_foil() {
        let card_id = CardId::new("FDN", "456", LanguageCode::FR, false);
        assert_eq!(card_id.to_string(), "  456 FDN · FR");
    }

    #[test]
    fn card_equality_same_values() {
        let card1 = Card::new(
            "ECL",
            "Lorwyn Eclipsed",
            "1",
            LanguageCode::FR,
            false,
            "Goblin Boarders",
            RarityCode::C,
            2,
            1000,
        );

        let card2 = card1.clone();
        assert_eq!(card1, card2);
    }

    #[test]
    fn card_equality_different_values() {
        let card1 = Card::new(
            "ECL",
            "Lorwyn Eclipsed",
            "1",
            LanguageCode::FR,
            false,
            "Goblin Boarders",
            RarityCode::C,
            2,
            1000,
        );

        let card2 = Card::new(
            "FND",
            "Foundations",
            "2",
            LanguageCode::FR,
            true,
            "Goblin Boarders",
            RarityCode::C,
            1,
            2000,
        );

        assert_ne!(card1, card2);
    }
}
