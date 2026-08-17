use super::dto::{
    CollectionCardResponse, CollectionParams, CollectionStatsResponse, MessageResponse,
    PaginatedCollectionResponse, RarityFilterResponse, RarityFiltersResponse,
    SetRarityFilterRequest,
};
use crate::application::error::AppError;
use crate::application::service::collection_service::COLLECTION_MAX_OFFSET;
use crate::domain::collection::CollectionQuery;
use crate::domain::error::FunctionalError;
use crate::domain::pagination::Pagination;
use crate::domain::rarity_code::RarityCode;
use crate::domain::rarity_trade_filter::RarityTradeFilterRule;
use crate::infrastructure::AppState;
use crate::infrastructure::adapter_in::auth_extractor::AuthenticatedUser;
use crate::infrastructure::adapter_in::card::dto::{PriceHistoryEntryResponse, PriceHistoryParams};
use axum::body::to_bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum_extra::extract::Query;

pub fn create_collection_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", get(get_collection))
        .route("/import", post(import_cards))
        .route("/stats", get(get_collection_stats))
        .route("/price-history", get(get_collection_price_history))
        .route(
            "/visibility/rarities",
            get(get_rarity_filters).post(set_rarity_filter),
        )
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
        (status = 200, description = "Import successful", body = MessageResponse),
        (status = 400, description = "Invalid body (non UTF-8, ...)"),
        (status = 401, description = "Missing or invalid token"),
    ),
    security(("bearer_auth" = [])),
    tag = "collection",
)]
pub(crate) async fn import_cards(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AppState>,
    body: axum::body::Body,
) -> Result<axum::Json<MessageResponse>, AppError> {
    let bytes = to_bytes(body, 10 * 1024 * 1024)
        .await
        .map_err(|e| FunctionalError::WrongFormat(format!("Failed to read body: {}", e)))?;

    let csv = String::from_utf8(bytes.to_vec())
        .map_err(|_| FunctionalError::WrongFormat("Body is not valid UTF-8".to_string()))?;

    tracing::info!("Importing cards for user: {}", user.id);

    state
        .import_card_use_case
        .clone()
        .import_cards(&csv, user)
        .await?;

    Ok(axum::Json(MessageResponse {
        message: "Cards imported successfully".to_string(),
    }))
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
    let pagination = Pagination::try_new(params.page, params.page_size, COLLECTION_MAX_OFFSET)?;

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

#[utoipa::path(
    get,
    path = "/collection/visibility/rarities",
    responses(
        (status = 200, description = "Rarities owned within the binders selected for trade, with their trade rule and computed counts", body = RarityFiltersResponse),
        (status = 401, description = "Missing or invalid token"),
    ),
    security(("bearer_auth" = [])),
    tag = "collection",
)]
pub(crate) async fn get_rarity_filters(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AppState>,
) -> Result<axum::Json<RarityFiltersResponse>, AppError> {
    let rarities = state
        .get_rarity_trade_filters_use_case
        .get_rarity_trade_filters(user.id)
        .await?;

    Ok(axum::Json(RarityFiltersResponse {
        rarities: rarities
            .into_iter()
            .map(RarityFilterResponse::from)
            .collect(),
    }))
}

#[utoipa::path(
    post,
    path = "/collection/visibility/rarities",
    request_body = SetRarityFilterRequest,
    responses(
        (status = 204, description = "Trade rule updated successfully"),
        (status = 400, description = "Invalid rarity code or kept_copies out of range"),
        (status = 401, description = "Missing or invalid token"),
    ),
    security(("bearer_auth" = [])),
    tag = "collection",
)]
pub(crate) async fn set_rarity_filter(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AppState>,
    axum::Json(payload): axum::Json<SetRarityFilterRequest>,
) -> Result<StatusCode, AppError> {
    let rarity = RarityCode::try_new(&payload.rarity)?;
    let kept_copies = u8::try_from(payload.kept_copies).map_err(|_| {
        FunctionalError::WrongFormat("Kept copies must be between 0 and 4".to_string())
    })?;

    state
        .set_rarity_trade_filter_use_case
        .set_rarity_trade_filter(
            user.id,
            RarityTradeFilterRule {
                rarity,
                is_open: payload.is_open,
                kept_copies,
            },
        )
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
