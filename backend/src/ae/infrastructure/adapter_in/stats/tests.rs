use super::dto::StatsResponse;
use crate::application::error::{AppError, InfraError};
use crate::application::use_case::MockStatsUseCase;
use crate::domain::stats::Stats;
use crate::infrastructure::adapter_out::repository::common_repository_tests::{
    insert_card_with_rarity, insert_collection_entry_with_binder, insert_price,
    insert_rarity_filter, insert_trading_binder, insert_user_with_visibility,
};
use crate::infrastructure::adapter_out::repository::entities::CardMarketPriceEntity;
use crate::infrastructure::testing::{get, json_of, public_app_on};
use crate::infrastructure::{AppState, create_router};
use axum::http::{StatusCode, header};
use chrono::NaiveDate;
use serde_json::Value;
use sqlx::PgPool;
use std::sync::Arc;

/// A rare card, `FDN` + `number`, in English.
async fn insert_rare(pool: &PgPool, number: &str, cardmarket_id: i32) {
    insert_card_with_rarity(pool, "FDN", number, "EN", "Rare", cardmarket_id, "R").await;
}

/// `quantity` non-foil copies of the rare `FDN` + `number` in `binder` of `user_id`.
async fn insert_copies(
    pool: &PgPool,
    user_id: &str,
    number: &str,
    quantity: i32,
    binder: Option<&str>,
) {
    insert_collection_entry_with_binder(
        pool,
        "FDN",
        number,
        "EN",
        false,
        user_id,
        quantity,
        100,
        chrono::Utc::now(),
        binder,
    )
    .await;
}

#[sqlx::test]
async fn proposed_copies_count_public_collections_and_ignore_private_ones(pool: PgPool) {
    insert_rare(&pool, "1", 1).await;
    insert_user_with_visibility(&pool, "alice", "alice", "public").await;
    insert_user_with_visibility(&pool, "bob", "bob", "public").await;
    insert_user_with_visibility(&pool, "carol", "carol", "private").await;
    insert_copies(&pool, "alice", "1", 3, None).await;
    insert_copies(&pool, "bob", "1", 4, Some("Binder")).await;
    insert_copies(&pool, "carol", "1", 5, None).await;

    let response = get(public_app_on(pool), "/stats").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(json_of(response).await["proposed_copy_number"], 7);
}

#[sqlx::test]
async fn proposed_copies_of_a_trade_collection_are_those_of_its_open_binders(pool: PgPool) {
    insert_rare(&pool, "1", 1).await;
    insert_user_with_visibility(&pool, "alice", "alice", "trade").await;
    insert_rarity_filter(&pool, "alice", "R", true, 0).await;
    insert_trading_binder(&pool, "alice", "Open").await;
    insert_copies(&pool, "alice", "1", 2, Some("Open")).await;
    insert_copies(&pool, "alice", "1", 8, Some("Closed")).await;
    insert_copies(&pool, "alice", "1", 16, None).await;

    let body = json_of(get(public_app_on(pool), "/stats").await).await;

    assert_eq!(body["proposed_copy_number"], 2);
}

#[sqlx::test]
async fn proposed_copies_follow_rarity_filters_and_deduct_kept_copies(pool: PgPool) {
    insert_rare(&pool, "1", 1).await;
    insert_rare(&pool, "2", 2).await;
    insert_card_with_rarity(&pool, "FDN", "3", "EN", "Mythic", 3, "M").await;
    insert_card_with_rarity(&pool, "FDN", "4", "EN", "Common", 4, "C").await;
    insert_user_with_visibility(&pool, "alice", "alice", "trade").await;
    insert_trading_binder(&pool, "alice", "Open").await;
    // Rares: open, one copy kept. Mythics: closed. Commons: no filter, which means closed.
    insert_rarity_filter(&pool, "alice", "R", true, 1).await;
    insert_rarity_filter(&pool, "alice", "M", false, 0).await;
    insert_copies(&pool, "alice", "1", 3, Some("Open")).await;
    insert_copies(&pool, "alice", "2", 1, Some("Open")).await;
    insert_copies(&pool, "alice", "3", 5, Some("Open")).await;
    insert_copies(&pool, "alice", "4", 7, Some("Open")).await;

    let body = json_of(get(public_app_on(pool), "/stats").await).await;

    // 3 − 1 kept = 2 for the first rare, 1 − 1 kept = 0 for the second, nothing else.
    assert_eq!(body["proposed_copy_number"], 2);
}

#[sqlx::test]
async fn proposed_copies_are_zero_on_an_empty_platform(pool: PgPool) {
    let response = get(public_app_on(pool), "/stats").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(json_of(response).await["proposed_copy_number"], 0);
}

#[sqlx::test]
async fn stats_are_served_without_authentication(pool: PgPool) {
    let date = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
    insert_price(&pool, CardMarketPriceEntity::simple_at(1, date, 100)).await;
    insert_price(&pool, CardMarketPriceEntity::simple_at(2, date, 100)).await;

    let response = get(public_app_on(pool), "/stats").await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_of(response).await;
    assert_eq!(body["card_number"], 0);
    assert_eq!(body["card_price_number"], 2);
    assert!(body["db_size_mb"].is_u64());
    assert_eq!(body["last_price_date"], "2026-10-01");
}

#[sqlx::test]
async fn last_price_date_is_the_most_recent_price_date(pool: PgPool) {
    let older = NaiveDate::from_ymd_opt(2026, 9, 30).unwrap();
    let latest = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
    insert_price(&pool, CardMarketPriceEntity::simple_at(1, older, 100)).await;
    insert_price(&pool, CardMarketPriceEntity::simple_at(1, latest, 100)).await;

    let body = json_of(get(public_app_on(pool), "/stats").await).await;

    assert_eq!(body["last_price_date"], "2026-10-01");
}

#[sqlx::test]
async fn last_price_date_is_null_without_any_price(pool: PgPool) {
    sqlx::query("TRUNCATE cardmarket_price")
        .execute(&pool)
        .await
        .unwrap();

    let response = get(public_app_on(pool), "/stats").await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_of(response).await;
    assert_eq!(body["card_price_number"], 0);
    assert!(body.get("last_price_date").is_some_and(Value::is_null));
}

#[sqlx::test]
async fn stats_are_cached_for_six_hours(pool: PgPool) {
    let response = get(public_app_on(pool), "/stats").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CACHE_CONTROL).unwrap(),
        "public, max-age=21600"
    );
}

#[sqlx::test]
async fn maintenance_stats_no_longer_exist(pool: PgPool) {
    let response = get(public_app_on(pool), "/maintenance/stats").await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_failure_is_an_error_response_without_cache() {
    let mut mock = MockStatsUseCase::new();
    mock.expect_get_stats().times(1).returning(|| {
        Box::pin(async {
            Err(AppError::Infra(InfraError::RepositoryError(
                "DB error".to_string(),
            )))
        })
    });
    let app = create_router(AppState {
        stats_use_case: Arc::new(mock),
        ..AppState::for_testing()
    });

    let response = get(app, "/stats").await;

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(response.headers().get(header::CACHE_CONTROL).is_none());
    assert_eq!(json_of(response).await["code"], "internal");
}

#[test]
fn response_formats_the_last_price_date_as_an_iso_day() {
    let response: StatsResponse = Stats {
        card_number: 100,
        card_price_number: 150,
        db_size_mb: 500,
        last_price_date: NaiveDate::from_ymd_opt(2026, 10, 1),
        proposed_copy_number: 1248,
    }
    .into();

    assert_eq!(response.proposed_copy_number, 1248);
    assert_eq!(response.card_number, 100);
    assert_eq!(response.card_price_number, 150);
    assert_eq!(response.db_size_mb, 500);
    assert_eq!(response.last_price_date.as_deref(), Some("2026-10-01"));
}
