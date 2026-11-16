use crate::domain::search::{Offer, Possession, SearchCard, SearchImage};
pub(crate) use crate::infrastructure::adapter_in::collection::dto::{
    PriceGuideResponse, RarityCodeParam, SortDirParam, card_image_urls,
};
use crate::infrastructure::adapter_in::default_page_size;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

/// The Possession axis of the query: `owned=true` keeps the owned impressions, `owned=false` the
/// others, no `owned` all of them.
pub(crate) fn possession_of(owned: Option<bool>) -> Possession {
    match owned {
        Some(true) => Possession::Owned,
        Some(false) => Possession::NotOwned,
        None => Possession::Any,
    }
}

/// The Offre axis of the query: `offered_by` wins when it names a player, `offered` otherwise.
/// A blank username means no player.
pub(crate) fn offer_of(offered: Option<bool>, offered_by: Option<&str>) -> Offer {
    let player = offered_by.map(str::trim).filter(|s| !s.is_empty());
    match (player, offered) {
        (Some(username), _) => Offer::OfferedBy(username.to_string()),
        (None, Some(true)) => Offer::Offered,
        (None, _) => Offer::Any,
    }
}

/// An impression a search found (set and number, every language and finish together), reduced to
/// what is public: no purchase price, no date of addition, nothing of another player.
#[derive(Serialize, TS, ToSchema)]
#[serde(rename = "SearchCard")]
#[ts(export)]
pub struct SearchCardResponse {
    pub set_code: String,
    pub set_name: String,
    pub collector_number: String,
    pub rarity_code: String,
    pub scryfall_id: String,
    /// The English name.
    pub name: String,
    /// The French name, absent when unknown. The client chooses which name to show.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub french_name: Option<String>,
    /// Front image: relative to the frontend (e.g. `/card-images/FDN_1_EN.webp?v=gatherer`) when
    /// a Card of the impression has a downloaded one (French first, then English), the Scryfall
    /// URL of the Catalogue otherwise; `null` when there is none.
    pub image_url: Option<String>,
    /// Back image of a double-faced card, like `image_url` for a downloaded image; `null` for a
    /// single-faced card, or with a Scryfall image.
    pub image_back_url: Option<String>,
    /// Number of distinct players offering this impression to trade — after applying each
    /// player's visibility, trade binder selection and rarity filters — the authenticated one
    /// included. `0` when nobody does.
    pub owner_count: u64,
    /// Copies of this impression the authenticated player owns, every language and finish
    /// together; `0` when they own none. Never another player's.
    pub owned_quantity: u64,
    /// Language codes (`EN`, `FR`...) in which this impression is offered for trade: by the
    /// `offered_by` player when there is one, by any player otherwise. Empty when nobody offers
    /// it.
    pub offered_languages: Vec<String>,
    /// With `offered_by` only: `true` if one of the Cards of this impression of that player is
    /// engaged in an `ONE_ACCEPTED` or `FULLY_ACCEPTED` trade. Absent otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub reserved: Option<bool>,
    /// The normal Cardmarket price guide; `null` when there is no normal price.
    #[schema(value_type = PriceGuideResponse, required = false)]
    pub price_guide: Option<PriceGuideResponse>,
    /// The foil Cardmarket price guide; `null` when there is no foil price.
    #[schema(value_type = PriceGuideResponse, required = false)]
    pub foil_price_guide: Option<PriceGuideResponse>,
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
        let (image_url, image_back_url) = match c.image {
            Some(SearchImage::Local { card_id, image }) => card_image_urls(&card_id, Some(image)),
            Some(SearchImage::Scryfall(url)) => (Some(url), None),
            None => (None, None),
        };
        Self {
            set_code: c.set_name.code.to_string(),
            set_name: c.set_name.name,
            collector_number: c.collector_number,
            rarity_code: c.rarity_code.to_string(),
            scryfall_id: c.scryfall_id.to_string(),
            name: c.name,
            french_name: c.french_name,
            image_url,
            image_back_url,
            owner_count: c.owner_count,
            owned_quantity: c.owned_quantity,
            offered_languages: c
                .offered_languages
                .iter()
                .map(ToString::to_string)
                .collect(),
            reserved: c.reserved,
            price_guide: c.price_guide.map(PriceGuideResponse::from),
            foil_price_guide: c.foil_price_guide.map(PriceGuideResponse::from),
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
    pub(crate) sort_dir: SortDirParam,
    #[ts(optional)]
    pub(crate) q: Option<String>,
    /// Possession axis: `true` for the impressions the authenticated player owns, `false` for
    /// those they do not own, absent for all of them
    #[ts(optional)]
    pub(crate) owned: Option<bool>,
    /// Only the impressions offered by any player, the authenticated one included
    #[ts(optional)]
    pub(crate) offered: Option<bool>,
    /// Only the impressions offered by this player: exact username, case-insensitive. Wins over
    /// `offered`.
    #[ts(optional)]
    pub(crate) offered_by: Option<String>,
    /// Rarity codes, repeated for multiple values (e.g. `?rarity=C&rarity=U`)
    #[serde(default)]
    pub(crate) rarity: Vec<RarityCodeParam>,
    /// Comma-separated set codes
    #[ts(optional)]
    pub(crate) sets: Option<String>,
    /// Exact collector number (`1` does not match `10` or `1a`)
    #[ts(optional)]
    pub(crate) collector_number: Option<String>,
    /// Minimum reference price in cents
    #[ts(optional)]
    pub(crate) price_min: Option<u32>,
    /// Maximum reference price in cents
    #[ts(optional)]
    pub(crate) price_max: Option<u32>,
}

/// Scope of the search whose set facet is requested: the same `q` / `owned` / `offered` /
/// `offered_by` as `GET /search/card`, without its filters.
#[derive(Deserialize, TS, Default)]
#[ts(export)]
pub(crate) struct SearchSetsParams {
    #[ts(optional)]
    pub(crate) q: Option<String>,
    /// Possession axis: `true` for the impressions the authenticated player owns, `false` for
    /// those they do not own, absent for all of them
    #[ts(optional)]
    pub(crate) owned: Option<bool>,
    /// Only the impressions offered by any player
    #[ts(optional)]
    pub(crate) offered: Option<bool>,
    /// Only the impressions offered by this player: exact username, case-insensitive. Wins over
    /// `offered`.
    #[ts(optional)]
    pub(crate) offered_by: Option<String>,
}

impl Default for SearchParams {
    fn default() -> Self {
        Self {
            page: 0,
            page_size: default_page_size(),
            sort_dir: SortDirParam::default(),
            q: None,
            owned: None,
            offered: None,
            offered_by: None,
            rarity: Vec::new(),
            sets: None,
            collector_number: None,
            price_min: None,
            price_max: None,
        }
    }
}
