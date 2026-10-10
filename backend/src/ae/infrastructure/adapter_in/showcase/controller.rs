use crate::application::error::AppError;
use crate::domain::showcase::ShowcaseCard;
use crate::infrastructure::AppState;
use crate::infrastructure::adapter_in::PRICE_BOUND_CACHE_CONTROL;
use crate::infrastructure::adapter_in::collection::dto::card_image_urls;
use axum::Json;
use axum::extract::State;
use axum::http::header;
use axum::response::IntoResponse;
use axum::routing::get;

pub fn create_showcase_router() -> axum::Router<AppState> {
    axum::Router::new().route("/", get(get_showcase))
}

#[utoipa::path(
    get,
    path = "/showcase",
    responses(
        (status = 200, description = "Front image paths of the most expensive cards held in the platform's collections, most expensive first, at most 30, cached for 6 hours. Same format as `image_url`: relative to the frontend.", body = Vec<String>,
            headers(("Cache-Control" = String, description = "public, max-age=21600"))),
    ),
    tag = "showcase",
)]
pub(crate) async fn get_showcase(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let cards = state.get_showcase_use_case.get_showcase().await?;
    let image_urls: Vec<String> = cards.iter().filter_map(front_image_url).collect();
    Ok((
        [(header::CACHE_CONTROL, PRICE_BOUND_CACHE_CONTROL)],
        Json(image_urls),
    ))
}

fn front_image_url(card: &ShowcaseCard) -> Option<String> {
    card_image_urls(&card.card_id, Some(card.image)).0
}
