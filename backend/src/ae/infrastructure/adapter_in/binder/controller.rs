use super::dto::{BinderNameRequest, BinderResponse, BindersResponse};
use crate::application::error::AppError;
use crate::infrastructure::AppState;
use crate::infrastructure::adapter_in::auth_extractor::AuthenticatedUser;
use crate::infrastructure::adapter_in::parse_binder_id;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, patch};

/// Nested under `/collection/binders`: the binders a player stores their collection in.
pub fn create_binder_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", get(list_binders).post(create_binder))
        .route("/{id}", patch(rename_binder).delete(delete_binder))
}

#[utoipa::path(
    get,
    path = "/collection/binders",
    responses(
        (status = 200, description = "The binders of the authenticated user, empty ones included, sorted by name", body = BindersResponse),
        (status = 401, description = "Missing or invalid token"),
    ),
    security(("bearer_auth" = [])),
    tag = "binders",
)]
pub(crate) async fn list_binders(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AppState>,
) -> Result<axum::Json<BindersResponse>, AppError> {
    let binders = state.list_binders_use_case.list_binders(user.id).await?;

    Ok(axum::Json(BindersResponse {
        binders: binders.into_iter().map(BinderResponse::from).collect(),
    }))
}

#[utoipa::path(
    post,
    path = "/collection/binders",
    request_body = BinderNameRequest,
    responses(
        (status = 201, description = "Binder created, closed to trade", body = BinderResponse),
        (status = 400, description = "Blank or too long name; `code` is `invalid_binder_name`"),
        (status = 401, description = "Missing or invalid token"),
        (status = 409, description = "The user already has a binder of this name; `code` is `binder_name_taken`"),
    ),
    security(("bearer_auth" = [])),
    tag = "binders",
)]
pub(crate) async fn create_binder(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AppState>,
    axum::Json(payload): axum::Json<BinderNameRequest>,
) -> Result<(StatusCode, axum::Json<BinderResponse>), AppError> {
    let binder = state
        .create_binder_use_case
        .create_binder(user.id, &payload.name)
        .await?;

    Ok((StatusCode::CREATED, axum::Json(binder.into())))
}

#[utoipa::path(
    patch,
    path = "/collection/binders/{id}",
    params(("id" = uuid::Uuid, Path, description = "Binder id")),
    request_body = BinderNameRequest,
    responses(
        (status = 200, description = "Binder renamed, its trade opening kept", body = BinderResponse),
        (status = 400, description = "Blank or too long name; `code` is `invalid_binder_name`"),
        (status = 401, description = "Missing or invalid token"),
        (status = 404, description = "Binder not found, or not owned by the caller; `code` is `binder_not_found`"),
        (status = 409, description = "The user already has a binder of this name; `code` is `binder_name_taken`"),
    ),
    security(("bearer_auth" = [])),
    tag = "binders",
)]
pub(crate) async fn rename_binder(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    axum::Json(payload): axum::Json<BinderNameRequest>,
) -> Result<axum::Json<BinderResponse>, AppError> {
    let binder = state
        .rename_binder_use_case
        .rename_binder(user.id, parse_binder_id(&id)?, &payload.name)
        .await?;

    Ok(axum::Json(binder.into()))
}

#[utoipa::path(
    delete,
    path = "/collection/binders/{id}",
    params(("id" = uuid::Uuid, Path, description = "Binder id")),
    responses(
        (status = 204, description = "Empty binder deleted, with its trade opening"),
        (status = 401, description = "Missing or invalid token"),
        (status = 404, description = "Binder not found, or not owned by the caller; `code` is `binder_not_found`"),
        (status = 409, description = "The binder still holds copies; `code` is `binder_not_empty`"),
    ),
    security(("bearer_auth" = [])),
    tag = "binders",
)]
pub(crate) async fn delete_binder(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    state
        .delete_binder_use_case
        .delete_binder(user.id, parse_binder_id(&id)?)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
