use super::dto::SearchParams;
use crate::application::error::AppError;
use crate::application::service::search_service::SEARCH_MAX_OFFSET;
use crate::domain::collection::{CollectionQuery, SearchQuery};
use crate::domain::pagination::Pagination;
use crate::infrastructure::AppState;
use crate::infrastructure::adapter_in::auth_extractor::AuthenticatedUser;
use crate::infrastructure::adapter_in::collection::dto::{
    CollectionCardResponse, PaginatedCollectionResponse,
};
use axum::extract::State;
use axum::routing::get;
use axum_extra::extract::Query;

pub fn create_search_router() -> axum::Router<AppState> {
    axum::Router::new().nest("/card", axum::Router::new().route("/", get(search_cards)))
}

#[utoipa::path(
    get,
    path = "/search/card",
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
        ("player_username" = Option<String>, Query, description = "Exact username of the owner to filter by (case-insensitive, no partial match)"),
    ),
    responses(
        (status = 200, description = "Paginated card search results", body = PaginatedCollectionResponse),
        (status = 400, description = "Pagination out of bounds, or sort_by=added_at without player_username"),
        (status = 401, description = "Missing or invalid token"),
    ),
    security(("bearer_auth" = [])),
    tag = "search",
)]
pub(crate) async fn search_cards(
    AuthenticatedUser(_user): AuthenticatedUser,
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<axum::Json<PaginatedCollectionResponse>, AppError> {
    let pagination = Pagination::try_new(params.page, params.page_size, SEARCH_MAX_OFFSET)?;

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

    let player_username = params
        .player_username
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let query = SearchQuery::try_new(
        CollectionQuery {
            pagination,
            sort_by: params.sort_by.into(),
            sort_dir: params.sort_dir.into(),
            search_query: params.q,
            rarity,
            sets,
            price_min: params.price_min,
            price_max: params.price_max,
        },
        player_username,
    )?;

    let result = state.search_cards_use_case.search_cards(query).await?;

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
