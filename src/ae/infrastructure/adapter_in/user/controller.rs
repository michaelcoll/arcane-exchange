use super::dto::UserProfileResponse;
use crate::application::error::AppError;
use crate::domain::error::FunctionalError;
use crate::infrastructure::AppState;
use crate::infrastructure::adapter_in::auth_extractor::AuthenticatedUser;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};

pub fn create_user_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", post(register))
        .route("/{username}", get(get_user_profile))
}

#[utoipa::path(
    post,
    path = "/user",
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
    path = "/user/{username}",
    params(("username" = String, Path, description = "Player username")),
    responses(
        (status = 200, description = "Public user profile", body = UserProfileResponse),
        (status = 401, description = "Missing or invalid authentication token"),
        (status = 404, description = "Username not found"),
    ),
    security(("bearer_auth" = [])),
    tag = "auth",
)]
pub(crate) async fn get_user_profile(
    State(state): State<AppState>,
    AuthenticatedUser(_user): AuthenticatedUser,
    Path(username): Path<String>,
) -> Result<axum::Json<UserProfileResponse>, AppError> {
    let user = state
        .get_user_profile_use_case
        .get_user_profile(&username)
        .await?;

    Ok(axum::Json(UserProfileResponse::from(user)))
}
