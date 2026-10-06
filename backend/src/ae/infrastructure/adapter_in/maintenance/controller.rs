use super::dto::EnqueueResponse;
use crate::application::error::AppError;
use crate::infrastructure::AppState;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;

pub fn create_maintenance_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/trigger-price-update", post(trigger_price_update))
        .route("/update-cardmarket-ids", post(update_cardmarket_ids))
        .route("/update-card-images", post(update_card_images))
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
    path = "/maintenance/update-card-images",
    responses(
        (status = 202, description = "Cards in fallback (no Gatherer image in their language) and pending cards enqueued for a new resolution", body = EnqueueResponse),
    ),
    tag = "maintenance",
)]
pub(crate) async fn update_card_images(
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<EnqueueResponse>), AppError> {
    let enqueued = state
        .enqueue_card_image_use_case
        .enqueue_fallback_and_pending_updates()
        .await?;

    Ok((StatusCode::ACCEPTED, Json(EnqueueResponse { enqueued })))
}
