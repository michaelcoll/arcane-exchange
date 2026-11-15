use super::dto::{
    PaginatedSearchResponse, SearchCardResponse, SearchParams, SearchSetsParams, offer_of,
};
use crate::application::error::AppError;
use crate::domain::pagination::PageRequest;
use crate::domain::search::{SearchQuery, SearchScope};
use crate::infrastructure::AppState;
use crate::infrastructure::adapter_in::SetInfoResponse;
use crate::infrastructure::adapter_in::auth_extractor::AuthenticatedUser;
use axum::extract::State;
use axum::routing::get;
use axum_extra::extract::Query;

pub fn create_search_router() -> axum::Router<AppState> {
    axum::Router::new().nest(
        "/card",
        axum::Router::new()
            .route("/", get(search_cards))
            .route("/sets", get(search_card_sets)),
    )
}

#[utoipa::path(
    get,
    path = "/search/card",
    params(
        ("page" = Option<u32>, Query, description = "Page number, 0-based (default 0). `page * page_size` must not exceed 10000"),
        ("page_size" = Option<u32>, Query, description = "Items per page, 1 to 100 (default 20)"),
        ("sort_dir" = Option<super::dto::SortDirParam>, Query, description = "Sort direction of the reference price (the normal trend price, or the foil one when there is no normal price). With `q`, the best name match comes first"),
        ("q" = Option<String>, Query, description = "English or French card name, by substring or similarity. Does not match the set"),
        ("offered" = Option<bool>, Query, description = "Only the impressions offered for trade by any player, the authenticated one included"),
        ("offered_by" = Option<String>, Query, description = "Only the impressions offered by this player: exact username, case-insensitive, no partial match. Wins over `offered`; adds `reserved` to the results"),
        ("rarity" = Option<Vec<super::dto::RarityCodeParam>>, Query, description = "Rarity codes, repeated for multiple values (e.g. rarity=C&rarity=U)"),
        ("sets" = Option<String>, Query, description = "Comma-separated set codes"),
        ("price_min" = Option<u32>, Query, description = "Minimum reference price in cents"),
        ("price_max" = Option<u32>, Query, description = "Maximum reference price in cents"),
    ),
    responses(
        (status = 200, description = "Paginated impressions of the Catalogue", body = PaginatedSearchResponse),
        (status = 400, description = "Pagination out of bounds"),
        (status = 401, description = "Missing or invalid token"),
    ),
    security(("bearer_auth" = [])),
    tag = "search",
)]
pub(crate) async fn search_cards(
    AuthenticatedUser(_user): AuthenticatedUser,
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<axum::Json<PaginatedSearchResponse>, AppError> {
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

    let query = SearchQuery {
        pagination,
        sort_dir: params.sort_dir.into(),
        // A blank text is no text, like in the facet.
        search_query: params.q.filter(|q| !q.trim().is_empty()),
        offer: offer_of(params.offered, params.offered_by.as_deref()),
        rarity,
        sets,
        price_min: params.price_min,
        price_max: params.price_max,
    };

    let result = state.search_cards_use_case.search_cards(query).await?;

    Ok(axum::Json(PaginatedSearchResponse {
        items: result
            .items
            .into_iter()
            .map(SearchCardResponse::from)
            .collect(),
        total: result.total,
        page: result.pagination.page(),
        page_size: result.pagination.page_size(),
    }))
}

#[utoipa::path(
    get,
    path = "/search/card/sets",
    params(
        ("q" = Option<String>, Query, description = "Card name, as in GET /search/card"),
        ("offered" = Option<bool>, Query, description = "Only the impressions offered by any player, as in GET /search/card"),
        ("offered_by" = Option<String>, Query, description = "Only the impressions offered by this player, as in GET /search/card"),
    ),
    responses(
        (status = 200, description = "Sets of the impressions GET /search/card can return for this scope, sorted by name. Rarity, set and price filters are ignored, so the list stays stable while filtering", body = Vec<SetInfoResponse>),
        (status = 401, description = "Missing or invalid token"),
    ),
    security(("bearer_auth" = [])),
    tag = "search",
)]
pub(crate) async fn search_card_sets(
    AuthenticatedUser(_user): AuthenticatedUser,
    State(state): State<AppState>,
    Query(params): Query<SearchSetsParams>,
) -> Result<axum::Json<Vec<SetInfoResponse>>, AppError> {
    let scope = SearchScope {
        // Blank means "no text scope". A non-blank `q` is kept as is, so the facet matches
        // exactly what `GET /search/card` does with it.
        search_query: params.q.filter(|q| !q.trim().is_empty()),
        offer: offer_of(params.offered, params.offered_by.as_deref()),
    };

    let sets = state.search_sets_use_case.search_sets(scope).await?;

    Ok(axum::Json(
        sets.into_iter().map(SetInfoResponse::from).collect(),
    ))
}
