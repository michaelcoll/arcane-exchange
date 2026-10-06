use super::dto::StatsResponse;
use crate::application::error::{AppError, InfraError};
use crate::application::service::stats_service::StatsService;
use crate::application::use_case::MockStatsUseCase;
use crate::domain::stats::Stats;
use crate::infrastructure::adapter_out::repository::common_repository_tests::insert_price;
use crate::infrastructure::adapter_out::repository::entities::CardMarketPriceEntity;
use crate::infrastructure::adapter_out::repository::stats_repository_adapter::StatsRepositoryAdapter;
use crate::infrastructure::{AppState, create_router};
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use chrono::NaiveDate;
use serde_json::Value;
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

/// The whole API router, with the stats use case wired to the real database.
fn app_on(pool: PgPool) -> axum::Router {
    create_router(AppState {
        stats_use_case: Arc::new(StatsService::new(Arc::new(StatsRepositoryAdapter::new(
            pool,
        )))),
        ..AppState::for_testing()
    })
}

/// An anonymous GET: no `Authorization` header.
async fn get(app: axum::Router, uri: &str) -> Response {
    app.oneshot(Request::get(uri).body(Body::empty()).unwrap())
        .await
        .unwrap()
}

async fn json_of(response: Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[sqlx::test]
async fn stats_are_served_without_authentication(pool: PgPool) {
    let date = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
    insert_price(&pool, CardMarketPriceEntity::simple_at(1, date, 100)).await;
    insert_price(&pool, CardMarketPriceEntity::simple_at(2, date, 100)).await;

    let response = get(app_on(pool), "/stats").await;

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

    let body = json_of(get(app_on(pool), "/stats").await).await;

    assert_eq!(body["last_price_date"], "2026-10-01");
}

#[sqlx::test]
async fn last_price_date_is_null_without_any_price(pool: PgPool) {
    sqlx::query("TRUNCATE cardmarket_price")
        .execute(&pool)
        .await
        .unwrap();

    let response = get(app_on(pool), "/stats").await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_of(response).await;
    assert_eq!(body["card_price_number"], 0);
    assert!(body.get("last_price_date").is_some_and(Value::is_null));
}

#[sqlx::test]
async fn stats_are_cached_for_six_hours(pool: PgPool) {
    let response = get(app_on(pool), "/stats").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CACHE_CONTROL).unwrap(),
        "public, max-age=21600"
    );
}

#[sqlx::test]
async fn maintenance_stats_no_longer_exist(pool: PgPool) {
    let response = get(app_on(pool), "/maintenance/stats").await;

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
    }
    .into();

    assert_eq!(response.card_number, 100);
    assert_eq!(response.card_price_number, 150);
    assert_eq!(response.db_size_mb, 500);
    assert_eq!(response.last_price_date.as_deref(), Some("2026-10-01"));
}
