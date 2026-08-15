use super::dto::{SetVisibilityRequest, VisibilityResponse};
use crate::application::error::AppError;
use crate::domain::error::FunctionalError;
use crate::infrastructure::AppState;
use crate::infrastructure::adapter_in::auth_extractor::AuthenticatedUser;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};

pub fn create_user_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/register", post(register))
        .route("/visibility", get(get_visibility).put(set_visibility))
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
