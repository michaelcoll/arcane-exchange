use super::dto::{EnqueueResponse, StatsResponse};
use crate::application::error::AppError;
use crate::infrastructure::AppState;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};

pub fn create_maintenance_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/stats", get(get_stats))
        .route("/trigger-price-update", post(trigger_price_update))
        .route("/update-cardmarket-ids", post(update_cardmarket_ids))
        .route("/update-gatherer-ids", post(update_gatherer_ids))
}

#[utoipa::path(
    get,
    path = "/maintenance/stats",
    responses(
        (status = 200, description = "Global database statistics", body = StatsResponse),
    ),
    tag = "maintenance",
)]
pub(crate) async fn get_stats(
    State(state): State<AppState>,
) -> Result<Json<StatsResponse>, AppError> {
    let stats = state.stats_use_case.get_stats().await?;
    Ok(Json(stats.into()))
}

#[utoipa::path(
    post,
    path = "/maintenance/trigger-price-update",
    responses(
        (status = 204, description = "Price update triggered successfully"),
    ),
    tag = "maintenance",
)]
pub(crate) async fn trigger_price_update(
    State(state): State<AppState>,
) -> Result<StatusCode, AppError> {
    state
        .import_price_use_case
        .import_prices_for_current_date()
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/maintenance/update-cardmarket-ids",
    responses(
        (status = 202, description = "CardMarket IDs enqueued for update", body = EnqueueResponse),
    ),
    tag = "maintenance",
)]
pub(crate) async fn update_cardmarket_ids(
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<EnqueueResponse>), AppError> {
    let enqueued = state
        .enqueue_cardmarket_id_use_case
        .enqueue_pending_updates()
        .await?;

    Ok((StatusCode::ACCEPTED, Json(EnqueueResponse { enqueued })))
}

#[utoipa::path(
    post,
    path = "/maintenance/update-gatherer-ids",
    responses(
        (status = 202, description = "Gatherer IDs enqueued for update", body = EnqueueResponse),
    ),
    tag = "maintenance",
)]
pub(crate) async fn update_gatherer_ids(
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<EnqueueResponse>), AppError> {
    let enqueued = state
        .enqueue_gatherer_id_use_case
        .enqueue_pending_updates()
        .await?;

    Ok((StatusCode::ACCEPTED, Json(EnqueueResponse { enqueued })))
}
