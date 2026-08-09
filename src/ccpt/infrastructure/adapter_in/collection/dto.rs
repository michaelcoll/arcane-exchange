use crate::domain::card::{Card, CollectionEntry};
use crate::domain::collection::{CollectionSortField, SortDirection};
use crate::domain::collection_stats::CollectionStats;
use crate::domain::price::PriceGuide;
use crate::domain::rarity_code::RarityCode;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

#[derive(Serialize, Debug, TS, ToSchema)]
#[serde(rename = "Message")]
#[ts(export, export_to = "Message.ts")]
pub struct MessageResponse {
    pub message: String,
}

// --- Collection stats ---
#[derive(Serialize, Debug, TS, ToSchema)]
#[serde(rename = "SetInfo")]
#[ts(export, export_to = "SetInfo.ts")]
pub struct SetInfoResponse {
    pub code: String,
    pub name: String,
}

#[derive(Serialize, Debug, TS, ToSchema)]
#[serde(rename = "CollectionStats")]
#[ts(export, export_to = "CollectionStats.ts")]
pub struct CollectionStatsResponse {
    pub total_cards: u64,
    pub unique_cards: u64,
    pub price_trend_min: Option<u32>,
    pub price_trend_max: Option<u32>,
    pub sets: Vec<SetInfoResponse>,
}

impl From<CollectionStats> for CollectionStatsResponse {
    fn from(s: CollectionStats) -> Self {
        Self {
            total_cards: s.total_cards,
            unique_cards: s.unique_cards,
            price_trend_min: s.price_trend_min.value,
            price_trend_max: s.price_trend_max.value,
            sets: s
                .sets
                .into_iter()
                .map(|sn| SetInfoResponse {
                    code: sn.code.to_string(),
                    name: sn.name,
                })
                .collect(),
        }
    }
}

// --- Query params ---
#[derive(Deserialize, Default, Debug, PartialEq, TS, ToSchema)]
#[serde(rename = "SortBy", rename_all = "snake_case")]
#[ts(export, export_to = "SortBy.ts")]
pub enum SortByParam {
    Avg,
    #[default]
    Trend,
    SetCode,
    LanguageCode,
}

#[derive(Deserialize, Default, Debug, PartialEq, TS, ToSchema)]
#[serde(rename = "SortDir", rename_all = "snake_case")]
#[ts(export, export_to = "SortDir.ts")]
pub enum SortDirParam {
    Asc,
    #[default]
    Desc,
}

impl From<SortByParam> for CollectionSortField {
    fn from(p: SortByParam) -> Self {
        match p {
            SortByParam::Avg => CollectionSortField::Avg,
            SortByParam::Trend => CollectionSortField::Trend,
            SortByParam::SetCode => CollectionSortField::SetCode,
            SortByParam::LanguageCode => CollectionSortField::LanguageCode,
        }
    }
}

impl From<SortDirParam> for SortDirection {
    fn from(p: SortDirParam) -> Self {
        match p {
            SortDirParam::Asc => SortDirection::Asc,
            SortDirParam::Desc => SortDirection::Desc,
        }
    }
}

#[derive(Deserialize, Debug, PartialEq, TS, ToSchema)]
#[serde(rename = "RarityCode")]
#[ts(export, export_to = "RarityCode.ts")]
pub enum RarityCodeParam {
    C,
    U,
    R,
    M,
    S,
}

impl From<RarityCodeParam> for RarityCode {
    fn from(p: RarityCodeParam) -> Self {
        match p {
            RarityCodeParam::C => RarityCode::C,
            RarityCodeParam::U => RarityCode::U,
            RarityCodeParam::R => RarityCode::R,
            RarityCodeParam::M => RarityCode::M,
            RarityCodeParam::S => RarityCode::S,
        }
    }
}

pub(crate) fn default_page_size() -> u32 {
    20
}

#[derive(Deserialize, TS)]
#[ts(export, export_to = "CollectionParams.ts")]
pub(crate) struct CollectionParams {
    #[serde(default)]
    pub(crate) page: u32,
    #[serde(default = "default_page_size")]
    pub(crate) page_size: u32,
    #[serde(default)]
    pub(crate) sort_by: SortByParam,
    #[serde(default)]
    pub(crate) sort_dir: SortDirParam,
    #[ts(optional)]
    pub(crate) q: Option<String>,
    /// Rarity codes, repeated for multiple values (e.g. `?rarity=C&rarity=U`)
    #[serde(default)]
    pub(crate) rarity: Vec<RarityCodeParam>,
    /// Comma-separated set codes
    #[ts(optional)]
    pub(crate) sets: Option<String>,
    /// Minimum trend price in cents
    #[ts(optional)]
    pub(crate) price_min: Option<u32>,
    /// Maximum trend price in cents
    #[ts(optional)]
    pub(crate) price_max: Option<u32>,
}

impl Default for CollectionParams {
    fn default() -> Self {
        Self {
            page: 0,
            page_size: default_page_size(),
            sort_by: SortByParam::default(),
            sort_dir: SortDirParam::default(),
            q: None,
            rarity: Vec::new(),
            sets: None,
            price_min: None,
            price_max: None,
        }
    }
}

// --- Réponses ---
#[derive(Serialize, TS, ToSchema)]
#[serde(rename = "PriceGuide")]
#[ts(export, export_to = "PriceGuide.ts")]
pub struct PriceGuideResponse {
    pub low: Option<u32>,
    pub avg: Option<u32>,
    pub trend: Option<u32>,
}

impl From<PriceGuide> for PriceGuideResponse {
    fn from(pg: PriceGuide) -> Self {
        Self {
            low: pg.low.value,
            avg: pg.avg.value,
            trend: pg.trend.value,
        }
    }
}

#[derive(Serialize, TS, ToSchema)]
#[serde(rename = "CollectionEntry")]
#[ts(export, export_to = "CollectionEntry.ts")]
pub struct CollectionEntryResponse {
    pub quantity: u8,
    pub purchase_price: u32,
    /// RFC 3339 timestamp
    pub added_at: String,
}

#[derive(Serialize, TS, ToSchema)]
#[serde(rename = "CollectionCard")]
#[ts(export, export_to = "CollectionCard.ts")]
pub struct CollectionCardResponse {
    pub set_code: String,
    pub collector_number: String,
    pub language_code: String,
    pub foil: bool,
    pub name: String,
    pub rarity_code: String,
    pub scryfall_id: String,
    pub the_gatherer_id: Option<String>,
    /// Present only when the card is owned by the authenticated user.
    pub collection_entry: Option<CollectionEntryResponse>,
    /// Number of distinct users owning this card (search mode only).
    pub owner_count: Option<u64>,
    /// `true` if this card is engaged in one of the authenticated user's trades in
    /// `ONE_ACCEPTED` or `FULLY_ACCEPTED` status. Always `false` in search mode.
    pub reserved: bool,
    pub price_guide: Option<PriceGuideResponse>,
}

#[derive(Serialize, TS, ToSchema)]
#[serde(rename = "PaginatedCollection")]
#[ts(export, export_to = "PaginatedCollection.ts")]
pub struct PaginatedCollectionResponse {
    pub items: Vec<CollectionCardResponse>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}

impl From<Card> for CollectionCardResponse {
    fn from(c: Card) -> Self {
        let (collection_entry, owner_count, reserved) = match c.collection_entry {
            CollectionEntry::Mine {
                quantity,
                purchase_price,
                added_at,
                reserved,
            } => (
                Some(CollectionEntryResponse {
                    quantity,
                    purchase_price,
                    added_at: added_at.to_rfc3339(),
                }),
                None,
                reserved,
            ),
            CollectionEntry::Public { owner_count } => (None, Some(owner_count), false),
            CollectionEntry::Owned { .. } => unreachable!(
                "Card.collection_entry never carries CollectionEntry::Owned; that variant is reserved for /card/offers"
            ),
        };

        Self {
            set_code: c.id.set_code.to_string(),
            collector_number: c.id.collector_number,
            language_code: c.id.language_code.to_string(),
            foil: c.id.foil,
            name: c.name,
            rarity_code: c.rarity_code.to_string(),
            scryfall_id: c.scryfall_id.to_string(),
            the_gatherer_id: c.the_gatherer_id,
            collection_entry,
            owner_count,
            reserved,
            price_guide: c.price_guide.map(PriceGuideResponse::from),
        }
    }
}
