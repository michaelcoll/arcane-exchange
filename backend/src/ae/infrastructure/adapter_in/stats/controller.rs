use super::dto::StatsResponse;
use crate::application::error::AppError;
use crate::infrastructure::AppState;
use crate::infrastructure::adapter_in::PRICE_BOUND_CACHE_CONTROL;
use axum::Json;
use axum::extract::State;
use axum::http::header;
use axum::response::IntoResponse;
use axum::routing::get;

pub fn create_stats_router() -> axum::Router<AppState> {
    axum::Router::new().route("/", get(get_stats))
}

#[utoipa::path(
    get,
    path = "/stats",
    responses(
        (status = 200, description = "Global platform statistics, cached for 6 hours", body = StatsResponse,
            headers(("Cache-Control" = String, description = "public, max-age=21600"))),
    ),
    tag = "stats",
)]
pub(crate) async fn get_stats(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let stats = state.stats_use_case.get_stats().await?;
    Ok((
        [(header::CACHE_CONTROL, PRICE_BOUND_CACHE_CONTROL)],
        Json(StatsResponse::from(stats)),
    ))
}
