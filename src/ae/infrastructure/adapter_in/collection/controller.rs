use super::dto::{
    CardImportResponse, CardImportStartedResponse, CollectionCardResponse, CollectionParams,
    CollectionStatsResponse, PaginatedCollectionResponse,
};
use crate::application::error::AppError;
use crate::domain::card_import::CardImportId;
use crate::domain::collection::CollectionQuery;
use crate::domain::error::FunctionalError;
use crate::domain::pagination::PageRequest;
use crate::infrastructure::AppState;
use crate::infrastructure::adapter_in::auth_extractor::AuthenticatedUser;
use crate::infrastructure::adapter_in::card::dto::{PriceHistoryEntryResponse, PriceHistoryParams};
use axum::body::to_bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum_extra::extract::Query;

pub fn create_collection_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", get(get_collection))
        .route("/import", post(import_cards).get(list_card_imports))
        .route("/import/{id}", get(get_card_import))
        .route("/stats", get(get_collection_stats))
        .route("/price-history", get(get_collection_price_history))
}

#[utoipa::path(
    post,
    path = "/collection/import",
    request_body(
        content = String,
        content_type = "text/plain",
        description = "ManaBox CSV content (max 10 MB)",
    ),
    responses(
        (status = 202, description = "Import accepted, processing in the background", body = CardImportStartedResponse),
        (status = 400, description = "Rejected file; `code` is `empty_file`, `malformed_csv`, `binder_export` or `unrecognized_format`"),
        (status = 401, description = "Missing or invalid token"),
        (status = 409, description = "An import is already in progress for this user"),
    ),
    security(("bearer_auth" = [])),
    tag = "collection",
)]
pub(crate) async fn import_cards(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AppState>,
    body: axum::body::Body,
) -> Result<(StatusCode, axum::Json<CardImportStartedResponse>), AppError> {
    let bytes = to_bytes(body, 10 * 1024 * 1024)
        .await
        .map_err(|e| FunctionalError::WrongFormat(format!("Failed to read body: {}", e)))?;

    let csv = String::from_utf8(bytes.to_vec())
        .map_err(|_| FunctionalError::MalformedCsv("body is not valid UTF-8".to_string()))?;

    tracing::info!("Importing cards for user: {}", user.id);

    let id = state.import_card_use_case.start_import(&csv, user).await?;

    Ok((
        StatusCode::ACCEPTED,
        axum::Json(CardImportStartedResponse { id: id.to_string() }),
    ))
}

#[utoipa::path(
    get,
    path = "/collection/import/{id}",
    params(("id" = String, Path, description = "Import id")),
    responses(
        (status = 200, description = "Import status", body = CardImportResponse),
        (status = 401, description = "Missing or invalid token"),
        (status = 404, description = "Import not found, or not owned by the caller"),
    ),
    security(("bearer_auth" = [])),
    tag = "collection",
)]
pub(crate) async fn get_card_import(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<axum::Json<CardImportResponse>, AppError> {
    let id = id
        .parse()
        .map(CardImportId)
        .map_err(|_| FunctionalError::ImportNotFound)?;

    let import = state.card_import_query_use_case.find(&id, &user).await?;

    Ok(axum::Json(CardImportResponse::from(import)))
}

#[utoipa::path(
    get,
    path = "/collection/import",
    responses(
        (status = 200, description = "The caller's imports, most recent first", body = Vec<CardImportResponse>),
        (status = 401, description = "Missing or invalid token"),
    ),
    security(("bearer_auth" = [])),
    tag = "collection",
)]
pub(crate) async fn list_card_imports(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AppState>,
) -> Result<axum::Json<Vec<CardImportResponse>>, AppError> {
    let imports = state.card_import_query_use_case.list(&user).await?;

    Ok(axum::Json(
        imports.into_iter().map(CardImportResponse::from).collect(),
    ))
}

#[utoipa::path(
    get,
    path = "/collection",
    params(
        ("page" = Option<u32>, Query, description = "Page number, 0-based (default 0). `page * page_size` must not exceed 10000"),
        ("page_size" = Option<u32>, Query, description = "Items per page, 1 to 100 (default 20)"),
        ("sort_by" = Option<super::dto::SortByParam>, Query, description = "Sort field"),
        ("sort_dir" = Option<super::dto::SortDirParam>, Query, description = "Sort direction"),
        ("q" = Option<String>, Query, description = "Fuzzy search on card name or set"),
        ("rarity" = Option<Vec<super::dto::RarityCodeParam>>, Query, description = "Rarity codes, repeated for multiple values (e.g. rarity=C&rarity=U)"),
        ("sets" = Option<String>, Query, description = "Comma-separated set codes"),
        ("price_min" = Option<u32>, Query, description = "Minimum trend price in cents"),
        ("price_max" = Option<u32>, Query, description = "Maximum trend price in cents"),
    ),
    responses(
        (status = 200, description = "Paginated card collection", body = PaginatedCollectionResponse),
        (status = 400, description = "Pagination out of bounds"),
        (status = 401, description = "Missing or invalid token"),
    ),
    security(("bearer_auth" = [])),
    tag = "collection",
)]
pub(crate) async fn get_collection(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AppState>,
    Query(params): Query<CollectionParams>,
) -> Result<axum::Json<PaginatedCollectionResponse>, AppError> {
    let pagination = PageRequest {
        page: params.page,
        page_size: params.page_size,
    };

    let rarity = params.rarity.into_iter().map(Into::into).collect();

    let sets = params
        .sets
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_uppercase)
        .collect::<Vec<_>>();

    let query = CollectionQuery {
        pagination,
        sort_by: params.sort_by.into(),
        sort_dir: params.sort_dir.into(),
        search_query: params.q,
        rarity,
        sets,
        price_min: params.price_min,
        price_max: params.price_max,
    };

    let result = state
        .get_collection_use_case
        .get_collection(&user.id, query)
        .await?;

    Ok(axum::Json(PaginatedCollectionResponse {
        items: result
            .items
            .into_iter()
            .map(CollectionCardResponse::from)
            .collect(),
        total: result.total,
        page: result.pagination.page(),
        page_size: result.pagination.page_size(),
    }))
}

#[utoipa::path(
    get,
    path = "/collection/stats",
    responses(
        (status = 200, description = "Collection stats for the authenticated user", body = CollectionStatsResponse),
        (status = 401, description = "Missing or invalid token"),
    ),
    security(("bearer_auth" = [])),
    tag = "collection",
)]
pub(crate) async fn get_collection_stats(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AppState>,
) -> Result<axum::Json<CollectionStatsResponse>, AppError> {
    let stats = state
        .get_collection_stats_use_case
        .get_collection_stats(&user.id)
        .await?;
    Ok(axum::Json(CollectionStatsResponse::from(stats)))
}

#[utoipa::path(
    get,
    path = "/collection/price-history",
    params(
        ("start_date" = Option<String>, Query, description = "Start date (ISO 8601: YYYY-MM-DD, inclusive). Defaults to end_date minus 30 days"),
        ("end_date" = Option<String>, Query, description = "End date (ISO 8601: YYYY-MM-DD, inclusive). Defaults to today"),
    ),
    responses(
        (status = 200, description = "Collection price history", body = Vec<PriceHistoryEntryResponse>),
        (status = 400, description = "Invalid date range (start_date > end_date)"),
        (status = 401, description = "Missing or invalid token"),
    ),
    security(("bearer_auth" = [])),
    tag = "collection",
)]
pub(crate) async fn get_collection_price_history(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AppState>,
    Query(params): Query<PriceHistoryParams>,
) -> Result<axum::Json<Vec<PriceHistoryEntryResponse>>, AppError> {
    let entries = state
        .get_collection_price_history_use_case
        .get_collection_price_history(&user.id, params.start_date, params.end_date)
        .await?;

    Ok(axum::Json(
        entries
            .into_iter()
            .map(|e| PriceHistoryEntryResponse {
                date: e.date.to_string(),
                low: e.price_guide.low.value.unwrap_or(0) as i64,
                trend: e.price_guide.trend.value.unwrap_or(0) as i64,
                avg: e.price_guide.avg.value.unwrap_or(0) as i64,
            })
            .collect(),
    ))
}
