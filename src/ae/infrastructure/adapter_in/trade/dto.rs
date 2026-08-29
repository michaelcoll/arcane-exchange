use crate::domain::trade::{
    TradeCardDetail, TradeDetail, TradePartyState, TradeStatus, TradeSummary,
};
use crate::infrastructure::adapter_in::collection::dto::{PriceGuideResponse, default_page_size};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

#[derive(Deserialize, TS, ToSchema)]
#[ts(export, export_to = "CreateTradeRequest.ts")]
pub(crate) struct CreateTradeRequest {
    pub(crate) respondent_username: String,
}

#[derive(Serialize, TS, ToSchema)]
#[serde(rename = "CreateTradeResponse")]
#[ts(export, export_to = "CreateTradeResponse.ts")]
pub struct CreateTradeResponse {
    pub id: String,
}

#[derive(Deserialize, TS, ToSchema)]
#[ts(export, export_to = "AddTradeCardRequest.ts")]
pub(crate) struct AddTradeCardRequest {
    pub(crate) set_code: String,
    pub(crate) collector_number: String,
    pub(crate) language_code: String,
    pub(crate) foil: bool,
    pub(crate) owner_username: String,
    pub(crate) quantity: u8,
}

#[derive(Deserialize, TS, ToSchema)]
#[ts(export, export_to = "RemoveTradeCardRequest.ts")]
pub(crate) struct RemoveTradeCardRequest {
    pub(crate) set_code: String,
    pub(crate) collector_number: String,
    pub(crate) language_code: String,
    pub(crate) foil: bool,
    pub(crate) owner_username: String,
}

#[derive(Deserialize, TS, ToSchema)]
#[ts(export, export_to = "RateTradeRequest.ts")]
pub(crate) struct RateTradeRequest {
    /// Rating given to the other party, from 0 to 5 inclusive.
    pub(crate) rating: u8,
}

/// One card offered in a trade, from either side. Which side it's on is conveyed by which of
/// `TradeDetailResponse::my_cards`/`partner_cards` it's found in, not by a field here.
#[derive(Serialize, TS, ToSchema)]
#[serde(rename = "TradeCard")]
#[ts(export, export_to = "TradeCard.ts")]
pub struct TradeCardResponse {
    pub set_code: String,
    pub collector_number: String,
    pub language_code: String,
    pub foil: bool,
    pub name: String,
    pub quantity: u32,
    #[schema(value_type = PriceGuideResponse, required = false)]
    pub price_guide: Option<PriceGuideResponse>,
    pub scryfall_id: String,
    pub the_gatherer_id: Option<String>,
}

impl From<TradeCardDetail> for TradeCardResponse {
    fn from(c: TradeCardDetail) -> Self {
        Self {
            set_code: c.card_id.set_code.to_string(),
            collector_number: c.card_id.collector_number,
            language_code: c.card_id.language_code.to_string(),
            foil: c.card_id.foil,
            name: c.name,
            quantity: c.quantity,
            price_guide: c.price_guide.map(PriceGuideResponse::from),
            scryfall_id: c.scryfall_id.to_string(),
            the_gatherer_id: c.the_gatherer_id,
        }
    }
}

/// One party's acceptance/confirmation/rating state, already resolved from the caller's point
/// of view (`me` vs `partner`) — see `TradeDetailResponse`.
#[derive(Serialize, Debug, TS, ToSchema)]
#[serde(rename = "TradePartyState")]
#[ts(export, export_to = "TradePartyState.ts")]
pub struct TradePartyStateResponse {
    pub accepted: bool,
    pub confirmed: bool,
    pub rating: Option<u8>,
}

impl From<TradePartyState> for TradePartyStateResponse {
    fn from(s: TradePartyState) -> Self {
        Self {
            accepted: s.accepted,
            confirmed: s.confirmed,
            rating: s.rating,
        }
    }
}

#[derive(Serialize, TS, ToSchema)]
#[serde(rename = "TradeDetail")]
#[ts(export, export_to = "TradeDetail.ts")]
pub struct TradeDetailResponse {
    pub id: String,
    pub status: String,
    pub partner_username: String,
    pub my_cards: Vec<TradeCardResponse>,
    pub partner_cards: Vec<TradeCardResponse>,
    pub me: TradePartyStateResponse,
    pub partner: TradePartyStateResponse,
}

impl From<TradeDetail> for TradeDetailResponse {
    fn from(d: TradeDetail) -> Self {
        Self {
            id: d.id.to_string(),
            status: d.status.as_db_str().to_string(),
            partner_username: d.partner_username,
            my_cards: d
                .my_cards
                .into_iter()
                .map(TradeCardResponse::from)
                .collect(),
            partner_cards: d
                .partner_cards
                .into_iter()
                .map(TradeCardResponse::from)
                .collect(),
            me: d.me.into(),
            partner: d.partner.into(),
        }
    }
}

#[derive(Serialize, Debug, TS, ToSchema)]
#[serde(rename = "TradeSummary")]
#[ts(export, export_to = "TradeSummary.ts")]
pub struct TradeSummaryResponse {
    pub id: String,
    pub status: String,
    pub partner_username: String,
    pub my_card_count: u32,
    pub partner_card_count: u32,
    /// RFC 3339 timestamp
    pub updated_at: String,
}

impl From<TradeSummary> for TradeSummaryResponse {
    fn from(s: TradeSummary) -> Self {
        Self {
            id: s.id.to_string(),
            status: s.status.as_db_str().to_string(),
            partner_username: s.partner_username,
            my_card_count: s.my_card_count,
            partner_card_count: s.partner_card_count,
            updated_at: s.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Serialize, Debug, TS, ToSchema)]
#[serde(rename = "PaginatedTrades")]
#[ts(export, export_to = "PaginatedTrades.ts")]
pub struct PaginatedTradesResponse {
    pub items: Vec<TradeSummaryResponse>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}

/// Status filter values for `GET /trades`, repeated for multiple statuses
/// (e.g. `?status=PENDING&status=CLOSED`), mirroring `RarityCodeParam` on `/search/card`.
#[derive(Deserialize, Debug, PartialEq, TS, ToSchema)]
#[serde(rename = "TradeStatusParam", rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export, export_to = "TradeStatusParam.ts")]
pub enum TradeStatusParam {
    Pending,
    OneAccepted,
    FullyAccepted,
    Completed,
    Closed,
    Abandoned,
}

impl From<TradeStatusParam> for TradeStatus {
    fn from(p: TradeStatusParam) -> Self {
        match p {
            TradeStatusParam::Pending => TradeStatus::Pending,
            TradeStatusParam::OneAccepted => TradeStatus::OneAccepted,
            TradeStatusParam::FullyAccepted => TradeStatus::FullyAccepted,
            TradeStatusParam::Completed => TradeStatus::Completed,
            TradeStatusParam::Closed => TradeStatus::Closed,
            TradeStatusParam::Abandoned => TradeStatus::Abandoned,
        }
    }
}

#[derive(Deserialize, TS)]
#[ts(export, export_to = "ListTradesParams.ts")]
pub(crate) struct ListTradesParams {
    #[serde(default)]
    pub(crate) page: u32,
    #[serde(default = "default_page_size")]
    pub(crate) page_size: u32,
    /// Status filter, repeated for multiple values (e.g. `?status=PENDING&status=CLOSED`)
    #[serde(default)]
    pub(crate) status: Vec<TradeStatusParam>,
}
