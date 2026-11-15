use crate::domain::catalog::TrendPrices;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

/// One entry of the scan index, keyed by set and collector number.
///
/// The body of `GET /catalog/scan-index` is a JSON array of these; it is built and gzipped once
/// per catalog load by the `ScanIndexUseCase` from the serialization of `ScanIndexEntry`, this
/// type only documents its shape (a controller test decodes the served body as this type).
#[derive(Serialize, Debug, ToSchema)]
#[cfg_attr(test, derive(serde::Deserialize), serde(deny_unknown_fields))]
pub struct ScanIndexEntryResponse {
    /// Set code, 3 to 6 uppercase characters.
    pub set_code: String,
    /// Purely numeric collector number, without leading zeros.
    pub collector_number: String,
    /// Rarity code: `C`, `U`, `R`, `M` or `S`.
    pub rarity: String,
    /// English name.
    pub name: String,
    /// French name; absent when Scryfall does not have one, the client then shows the English name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub french_name: Option<String>,
}

/// A card to price, by set and collector number: no language, the price does not depend on it.
#[derive(Deserialize, Debug, TS, ToSchema)]
#[serde(rename = "TrendPriceCardRequest")]
#[ts(export)]
pub struct TrendPriceCardRequest {
    /// Set code, 3 to 6 characters.
    pub set_code: String,
    /// Collector number; leading zeros are ignored.
    pub collector_number: String,
}

#[derive(Deserialize, Debug, TS, ToSchema)]
#[serde(rename = "TrendPricesRequest")]
#[ts(export)]
pub struct TrendPricesRequest {
    /// The cards to price, 500 at most.
    pub cards: Vec<TrendPriceCardRequest>,
}

/// The latest Cardmarket trend prices of one requested card.
#[derive(Serialize, Debug, TS, ToSchema)]
#[serde(rename = "TrendPrice")]
#[ts(export)]
pub struct TrendPriceResponse {
    /// The set code as requested, uppercased.
    pub set_code: String,
    /// The collector number as requested.
    pub collector_number: String,
    /// Trend price of the normal finish, in cents; `null` when there is none.
    pub trend_cents: Option<u32>,
    /// Trend price of the foil finish, in cents; `null` when there is none.
    pub foil_trend_cents: Option<u32>,
}

impl TrendPriceResponse {
    pub(crate) fn new(card: TrendPriceCardRequest, set_code: String, prices: TrendPrices) -> Self {
        Self {
            set_code,
            collector_number: card.collector_number,
            trend_cents: prices.trend_cents,
            foil_trend_cents: prices.foil_trend_cents,
        }
    }
}

/// The trend prices of the requested cards, in the order of the request.
#[derive(Serialize, Debug, TS, ToSchema)]
#[serde(rename = "TrendPrices")]
#[ts(export)]
pub struct TrendPricesResponse {
    pub prices: Vec<TrendPriceResponse>,
}
