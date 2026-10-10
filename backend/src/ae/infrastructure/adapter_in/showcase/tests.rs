use crate::application::error::{AppError, InfraError};
use crate::application::use_case::MockGetShowcaseUseCase;
use crate::infrastructure::adapter_out::repository::common_repository_tests::{
    insert_card, insert_collection_entry, insert_price, insert_set, insert_user,
    insert_user_with_visibility, refresh_view,
};
use crate::infrastructure::adapter_out::repository::entities::CardMarketPriceEntity;
use crate::infrastructure::testing::{get, json_of, public_app_on};
use crate::infrastructure::{AppState, create_router};
use axum::http::{StatusCode, header};
use chrono::Utc;
use serde_json::{Value, json};
use sqlx::PgPool;
use std::sync::Arc;

const OWNER: &str = "owner";

/// A platform with one player, `OWNER`, whose collection is `private` (the default).
async fn seed_owner(pool: &PgPool) {
    insert_set(pool, "TST").await;
    insert_user(pool, OWNER, OWNER).await;
}

/// A card of `TST` whose image is downloaded, with the trend price of its normal finish.
async fn seed_card(pool: &PgPool, number: &str, language: &str, cardmarket_id: i32, trend: i32) {
    seed_card_with_pending_image(pool, number, language, cardmarket_id).await;
    set_image_source(pool, number, language, "gatherer_localized").await;
    insert_price(pool, CardMarketPriceEntity::simple(cardmarket_id, trend)).await;
}

/// A card of `TST` whose image has not been downloaded yet, and with no price.
async fn seed_card_with_pending_image(
    pool: &PgPool,
    number: &str,
    language: &str,
    cardmarket_id: i32,
) {
    insert_card(pool, "TST", number, language, "A card", cardmarket_id).await;
}

async fn set_image_source(pool: &PgPool, number: &str, language: &str, source: &str) {
    sqlx::query(
        "UPDATE card SET image_source = $1 WHERE (set_code, collector_number, language_code) = ('TST', $2, $3)",
    )
    .bind(source)
    .bind(number)
    .bind(language)
    .execute(pool)
    .await
    .unwrap();
}

/// `user_id` holds `quantity` copies of a card of `TST` in one finish.
async fn seed_held_copies(
    pool: &PgPool,
    user_id: &str,
    number: &str,
    language: &str,
    foil: bool,
    quantity: i32,
) {
    insert_collection_entry(
        pool,
        "TST",
        number,
        language,
        foil,
        user_id,
        quantity,
        0,
        Utc::now(),
    )
    .await;
}

/// What `GET /showcase` answers once the prices of the collections are up to date.
async fn showcase_of(pool: PgPool) -> Value {
    refresh_view(&pool).await;
    let response = get(public_app_on(pool), "/showcase").await;
    assert_eq!(response.status(), StatusCode::OK);
    json_of(response).await
}

#[sqlx::test]
async fn the_showcase_lists_image_paths_by_decreasing_trend_price(pool: PgPool) {
    seed_owner(&pool).await;
    for (number, cardmarket_id, trend) in [("1", 101, 500), ("2", 102, 9000), ("3", 103, 20)] {
        seed_card(&pool, number, "EN", cardmarket_id, trend).await;
        seed_held_copies(&pool, OWNER, number, "EN", false, 1).await;
    }

    assert_eq!(
        showcase_of(pool).await,
        json!([
            "/card-images/TST_2_EN.webp?v=gatherer",
            "/card-images/TST_1_EN.webp?v=gatherer",
            "/card-images/TST_3_EN.webp?v=gatherer",
        ])
    );
}

#[sqlx::test]
async fn the_showcase_keeps_the_thirty_most_expensive_cards(pool: PgPool) {
    seed_owner(&pool).await;
    for n in 1..=31 {
        let number = n.to_string();
        seed_card(&pool, &number, "EN", 100 + n, n * 100).await;
        seed_held_copies(&pool, OWNER, &number, "EN", false, 1).await;
    }

    let body = showcase_of(pool).await;

    let paths = body.as_array().unwrap();
    assert_eq!(paths.len(), 30);
    assert_eq!(paths[0], "/card-images/TST_31_EN.webp?v=gatherer");
    assert_eq!(paths[29], "/card-images/TST_2_EN.webp?v=gatherer");
}

#[sqlx::test]
async fn a_card_is_ranked_by_the_price_of_the_finish_held(pool: PgPool) {
    seed_owner(&pool).await;
    seed_card_with_pending_image(&pool, "1", "EN", 101).await;
    set_image_source(&pool, "1", "EN", "gatherer_localized").await;
    insert_price(&pool, CardMarketPriceEntity::with_foil(101, 100, 9000)).await;
    seed_card(&pool, "2", "EN", 102, 500).await;
    seed_held_copies(&pool, OWNER, "1", "EN", false, 1).await;
    seed_held_copies(&pool, OWNER, "2", "EN", false, 1).await;

    assert_eq!(
        showcase_of(pool).await,
        json!([
            "/card-images/TST_2_EN.webp?v=gatherer",
            "/card-images/TST_1_EN.webp?v=gatherer",
        ])
    );
}

#[sqlx::test]
async fn a_card_held_in_both_finishes_appears_once_at_its_highest_price(pool: PgPool) {
    seed_owner(&pool).await;
    seed_card_with_pending_image(&pool, "1", "EN", 101).await;
    set_image_source(&pool, "1", "EN", "gatherer_localized").await;
    insert_price(&pool, CardMarketPriceEntity::with_foil(101, 100, 9000)).await;
    seed_card(&pool, "2", "EN", 102, 500).await;
    seed_held_copies(&pool, OWNER, "1", "EN", false, 1).await;
    seed_held_copies(&pool, OWNER, "1", "EN", true, 1).await;
    seed_held_copies(&pool, OWNER, "2", "EN", false, 1).await;

    assert_eq!(
        showcase_of(pool).await,
        json!([
            "/card-images/TST_1_EN.webp?v=gatherer",
            "/card-images/TST_2_EN.webp?v=gatherer",
        ])
    );
}

/// The price does not depend on the language, so the languages of a card always tie: the first
/// language code in alphabetical order represents the card.
#[sqlx::test]
async fn a_card_held_in_several_languages_appears_once_in_its_first_language(pool: PgPool) {
    seed_owner(&pool).await;
    seed_card(&pool, "1", "FR", 101, 500).await;
    for language in ["EN", "DE"] {
        seed_card_with_pending_image(&pool, "1", language, 101).await;
        set_image_source(&pool, "1", language, "gatherer_localized").await;
    }
    for language in ["FR", "EN", "DE"] {
        seed_held_copies(&pool, OWNER, "1", language, false, 1).await;
    }

    assert_eq!(
        showcase_of(pool).await,
        json!(["/card-images/TST_1_DE.webp?v=gatherer"])
    );
}

#[sqlx::test]
async fn a_card_whose_image_is_pending_is_left_out(pool: PgPool) {
    seed_owner(&pool).await;
    seed_card_with_pending_image(&pool, "1", "EN", 101).await;
    insert_price(&pool, CardMarketPriceEntity::simple(101, 9000)).await;
    seed_card(&pool, "2", "EN", 102, 500).await;
    seed_held_copies(&pool, OWNER, "1", "EN", false, 1).await;
    seed_held_copies(&pool, OWNER, "2", "EN", false, 1).await;

    assert_eq!(
        showcase_of(pool).await,
        json!(["/card-images/TST_2_EN.webp?v=gatherer"])
    );
}

#[sqlx::test]
async fn a_language_with_an_image_stands_for_a_card_pending_in_another_language(pool: PgPool) {
    seed_owner(&pool).await;
    seed_card_with_pending_image(&pool, "1", "EN", 101).await;
    seed_card(&pool, "1", "FR", 101, 500).await;
    seed_held_copies(&pool, OWNER, "1", "EN", false, 1).await;
    seed_held_copies(&pool, OWNER, "1", "FR", false, 1).await;

    assert_eq!(
        showcase_of(pool).await,
        json!(["/card-images/TST_1_FR.webp?v=gatherer"])
    );
}

#[sqlx::test]
async fn a_card_in_fallback_shows_the_english_image_versioned_by_its_origin(pool: PgPool) {
    seed_owner(&pool).await;
    seed_card(&pool, "1", "FR", 101, 500).await;
    set_image_source(&pool, "1", "FR", "scryfall").await;
    seed_held_copies(&pool, OWNER, "1", "FR", false, 1).await;

    assert_eq!(
        showcase_of(pool).await,
        json!(["/card-images/TST_1_EN.webp?v=scryfall"])
    );
}

#[sqlx::test]
async fn a_card_without_trend_price_is_left_out(pool: PgPool) {
    seed_owner(&pool).await;
    seed_card_with_pending_image(&pool, "1", "EN", 101).await;
    set_image_source(&pool, "1", "EN", "gatherer_localized").await;
    seed_card(&pool, "2", "EN", 102, 500).await;
    seed_held_copies(&pool, OWNER, "1", "EN", false, 1).await;
    seed_held_copies(&pool, OWNER, "2", "EN", false, 1).await;

    assert_eq!(
        showcase_of(pool).await,
        json!(["/card-images/TST_2_EN.webp?v=gatherer"])
    );
}

/// The whole body is compared: it carries image paths only, so neither who holds a card, nor how
/// many copies, nor its price.
#[sqlx::test]
async fn a_card_of_private_collections_is_shown_once_whoever_holds_it_and_however_many(
    pool: PgPool,
) {
    seed_owner(&pool).await;
    insert_user_with_visibility(&pool, "other", "other", "private").await;
    seed_card(&pool, "1", "EN", 101, 500).await;
    seed_held_copies(&pool, OWNER, "1", "EN", false, 4).await;
    seed_held_copies(&pool, "other", "1", "EN", false, 3).await;

    assert_eq!(
        showcase_of(pool).await,
        json!(["/card-images/TST_1_EN.webp?v=gatherer"])
    );
}

#[sqlx::test]
async fn a_platform_without_cards_has_an_empty_showcase_served_without_authentication(
    pool: PgPool,
) {
    let response = get(public_app_on(pool), "/showcase").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(json_of(response).await, json!([]));
}

#[sqlx::test]
async fn the_showcase_is_cached_for_six_hours(pool: PgPool) {
    let response = get(public_app_on(pool), "/showcase").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CACHE_CONTROL).unwrap(),
        "public, max-age=21600"
    );
}

#[tokio::test]
async fn a_failure_is_an_error_response_without_cache() {
    let mut mock = MockGetShowcaseUseCase::new();
    mock.expect_get_showcase().times(1).returning(|| {
        Box::pin(async {
            Err(AppError::Infra(InfraError::RepositoryError(
                "DB error".to_string(),
            )))
        })
    });
    let app = create_router(AppState {
        get_showcase_use_case: Arc::new(mock),
        ..AppState::for_testing()
    });

    let response = get(app, "/showcase").await;

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(response.headers().get(header::CACHE_CONTROL).is_none());
    assert_eq!(json_of(response).await["code"], "internal");
}
