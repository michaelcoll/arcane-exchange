use super::dto::{
    ScanIndexEntryResponse, TrendPriceResponse, TrendPricesRequest, TrendPricesResponse,
};
use crate::application::error::AppError;
use crate::domain::set_name::SetCode;
use crate::infrastructure::AppState;
use crate::infrastructure::adapter_in::auth_extractor::AuthenticatedUser;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};

pub fn create_catalog_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/scan-index", get(get_scan_index))
        .route("/trend-prices", post(get_trend_prices))
}

#[utoipa::path(
    get,
    path = "/catalog/scan-index",
    params(
        ("If-None-Match" = Option<String>, Header, description = "ETag of the index the client already has"),
    ),
    responses(
        (status = 200, description = "The scan index: one entry per catalog card whose collector number is purely numeric. JSON compressed with gzip (`Content-Encoding: gzip`), with an `ETag` derived from a hash of its content.",
            body = Vec<ScanIndexEntryResponse>,
            headers(("ETag" = String, description = "Hash of the content, unchanged by a catalog reload that changes nothing"))),
        (status = 304, description = "The client's index is the current one"),
        (status = 401, description = "Missing or invalid token"),
        (status = 503, description = "No catalog is loaded yet (`scan_index_unavailable`)"),
    ),
    security(("bearer_auth" = [])),
    tag = "catalog",
)]
pub(crate) async fn get_scan_index(
    AuthenticatedUser(_user): AuthenticatedUser,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let index = state.scan_index_use_case.get_scan_index().await?;
    let etag = HeaderValue::from_str(&index.etag).expect("an ETag is a quoted hexadecimal hash");

    if matches_etag(&headers, &index.etag) {
        return Ok((StatusCode::NOT_MODIFIED, [(header::ETAG, etag)]).into_response());
    }
    Ok((
        [
            (header::ETAG, etag),
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            ),
            (header::CONTENT_ENCODING, HeaderValue::from_static("gzip")),
        ],
        index.gzipped_json.clone(),
    )
        .into_response())
}

#[utoipa::path(
    post,
    path = "/catalog/trend-prices",
    request_body = TrendPricesRequest,
    responses(
        (status = 200, description = "One entry per requested card, in the order of the request: the latest Cardmarket trend price of each finish, in cents, `null` when there is none (card absent from the catalog, without Cardmarket id, or unpriced). Does not depend on the language, nor on anyone owning the card", body = TrendPricesResponse),
        (status = 400, description = "More than 500 cards (`invalid_request`), or invalid set code"),
        (status = 401, description = "Missing or invalid token"),
    ),
    security(("bearer_auth" = [])),
    tag = "catalog",
)]
pub(crate) async fn get_trend_prices(
    AuthenticatedUser(_user): AuthenticatedUser,
    State(state): State<AppState>,
    axum::Json(request): axum::Json<TrendPricesRequest>,
) -> Result<axum::Json<TrendPricesResponse>, AppError> {
    let set_codes = request
        .cards
        .iter()
        .map(|card| SetCode::try_new(card.set_code.trim()))
        .collect::<Result<Vec<_>, _>>()?;
    let keys = set_codes
        .iter()
        .cloned()
        .zip(
            request
                .cards
                .iter()
                .map(|card| card.collector_number.clone()),
        )
        .collect();

    let prices = state.trend_prices_use_case.get_trend_prices(keys).await?;

    Ok(axum::Json(TrendPricesResponse {
        prices: request
            .cards
            .into_iter()
            .zip(set_codes)
            .zip(prices)
            .map(|((card, set_code), prices)| {
                TrendPriceResponse::new(card, set_code.to_string(), prices)
            })
            .collect(),
    }))
}

/// Whether one of the entity tags of `If-None-Match` (a comma-separated list, possibly weak) is `etag`.
fn matches_etag(headers: &HeaderMap, etag: &str) -> bool {
    headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(',')
                .map(|candidate| candidate.trim().trim_start_matches("W/"))
                .any(|candidate| candidate == etag)
        })
}
