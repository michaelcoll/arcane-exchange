use crate::domain::card::CollectionEntry;
use crate::domain::card_offer::CardOfferSortField;
use crate::infrastructure::adapter_in::collection::dto::default_page_size;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

// --- Price history ---
/// Shared by `/collection/price-history` and `/card/{scryfall_id}/price-history`; both dates are
/// optional and defaulted by the use case (last 30 days when absent).
#[derive(Deserialize, TS)]
#[ts(export, export_to = "PriceHistoryParams.ts")]
pub(crate) struct PriceHistoryParams {
    /// ISO 8601 date string (YYYY-MM-DD)
    #[ts(optional, type = "string")]
    pub(crate) start_date: Option<NaiveDate>,
    /// ISO 8601 date string (YYYY-MM-DD)
    #[ts(optional, type = "string")]
    pub(crate) end_date: Option<NaiveDate>,
}

#[derive(Serialize, Debug, TS, ToSchema)]
#[serde(rename = "PriceHistoryEntry")]
#[ts(export, export_to = "PriceHistoryEntry.ts")]
pub struct PriceHistoryEntryResponse {
    /// ISO 8601 date string (YYYY-MM-DD)
    pub date: String,
    pub low: i64,
    pub trend: i64,
    pub avg: i64,
}

// --- Offers ---

#[derive(Deserialize, Default, TS, ToSchema)]
#[serde(rename = "CardOffersSortBy", rename_all = "snake_case")]
#[ts(export, export_to = "CardOffersSortBy.ts")]
pub(crate) enum CardOffersSortByParam {
    #[default]
    SellingPrice,
}

impl From<CardOffersSortByParam> for CardOfferSortField {
    fn from(p: CardOffersSortByParam) -> Self {
        match p {
            CardOffersSortByParam::SellingPrice => CardOfferSortField::SellingPrice,
        }
    }
}

#[derive(Deserialize, TS)]
#[ts(export, export_to = "CardOffersParams.ts")]
pub(crate) struct CardOffersParams {
    pub(crate) set_code: String,
    pub(crate) collector_number: String,
    pub(crate) language_code: String,
    pub(crate) foil: bool,
    #[serde(default)]
    pub(crate) sort_by: CardOffersSortByParam,
    #[serde(default)]
    pub(crate) page: u32,
    #[serde(default = "default_page_size")]
    pub(crate) page_size: u32,
}

#[derive(Serialize, Debug, TS, ToSchema)]
#[serde(rename = "CardOffer")]
#[ts(export, export_to = "CardOffer.ts")]
pub struct CardOfferResponse {
    pub owner_username: String,
    /// Quantity this owner actually offers to trade — after applying their collection
    /// visibility, trade binder selection and rarity filters. Never the raw quantity they own:
    /// an owner offering `0` never appears in the list.
    pub quantity: u8,
    pub selling_price: Option<u32>,
    /// `true` if this exact copy is engaged in one of the owner's trades in `ONE_ACCEPTED` or
    /// `FULLY_ACCEPTED` status — it cannot be traded away right now.
    pub reserved: bool,
}

impl From<CollectionEntry> for CardOfferResponse {
    fn from(entry: CollectionEntry) -> Self {
        match entry {
            CollectionEntry::Owned {
                owner_username,
                quantity,
                selling_price,
                reserved,
            } => Self {
                owner_username,
                quantity,
                selling_price,
                reserved,
            },
            CollectionEntry::Mine { .. } => {
                unreachable!("get_offers only ever returns CollectionEntry::Owned entries")
            }
            CollectionEntry::Public { .. } => {
                unreachable!("get_offers only ever returns CollectionEntry::Owned entries")
            }
        }
    }
}

#[derive(Serialize, Debug, TS, ToSchema)]
#[serde(rename = "PaginatedCardOffers")]
#[ts(export, export_to = "PaginatedCardOffers.ts")]
pub struct PaginatedCardOffersResponse {
    pub items: Vec<CardOfferResponse>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}
