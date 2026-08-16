use super::dto::{
    AddTradeBinderRequest, SetVisibilityRequest, TradeBindersResponse, VisibilityResponse,
};
use crate::application::error::AppError;
use crate::domain::error::FunctionalError;
use crate::infrastructure::AppState;
use crate::infrastructure::adapter_in::auth_extractor::AuthenticatedUser;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};

pub fn create_user_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/register", post(register))
        .route("/visibility", get(get_visibility).put(set_visibility))
        .route(
            "/trade-binders",
            get(get_trade_binders).post(add_trade_binder),
        )
        .route("/trade-binders/{name}", delete(remove_trade_binder))
}

#[utoipa::path(
    post,
    path = "/user/register",
    responses(
        (status = 204, description = "User registered/updated successfully"),
        (status = 400, description = "Missing username claim in token"),
        (status = 401, description = "Missing or invalid authentication token"),
    ),
    security(("bearer_auth" = [])),
    tag = "auth",
)]
pub(crate) async fn register(
    State(state): State<AppState>,
    AuthenticatedUser(user): AuthenticatedUser,
) -> Result<StatusCode, AppError> {
    if user.username.is_none() {
        return Err(
            FunctionalError::WrongFormat("Missing username claim in token".to_string()).into(),
        );
    }

    state.register_user_use_case.register_user(&user).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/user/visibility",
    responses(
        (status = 200, description = "Current collection visibility", body = VisibilityResponse),
        (status = 401, description = "Missing or invalid authentication token"),
        (status = 404, description = "Authenticated user has never registered"),
    ),
    security(("bearer_auth" = [])),
    tag = "auth",
)]
pub(crate) async fn get_visibility(
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
    path = "/user/visibility",
    request_body = SetVisibilityRequest,
    responses(
        (status = 204, description = "Visibility updated successfully"),
        (status = 400, description = "Invalid visibility value"),
        (status = 401, description = "Missing or invalid authentication token"),
        (status = 404, description = "Authenticated user has never registered"),
    ),
    security(("bearer_auth" = [])),
    tag = "auth",
)]
pub(crate) async fn set_visibility(
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
    path = "/user/trade-binders",
    responses(
        (status = 200, description = "Binders selected for trade by the authenticated user", body = TradeBindersResponse),
        (status = 401, description = "Missing or invalid authentication token"),
    ),
    security(("bearer_auth" = [])),
    tag = "auth",
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
    path = "/user/trade-binders",
    request_body = AddTradeBinderRequest,
    responses(
        (status = 204, description = "Binder selected for trade"),
        (status = 400, description = "Binder name is empty"),
        (status = 401, description = "Missing or invalid authentication token"),
        (status = 404, description = "Binder not found in the authenticated user's collection"),
    ),
    security(("bearer_auth" = [])),
    tag = "auth",
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
    path = "/user/trade-binders/{name}",
    params(("name" = String, Path, description = "Binder name")),
    responses(
        (status = 204, description = "Binder deselected for trade"),
        (status = 401, description = "Missing or invalid authentication token"),
    ),
    security(("bearer_auth" = [])),
    tag = "auth",
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
