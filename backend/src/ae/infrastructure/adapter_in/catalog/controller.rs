use super::dto::ScanIndexEntryResponse;
use crate::application::error::AppError;
use crate::infrastructure::AppState;
use crate::infrastructure::adapter_in::auth_extractor::AuthenticatedUser;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;

pub fn create_catalog_router() -> axum::Router<AppState> {
    axum::Router::new().route("/scan-index", get(get_scan_index))
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
