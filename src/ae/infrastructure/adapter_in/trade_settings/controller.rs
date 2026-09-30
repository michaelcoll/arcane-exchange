use super::dto::{
    AddTradeBinderRequest, RarityFilterResponse, RarityFiltersResponse, SetRarityFilterRequest,
    SetVisibilityRequest, TradeBindersResponse, VisibilityResponse,
};
use crate::application::error::AppError;
use crate::domain::error::FunctionalError;
use crate::domain::rarity_code::RarityCode;
use crate::domain::rarity_trade_filter::RarityTradeFilterRule;
use crate::infrastructure::AppState;
use crate::infrastructure::adapter_in::auth_extractor::AuthenticatedUser;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, get};

/// Nested under `/collection/trade-settings`: the settings by which a player decides what of their
/// collection they expose for trade.
pub fn create_trade_settings_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route(
            "/visibility",
            get(get_trade_visibility).put(set_trade_visibility),
        )
        .route("/binders", get(get_trade_binders).post(add_trade_binder))
        .route("/binders/{name}", delete(remove_trade_binder))
        .route(
            "/rarities",
            get(get_trade_rarity_filters).post(set_trade_rarity_filter),
        )
}

#[utoipa::path(
    get,
    path = "/collection/trade-settings/visibility",
    responses(
        (status = 200, description = "Current collection visibility", body = VisibilityResponse),
        (status = 401, description = "Missing or invalid authentication token"),
        (status = 404, description = "Authenticated user has never registered"),
    ),
    security(("bearer_auth" = [])),
    tag = "trade-settings",
)]
pub(crate) async fn get_trade_visibility(
    State(state): State<AppState>,
    AuthenticatedUser(user): AuthenticatedUser,
) -> Result<axum::Json<VisibilityResponse>, AppError> {
    let visibility = state
        .get_collection_visibility_use_case
        .get_visibility(user.id)
        .await?;

    Ok(axum::Json(VisibilityResponse {
        visibility: visibility.into(),
    }))
}

#[utoipa::path(
    put,
    path = "/collection/trade-settings/visibility",
    request_body = SetVisibilityRequest,
    responses(
        (status = 204, description = "Visibility updated successfully"),
        (status = 400, description = "Invalid visibility value"),
        (status = 401, description = "Missing or invalid authentication token"),
        (status = 404, description = "Authenticated user has never registered"),
    ),
    security(("bearer_auth" = [])),
    tag = "trade-settings",
)]
pub(crate) async fn set_trade_visibility(
    State(state): State<AppState>,
    AuthenticatedUser(user): AuthenticatedUser,
    axum::Json(payload): axum::Json<SetVisibilityRequest>,
) -> Result<StatusCode, AppError> {
    state
        .set_collection_visibility_use_case
        .set_visibility(user.id, payload.visibility.into())
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/collection/trade-settings/binders",
    responses(
        (status = 200, description = "Binders selected for trade by the authenticated user", body = TradeBindersResponse),
        (status = 401, description = "Missing or invalid authentication token"),
    ),
    security(("bearer_auth" = [])),
    tag = "trade-settings",
)]
pub(crate) async fn get_trade_binders(
    State(state): State<AppState>,
    AuthenticatedUser(user): AuthenticatedUser,
) -> Result<axum::Json<TradeBindersResponse>, AppError> {
    let binders = state
        .get_trade_binders_use_case
        .get_trade_binders(user.id)
        .await?;

    Ok(axum::Json(TradeBindersResponse { binders }))
}

#[utoipa::path(
    post,
    path = "/collection/trade-settings/binders",
    request_body = AddTradeBinderRequest,
    responses(
        (status = 204, description = "Binder selected for trade"),
        (status = 400, description = "Binder name is empty"),
        (status = 401, description = "Missing or invalid authentication token"),
        (status = 404, description = "Binder not found in the authenticated user's collection"),
    ),
    security(("bearer_auth" = [])),
    tag = "trade-settings",
)]
pub(crate) async fn add_trade_binder(
    State(state): State<AppState>,
    AuthenticatedUser(user): AuthenticatedUser,
    axum::Json(payload): axum::Json<AddTradeBinderRequest>,
) -> Result<StatusCode, AppError> {
    state
        .add_trade_binder_use_case
        .add_trade_binder(user.id, payload.binder_name)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    delete,
    path = "/collection/trade-settings/binders/{name}",
    params(("name" = String, Path, description = "Binder name")),
    responses(
        (status = 204, description = "Binder deselected for trade"),
        (status = 401, description = "Missing or invalid authentication token"),
    ),
    security(("bearer_auth" = [])),
    tag = "trade-settings",
)]
pub(crate) async fn remove_trade_binder(
    State(state): State<AppState>,
    AuthenticatedUser(user): AuthenticatedUser,
    Path(name): Path<String>,
) -> Result<StatusCode, AppError> {
    state
        .remove_trade_binder_use_case
        .remove_trade_binder(user.id, name)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/collection/trade-settings/rarities",
    responses(
        (status = 200, description = "Rarities owned within the binders selected for trade, with their rarity filter and computed counts", body = RarityFiltersResponse),
        (status = 401, description = "Missing or invalid token"),
    ),
    security(("bearer_auth" = [])),
    tag = "trade-settings",
)]
pub(crate) async fn get_trade_rarity_filters(
    State(state): State<AppState>,
    AuthenticatedUser(user): AuthenticatedUser,
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
    path = "/collection/trade-settings/rarities",
    request_body = SetRarityFilterRequest,
    responses(
        (status = 204, description = "Rarity filter updated successfully"),
        (status = 400, description = "Invalid rarity code or kept_copies out of range"),
        (status = 401, description = "Missing or invalid token"),
    ),
    security(("bearer_auth" = [])),
    tag = "trade-settings",
)]
pub(crate) async fn set_trade_rarity_filter(
    State(state): State<AppState>,
    AuthenticatedUser(user): AuthenticatedUser,
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
