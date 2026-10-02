use crate::domain::rarity_trade_filter::RarityTradeFilter;
use crate::domain::user::CollectionVisibility;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

// --- Collection visibility ---
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[serde(rename = "CollectionVisibility", rename_all = "snake_case")]
#[ts(export)]
pub enum CollectionVisibilityParam {
    Public,
    Trade,
    Private,
}

impl From<CollectionVisibilityParam> for CollectionVisibility {
    fn from(p: CollectionVisibilityParam) -> Self {
        match p {
            CollectionVisibilityParam::Public => CollectionVisibility::Public,
            CollectionVisibilityParam::Trade => CollectionVisibility::Trade,
            CollectionVisibilityParam::Private => CollectionVisibility::Private,
        }
    }
}

impl From<CollectionVisibility> for CollectionVisibilityParam {
    fn from(v: CollectionVisibility) -> Self {
        match v {
            CollectionVisibility::Public => CollectionVisibilityParam::Public,
            CollectionVisibility::Trade => CollectionVisibilityParam::Trade,
            CollectionVisibility::Private => CollectionVisibilityParam::Private,
        }
    }
}

#[derive(Debug, Serialize, TS, ToSchema)]
#[serde(rename = "VisibilityResponse")]
#[ts(export)]
pub struct VisibilityResponse {
    pub visibility: CollectionVisibilityParam,
}

#[derive(Debug, Deserialize, TS, ToSchema)]
#[ts(export)]
pub(crate) struct SetVisibilityRequest {
    pub(crate) visibility: CollectionVisibilityParam,
}

// --- Binders open for trade ---
#[derive(Debug, Serialize, TS, ToSchema)]
#[ts(export)]
pub struct TradeBindersResponse {
    pub binders: Vec<String>,
}

#[derive(Debug, Deserialize, TS, ToSchema)]
#[ts(export)]
pub(crate) struct AddTradeBinderRequest {
    pub(crate) binder_name: String,
}

// --- Rarity trade filters ---
#[derive(Serialize, Debug, TS, ToSchema)]
#[serde(rename = "RarityFilter")]
#[ts(export)]
pub struct RarityFilterResponse {
    pub rarity: String,
    pub is_open: bool,
    pub kept_copies: u8,
    pub copies: u64,
    pub proposed: u64,
}

impl From<RarityTradeFilter> for RarityFilterResponse {
    fn from(f: RarityTradeFilter) -> Self {
        Self {
            rarity: f.rarity.to_string(),
            is_open: f.is_open,
            kept_copies: f.kept_copies,
            copies: f.copies,
            proposed: f.proposed,
        }
    }
}

#[derive(Serialize, Debug, TS, ToSchema)]
#[serde(rename = "RarityFilters")]
#[ts(export)]
pub struct RarityFiltersResponse {
    pub rarities: Vec<RarityFilterResponse>,
}

#[derive(Deserialize, Debug, TS, ToSchema)]
#[ts(export)]
pub(crate) struct SetRarityFilterRequest {
    pub(crate) rarity: String,
    pub(crate) is_open: bool,
    /// Signed so out-of-range values (e.g. negative) fail domain validation with a clean 400
    /// instead of being rejected by the JSON extractor as a 422 before reaching the handler.
    pub(crate) kept_copies: i32,
}
