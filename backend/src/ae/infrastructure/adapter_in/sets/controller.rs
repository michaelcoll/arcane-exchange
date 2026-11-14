use crate::application::error::AppError;
use crate::domain::set_name::SetCode;
use crate::infrastructure::AppState;
use crate::infrastructure::adapter_in::SetInfoResponse;
use crate::infrastructure::adapter_in::auth_extractor::AuthenticatedUser;
use axum::extract::{FromRequestParts, Path, Query, State};
use axum::http::request::Parts;
use axum::routing::get;
use serde::Deserialize;

pub fn create_set_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", get(list_sets))
        .route("/{set_code}", get(get_set))
}

#[derive(Deserialize, Default)]
pub(crate) struct ListSetsParams {
    #[serde(default)]
    owned: bool,
}

#[utoipa::path(
    get,
    path = "/sets",
    params(
        ("owned" = Option<bool>, Query, description = "`true` restricts the list to the sets the calling player owns at least one card of, and requires authentication"),
    ),
    responses(
        (status = 200, description = "The whole catalog of sets, the most recent first (by name within a release date); sets without a release date come last", body = Vec<SetInfoResponse>),
        (status = 401, description = "`owned=true` without a valid token"),
    ),
    tag = "sets",
)]
pub(crate) async fn list_sets(
    State(state): State<AppState>,
    Query(params): Query<ListSetsParams>,
    mut parts: Parts,
) -> Result<axum::Json<Vec<SetInfoResponse>>, AppError> {
    // The route stays public: the caller is only required, and resolved, for `owned=true`.
    let owner = if params.owned {
        let AuthenticatedUser(user) =
            AuthenticatedUser::from_request_parts(&mut parts, &state).await?;
        Some(user.id)
    } else {
        None
    };
    let sets = state.list_sets_use_case.list_sets(owner).await?;

    Ok(axum::Json(sets.into_iter().map(Into::into).collect()))
}

#[utoipa::path(
    get,
    path = "/sets/{set_code}",
    params(
        ("set_code" = String, Path, description = "Set code, 3 to 6 characters"),
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
