use crate::domain::rarity_code::RarityCode;
use crate::domain::set_name::SetCode;
use uuid::Uuid;

/// One card of the catalog, keyed by set and collector number: no language, no finish.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogCard {
    pub set_code: SetCode,
    /// As Scryfall gives it, without leading zeros (`12`, `H13`, `★`).
    pub collector_number: String,
    pub rarity: RarityCode,
    pub name: String,
    pub french_name: Option<String>,
    /// The Scryfall id of the reference object (the English one when there is one).
    pub scryfall_id: Uuid,
    pub cardmarket_id: Option<u32>,
}
