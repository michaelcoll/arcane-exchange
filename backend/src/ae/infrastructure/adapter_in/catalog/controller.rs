use super::dto::{
    CatalogCardResponse, CatalogSearchParams, PaginatedCatalogResponse, ScanIndexEntryResponse,
};
use crate::application::error::AppError;
use crate::domain::pagination::PageRequest;
use crate::domain::set_name::SetCode;
use crate::infrastructure::AppState;
use crate::infrastructure::adapter_in::auth_extractor::AuthenticatedUser;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum_extra::extract::Query;

pub fn create_catalog_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/cards", get(search_catalog))
        .route("/scan-index", get(get_scan_index))
}

#[utoipa::path(
    get,
    path = "/catalog/cards",
    params(
        ("q" = Option<String>, Query, description = "Name to search, English or French: substring or trigram similarity. Blank or absent matches every card"),
        ("set_code" = Option<String>, Query, description = "Restricts the search to a set"),
        ("page" = Option<u32>, Query, description = "Page number, 0-based (default 0). `page * page_size` must not exceed 1000"),
        ("page_size" = Option<u32>, Query, description = "Items per page, 1 to 100 (default 20)"),
    ),
    responses(
        (status = 200, description = "The impressions of the catalog matching the search, by decreasing similarity of the name, then set code, then collector number. Tokens are not in the catalog", body = PaginatedCatalogResponse),
        (status = 400, description = "Pagination out of bounds, or invalid set code"),
        (status = 401, description = "Missing or invalid token"),
    ),
    security(("bearer_auth" = [])),
    tag = "catalog",
)]
pub(crate) async fn search_catalog(
    AuthenticatedUser(_user): AuthenticatedUser,
    State(state): State<AppState>,
    Query(params): Query<CatalogSearchParams>,
) -> Result<axum::Json<PaginatedCatalogResponse>, AppError> {
    let set_code = params
        .set_code
        .filter(|code| !code.trim().is_empty())
        .map(|code| SetCode::try_new(code.trim()))
        .transpose()?;

    let result = state
        .search_catalog_use_case
        .search_catalog(
            params.q,
            set_code,
            PageRequest {
                page: params.page,
                page_size: params.page_size,
            },
        )
        .await?;

    Ok(axum::Json(PaginatedCatalogResponse {
        items: result
            .items
            .into_iter()
            .map(CatalogCardResponse::from)
            .collect(),
        total: result.total,
        page: result.pagination.page(),
        page_size: result.pagination.page_size(),
    }))
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
