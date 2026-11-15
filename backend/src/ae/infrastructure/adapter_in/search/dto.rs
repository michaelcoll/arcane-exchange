use crate::domain::search::{SearchCard, SearchSortField};
pub(crate) use crate::infrastructure::adapter_in::collection::dto::{
    PriceGuideResponse, RarityCodeParam, SortByParam, SortDirParam, card_image_urls,
};
use crate::infrastructure::adapter_in::default_page_size;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

impl From<SortByParam> for SearchSortField {
    fn from(p: SortByParam) -> Self {
        match p {
            SortByParam::Avg => SearchSortField::Avg,
            SortByParam::Trend => SearchSortField::Trend,
            SortByParam::SetCode => SearchSortField::SetCode,
            SortByParam::LanguageCode => SearchSortField::LanguageCode,
            SortByParam::AddedAt => SearchSortField::AddedAt,
        }
    }
}

/// A card a search found, reduced to what is public: no purchase price, no date of addition.
#[derive(Serialize, TS, ToSchema)]
#[serde(rename = "SearchCard")]
#[ts(export)]
pub struct SearchCardResponse {
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
    /// Number of distinct users actually offering this card to trade — after applying each
    /// owner's visibility, trade binder selection and rarity filters. A card no one offers
    /// anymore never appears in search results at all.
    pub owner_count: u64,
    /// `true` if this card is engaged in an `ONE_ACCEPTED` or `FULLY_ACCEPTED` trade of the
    /// `player_username`-filtered owner. Always `false` in the unscoped multi-owner search,
    /// where no single owner can be attributed.
    pub reserved: bool,
    #[schema(value_type = PriceGuideResponse, required = false)]
    pub price_guide: Option<PriceGuideResponse>,
}

#[derive(Serialize, TS, ToSchema)]
#[serde(rename = "PaginatedSearch")]
#[ts(export)]
pub struct PaginatedSearchResponse {
    pub items: Vec<SearchCardResponse>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}

impl From<SearchCard> for SearchCardResponse {
    fn from(c: SearchCard) -> Self {
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
            owner_count: c.owner_count,
            reserved: c.reserved,
            price_guide: c.price_guide.map(PriceGuideResponse::from),
        }
    }
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub(crate) struct SearchParams {
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
    /// Exact username of the owner to filter by (case-insensitive, no partial match)
    #[ts(optional)]
    pub(crate) player_username: Option<String>,
}

/// Scope of the search whose set facet is requested: the same `q` / `player_username` as
/// `GET /search/card`, without its filters.
#[derive(Deserialize, TS, Default)]
#[ts(export)]
pub(crate) struct SearchSetsParams {
    #[ts(optional)]
    pub(crate) q: Option<String>,
    /// Exact username of the owner to scope to (case-insensitive, no partial match)
    #[ts(optional)]
    pub(crate) player_username: Option<String>,
}

impl Default for SearchParams {
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
            player_username: None,
        }
    }
}
