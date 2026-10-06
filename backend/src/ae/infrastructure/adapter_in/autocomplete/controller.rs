use super::dto::{AutocompleteUserParams, UserSuggestionResponse};
use crate::application::error::AppError;
use crate::infrastructure::AppState;
use axum::Json;
use axum::extract::State;
use axum::routing::get;
use axum_extra::extract::Query;

pub fn create_autocomplete_router() -> axum::Router<AppState> {
    axum::Router::new().nest(
        "/user",
        axum::Router::new().route("/", get(autocomplete_user)),
    )
}

#[utoipa::path(
    get,
    path = "/autocomplete/user",
    params(
        ("q" = Option<String>, Query, description = "Partial username, fuzzy-matched (case-insensitive)"),
    ),
    responses(
        (status = 200, description = "Matching usernames, max 10, ordered by similarity score", body = Vec<UserSuggestionResponse>),
    ),
    tag = "autocomplete",
)]
pub(crate) async fn autocomplete_user(
    State(state): State<AppState>,
    Query(params): Query<AutocompleteUserParams>,
) -> Result<Json<Vec<UserSuggestionResponse>>, AppError> {
    let suggestions = state
        .autocomplete_users_use_case
        .autocomplete(params.q)
        .await?;

    Ok(Json(suggestions.into_iter().map(Into::into).collect()))
}
