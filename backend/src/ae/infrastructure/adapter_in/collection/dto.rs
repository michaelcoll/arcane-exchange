use crate::domain::card::{Card, CardId, CollectionEntry};
use crate::domain::card_image::{CardFace, CardImage};
use crate::domain::card_import::{CardImport, CardImportLineError};
use crate::domain::collection::{CollectionSortField, SortDirection};
use crate::domain::collection_entry::{CardEntries, CollectionEntryDetail, EntryBinder};
use crate::domain::collection_stats::{BinderInfo, CollectionStats};
use crate::domain::price::PriceGuide;
use crate::domain::rarity_code::RarityCode;
use crate::infrastructure::adapter_in::{SetInfoResponse, default_page_size};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

#[derive(Serialize, Debug, TS, ToSchema)]
#[serde(rename = "Message")]
#[ts(export)]
pub struct MessageResponse {
    pub message: String,
}

// --- Card import ---
#[derive(Serialize, Debug, TS, ToSchema)]
#[serde(rename = "CardImportStarted")]
#[ts(export)]
pub struct CardImportStartedResponse {
    pub id: String,
}

#[derive(Serialize, Debug, TS, ToSchema)]
#[serde(rename = "CardImportLineError")]
#[ts(export)]
pub struct CardImportLineErrorResponse {
    pub line: usize,
    pub field: String,
    pub value: String,
}

impl From<CardImportLineError> for CardImportLineErrorResponse {
    fn from(e: CardImportLineError) -> Self {
        Self {
            line: e.line,
            field: e.field,
            value: e.value,
        }
    }
}

#[derive(Serialize, Debug, TS, ToSchema)]
#[serde(rename = "CardImport")]
#[ts(export)]
pub struct CardImportResponse {
    pub id: String,
    /// `pending`, `running`, `completed` or `failed`.
    pub status: String,
    /// Data lines read from the CSV, before the "Tokens" set filter and deduplication.
    pub source_lines: u32,
    /// Cards to write, after deduplication — the denominator for `processed_lines`.
    pub total_lines: u32,
    pub processed_lines: u32,
    /// Truncated to at most 100 entries; `error_count` carries the true total.
    pub errors: Vec<CardImportLineErrorResponse>,
    pub error_count: u32,
    /// Technical message of a failed import, for diagnosis only — never shown to the user.
    pub error_message: Option<String>,
    /// Stable code of a failed import's error (`no_valid_line`, `internal`, …), translated by
    /// the clients. `null` unless the import failed.
    pub error_code: Option<String>,
    /// RFC 3339 timestamp
    pub created_at: String,
    /// RFC 3339 timestamp
    pub finished_at: Option<String>,
}

impl From<CardImport> for CardImportResponse {
    fn from(import: CardImport) -> Self {
        Self {
            id: import.id.to_string(),
            status: import.status.to_string(),
            source_lines: import.source_lines,
            total_lines: import.total_lines,
            processed_lines: import.processed_lines,
            errors: import
                .line_errors
                .into_iter()
                .map(CardImportLineErrorResponse::from)
                .collect(),
            error_count: import.line_error_count,
            error_message: import.error_message,
            error_code: import.error_code,
            created_at: import.created_at.to_rfc3339(),
            finished_at: import.finished_at.map(|d| d.to_rfc3339()),
        }
    }
}

// --- Collection stats ---
#[derive(Serialize, Debug, TS, ToSchema)]
#[serde(rename = "BinderInfo")]
#[ts(export)]
pub struct BinderInfoResponse {
    pub name: String,
    pub card_count: u64,
}

impl From<BinderInfo> for BinderInfoResponse {
    fn from(b: BinderInfo) -> Self {
        Self {
            name: b.name,
            card_count: b.card_count,
        }
    }
}

#[derive(Serialize, Debug, TS, ToSchema)]
#[serde(rename = "CollectionStats")]
#[ts(export)]
pub struct CollectionStatsResponse {
    pub total_cards: u64,
    pub unique_cards: u64,
    pub price_trend_min: Option<u32>,
    pub price_trend_max: Option<u32>,
    pub sets: Vec<SetInfoResponse>,
    pub binders: Vec<BinderInfoResponse>,
}

impl From<CollectionStats> for CollectionStatsResponse {
    fn from(s: CollectionStats) -> Self {
        Self {
            total_cards: s.total_cards,
            unique_cards: s.unique_cards,
            price_trend_min: s.price_trend_min.value,
            price_trend_max: s.price_trend_max.value,
            sets: s.sets.into_iter().map(SetInfoResponse::from).collect(),
            binders: s
                .binders
                .into_iter()
                .map(BinderInfoResponse::from)
                .collect(),
        }
    }
}

// --- Query params ---
#[derive(Deserialize, Default, Debug, PartialEq, TS, ToSchema)]
#[serde(rename = "SortBy", rename_all = "snake_case")]
#[ts(export)]
pub enum SortByParam {
    Avg,
    #[default]
    Trend,
    SetCode,
    LanguageCode,
    AddedAt,
}

#[derive(Deserialize, Default, Debug, PartialEq, TS, ToSchema)]
#[serde(rename = "SortDir", rename_all = "snake_case")]
#[ts(export)]
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
            SortByParam::AddedAt => CollectionSortField::AddedAt,
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
#[ts(export)]
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

/// The URLs of the front and back images of `card_id`, served by the frontend under
/// `/card-images`, both `None` while the image is pending and the back `None` for a single-faced
/// card. They are versioned by the origin of the file, which changes whenever the file is
/// replaced, so they can be cached as immutable; the cards sharing an image share its URLs.
pub(crate) fn card_image_urls(
    card_id: &CardId,
    image: Option<CardImage>,
) -> (Option<String>, Option<String>) {
    let url = |face| {
        let image = image?;
        let file_name = image.file_name(card_id, face)?;
        Some(format!(
            "/card-images/{file_name}?v={}",
            image.source.origin().url_version()
        ))
    };
    (url(CardFace::Front), url(CardFace::Back))
}

#[derive(Deserialize, TS)]
#[ts(export)]
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
#[ts(export)]
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

// --- Collection entries of a card ---
#[derive(Serialize, Debug, TS, ToSchema)]
#[serde(rename = "EntryBinder")]
#[ts(export)]
pub struct EntryBinderResponse {
    pub id: String,
    pub name: String,
}

impl From<EntryBinder> for EntryBinderResponse {
    fn from(b: EntryBinder) -> Self {
        Self {
            id: b.id.to_string(),
            name: b.name,
        }
    }
}

/// One Collection entry of the caller: the copies of a card in one finish and one binder.
#[derive(Serialize, Debug, TS, ToSchema)]
#[serde(rename = "CollectionEntryDetail")]
#[ts(export)]
pub struct CollectionEntryDetailResponse {
    /// Designates the entry in `PATCH` and `DELETE /collection/entries/{id}`.
    pub id: String,
    pub foil: bool,
    /// Absent for an entry stored in no binder.
    #[schema(value_type = EntryBinderResponse, required = false)]
    pub binder: Option<EntryBinderResponse>,
    pub quantity: u8,
    /// Price in cents; `null` when the copies have no purchase price.
    pub purchase_price: Option<u32>,
    /// RFC 3339 timestamp
    pub added_at: String,
}

impl From<CollectionEntryDetail> for CollectionEntryDetailResponse {
    fn from(e: CollectionEntryDetail) -> Self {
        Self {
            id: e.id.to_string(),
            foil: e.foil,
            binder: e.binder.map(EntryBinderResponse::from),
            quantity: e.quantity,
            purchase_price: e.purchase_price,
            added_at: e.added_at.to_rfc3339(),
        }
    }
}

/// Copies committed by the accepted Trades, per finish.
#[derive(Serialize, Debug, TS, ToSchema)]
#[serde(rename = "ReservedQuantity")]
#[ts(export)]
pub struct ReservedQuantityResponse {
    pub normal: u32,
    pub foil: u32,
}

/// What the caller owns of a card in both finishes, with what the accepted Trades committed.
#[derive(Serialize, Debug, TS, ToSchema)]
#[serde(rename = "CardEntries")]
#[ts(export)]
pub struct CardEntriesResponse {
    pub entries: Vec<CollectionEntryDetailResponse>,
    /// The owned quantity of a finish, all binders together, cannot go under its reserved
    /// quantity.
    pub reserved_quantity: ReservedQuantityResponse,
}

impl From<CardEntries> for CardEntriesResponse {
    fn from(c: CardEntries) -> Self {
        Self {
            entries: c
                .entries
                .into_iter()
                .map(CollectionEntryDetailResponse::from)
                .collect(),
            reserved_quantity: ReservedQuantityResponse {
                normal: c.reserved_normal,
                foil: c.reserved_foil,
            },
        }
    }
}

/// A copy to add to the collection, without purchase price.
#[derive(Deserialize, Debug, TS, ToSchema)]
#[serde(rename = "AddCollectionEntryRequest")]
#[ts(export)]
pub(crate) struct AddCollectionEntryRequest {
    /// Set code of the catalog card.
    pub(crate) set_code: String,
    /// Collector number of the catalog card.
    pub(crate) collector_number: String,
    /// Language of the copy: not checked against Scryfall, which may not have the card in it.
    pub(crate) language_code: String,
    pub(crate) foil: bool,
    /// Copies added, 1 to 255. Signed so an out-of-range value fails with `invalid_quantity`
    /// (400) instead of a JSON extractor rejection.
    pub(crate) quantity: i32,
    /// Binder to store the copies in; absent for none.
    #[serde(default)]
    #[ts(optional)]
    pub(crate) binder_id: Option<String>,
}

#[derive(Deserialize, Debug, TS, ToSchema)]
#[serde(rename = "UpdateCollectionEntryRequest")]
#[ts(export)]
pub(crate) struct UpdateCollectionEntryRequest {
    /// New quantity, 1 to 255. Signed so an out-of-range value fails with `invalid_quantity`
    /// (400) instead of a JSON extractor rejection.
    pub(crate) quantity: Option<i32>,
    /// New finish. A change onto an existing entry merges the two.
    pub(crate) foil: Option<bool>,
}

#[derive(Deserialize, Debug, TS, ToSchema)]
#[serde(rename = "MoveCollectionEntryRequest")]
#[ts(export)]
pub(crate) struct MoveCollectionEntryRequest {
    /// Target binder; `null` or absent moves the copies out of any binder.
    pub(crate) binder_id: Option<String>,
    /// Copies to move, 1 up to the entry's quantity; all of them when absent. Signed so an
    /// out-of-range value fails with `invalid_quantity` (400) instead of a JSON extractor
    /// rejection.
    pub(crate) quantity: Option<i32>,
}

#[derive(Serialize, TS, ToSchema)]
#[serde(rename = "CollectionEntry")]
#[ts(export)]
pub struct CollectionEntryResponse {
    pub quantity: u8,
    /// Price in cents; `null` when the copies have no purchase price.
    pub purchase_price: Option<u32>,
    /// RFC 3339 timestamp
    pub added_at: String,
}

#[derive(Serialize, TS, ToSchema)]
#[serde(rename = "CollectionCard")]
#[ts(export)]
pub struct CollectionCardResponse {
    pub set_code: String,
    pub collector_number: String,
    pub language_code: String,
    pub foil: bool,
    pub name: String,
    pub rarity_code: String,
    pub scryfall_id: String,
    /// Front image, relative to the frontend (e.g. `/card-images/FDN_1_EN.webp?v=gatherer`);
    /// `null` while the card's image is pending.
    pub image_url: Option<String>,
    /// Back image of a double-faced card, like `image_url`; `null` for a single-faced card.
    pub image_back_url: Option<String>,
    /// Present only when the card is owned by the authenticated user.
    #[schema(value_type = CollectionEntryResponse, required = false)]
    pub collection_entry: Option<CollectionEntryResponse>,
    /// Number of distinct users actually offering this card to trade (search mode only) —
    /// after applying each owner's visibility, trade binder selection and rarity filters. A
    /// card no one offers anymore never appears in search results at all.
    pub owner_count: Option<u64>,
    /// `true` if this card is engaged in an `ONE_ACCEPTED` or `FULLY_ACCEPTED` trade of its
    /// owner (the authenticated user in collection mode, or the `player_username`-filtered
    /// owner in search mode). Always `false` in the unscoped multi-owner search, where no
    /// single owner can be attributed.
    pub reserved: bool,
    #[schema(value_type = PriceGuideResponse, required = false)]
    pub price_guide: Option<PriceGuideResponse>,
}

#[derive(Serialize, TS, ToSchema)]
#[serde(rename = "PaginatedCollection")]
#[ts(export)]
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
            CollectionEntry::Public {
                owner_count,
                reserved,
            } => (None, Some(owner_count), reserved),
            CollectionEntry::Owned { .. } => unreachable!(
                "Card.collection_entry never carries CollectionEntry::Owned; that variant is reserved for /card/offers"
            ),
        };

        let (image_url, image_back_url) = card_image_urls(&c.id.card_id, c.image);
        Self {
            set_code: c.id.card_id.set_code.to_string(),
            collector_number: c.id.card_id.collector_number,
            language_code: c.id.card_id.language_code.to_string(),
            foil: c.id.foil,
            name: c.name,
            rarity_code: c.rarity_code.to_string(),
            scryfall_id: c.scryfall_id.to_string(),
            image_url,
            image_back_url,
            collection_entry,
            owner_count,
            reserved,
            price_guide: c.price_guide.map(PriceGuideResponse::from),
        }
    }
}
