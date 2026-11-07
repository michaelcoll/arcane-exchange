use crate::domain::rarity_code::RarityCode;
use crate::domain::set_name::SetCode;
use serde::Serialize;
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

/// What the scan index keeps of a catalog card: enough for the iOS scanner to recognize a card
/// offline. Its serialization is the wire shape of one entry of the index (the French name is left
/// out when Scryfall has none).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ScanIndexEntry {
    pub set_code: SetCode,
    /// Purely numeric, as in the catalog (no leading zeros).
    pub collector_number: String,
    pub rarity: RarityCode,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub french_name: Option<String>,
}

/// The published scan index: its JSON, gzipped, and the ETag derived from its content.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanIndex {
    /// Quoted entity tag (`"<hash>"`), the same for the same content.
    pub etag: String,
    pub gzipped_json: Vec<u8>,
}
