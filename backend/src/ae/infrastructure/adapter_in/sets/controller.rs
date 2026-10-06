use crate::application::error::AppError;
use crate::domain::set_name::SetCode;
use crate::infrastructure::AppState;
use crate::infrastructure::adapter_in::collection::dto::SetInfoResponse;
use axum::extract::{Path, State};
use axum::routing::get;

pub fn create_set_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", get(list_sets))
        .route("/{set_code}", get(get_set))
}

#[utoipa::path(
    get,
    path = "/sets",
    responses(
        (status = 200, description = "All known sets, ordered by name", body = Vec<SetInfoResponse>),
    ),
    tag = "sets",
)]
pub(crate) async fn list_sets(
    State(state): State<AppState>,
) -> Result<axum::Json<Vec<SetInfoResponse>>, AppError> {
    let sets = state.list_sets_use_case.list_sets().await?;

    Ok(axum::Json(sets.into_iter().map(Into::into).collect()))
}

#[utoipa::path(
    get,
    path = "/sets/{set_code}",
    params(
        ("set_code" = String, Path, description = "Set code, 3 to 5 characters"),
    ),
    responses(
        (status = 200, description = "The set matching set_code", body = SetInfoResponse),
        (status = 400, description = "Invalid set code"),
        (status = 404, description = "No set found for this set_code"),
    ),
    tag = "sets",
)]
pub(crate) async fn get_set(
    State(state): State<AppState>,
    Path(set_code): Path<String>,
) -> Result<axum::Json<SetInfoResponse>, AppError> {
    let code = SetCode::try_new(set_code)?;
    let set = state.get_set_use_case.get_set(code).await?;

    Ok(axum::Json(set.into()))
}
