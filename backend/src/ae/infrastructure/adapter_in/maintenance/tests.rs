//! The maintenance routes are public (see `ARCHITECTURE.md`): every request here is anonymous.

use crate::application::error::{AppError, InfraError};
use crate::application::use_case::{
    MockEnqueueCardImageUpdateUseCase, MockEnqueueCardMarketIdUpdateUseCase,
    MockImportPriceUseCase, MockUpdateCatalogUseCase,
};
use crate::domain::error::FunctionalError;
use crate::infrastructure::testing::{anonymous, assert_error, json_of};
use crate::infrastructure::{AppState, create_router};
use axum::http::{Method, StatusCode};
use serde_json::json;
use std::sync::Arc;

fn price_app(import_price_use_case: MockImportPriceUseCase) -> axum::Router {
    create_router(AppState {
        import_price_use_case: Arc::new(import_price_use_case),
        ..AppState::for_testing()
    })
}

fn cardmarket_ids_app(use_case: MockEnqueueCardMarketIdUpdateUseCase) -> axum::Router {
    create_router(AppState {
        enqueue_cardmarket_id_use_case: Arc::new(use_case),
        ..AppState::for_testing()
    })
}

fn card_images_app(use_case: MockEnqueueCardImageUpdateUseCase) -> axum::Router {
    create_router(AppState {
        enqueue_card_image_use_case: Arc::new(use_case),
        ..AppState::for_testing()
    })
}

fn catalog_app(use_case: MockUpdateCatalogUseCase) -> axum::Router {
    create_router(AppState {
        update_catalog_use_case: Arc::new(use_case),
        ..AppState::for_testing()
    })
}

fn repository_error(message: &str) -> AppError {
    AppError::Infra(InfraError::RepositoryError(message.to_string()))
}

// --- Trigger price update ---

#[tokio::test]
async fn trigger_price_update_returns_no_content_on_success() {
    let mut mock_import_price = MockImportPriceUseCase::new();
    mock_import_price
        .expect_import_prices_for_current_date()
        .times(1)
        .returning(|| Box::pin(async { Ok(()) }));

    let response = anonymous(
        price_app(mock_import_price),
        Method::POST,
        "/maintenance/trigger-price-update",
    )
    .await;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn trigger_price_update_returns_price_not_found_error() {
    let mut mock_import_price = MockImportPriceUseCase::new();
    mock_import_price
        .expect_import_prices_for_current_date()
        .times(1)
        .returning(|| {
            Box::pin(async { Err(AppError::Functional(FunctionalError::PriceNotFound)) })
        });

    let response = anonymous(
        price_app(mock_import_price),
        Method::POST,
        "/maintenance/trigger-price-update",
    )
    .await;

    assert_error(response, StatusCode::NOT_FOUND, "price_not_found").await;
}

#[tokio::test]
async fn trigger_price_update_can_be_called_multiple_times_successfully() {
    let mut mock_import_price = MockImportPriceUseCase::new();
    mock_import_price
        .expect_import_prices_for_current_date()
        .times(2)
        .returning(|| Box::pin(async { Ok(()) }));
    let app = price_app(mock_import_price);

    for _ in 0..2 {
        let response = anonymous(
            app.clone(),
            Method::POST,
            "/maintenance/trigger-price-update",
        )
        .await;

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }
}

#[tokio::test]
async fn trigger_price_update_returns_error_on_failure() {
    let mut mock_import_price = MockImportPriceUseCase::new();
    mock_import_price
        .expect_import_prices_for_current_date()
        .times(1)
        .returning(|| Box::pin(async { Err(repository_error("Import failed")) }));

    let response = anonymous(
        price_app(mock_import_price),
        Method::POST,
        "/maintenance/trigger-price-update",
    )
    .await;

    assert_error(response, StatusCode::INTERNAL_SERVER_ERROR, "internal").await;
}

// --- Update CardMarket IDs ---

#[tokio::test]
async fn update_cardmarket_ids_returns_accepted_with_enqueued_count() {
    let mut mock_enqueue = MockEnqueueCardMarketIdUpdateUseCase::new();
    mock_enqueue
        .expect_enqueue_pending_updates()
        .times(1)
        .returning(|| Box::pin(async { Ok(5) }));

    let response = anonymous(
        cardmarket_ids_app(mock_enqueue),
        Method::POST,
        "/maintenance/update-cardmarket-ids",
    )
    .await;

    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert_eq!(json_of(response).await, json!({ "enqueued": 5 }));
}

#[tokio::test]
async fn update_cardmarket_ids_returns_accepted_with_zero_when_all_deduplicated() {
    let mut mock_enqueue = MockEnqueueCardMarketIdUpdateUseCase::new();
    mock_enqueue
        .expect_enqueue_pending_updates()
        .times(1)
        .returning(|| Box::pin(async { Ok(0) }));

    let response = anonymous(
        cardmarket_ids_app(mock_enqueue),
        Method::POST,
        "/maintenance/update-cardmarket-ids",
    )
    .await;

    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert_eq!(json_of(response).await, json!({ "enqueued": 0 }));
}

#[tokio::test]
async fn update_cardmarket_ids_returns_error_on_repository_error() {
    let mut mock_enqueue = MockEnqueueCardMarketIdUpdateUseCase::new();
    mock_enqueue
        .expect_enqueue_pending_updates()
        .times(1)
        .returning(|| Box::pin(async { Err(repository_error("DB error")) }));

    let response = anonymous(
        cardmarket_ids_app(mock_enqueue),
        Method::POST,
        "/maintenance/update-cardmarket-ids",
    )
    .await;

    assert_error(response, StatusCode::INTERNAL_SERVER_ERROR, "internal").await;
}

#[tokio::test]
async fn update_cardmarket_ids_can_be_called_multiple_times() {
    let mut mock_enqueue = MockEnqueueCardMarketIdUpdateUseCase::new();
    mock_enqueue
        .expect_enqueue_pending_updates()
        .times(2)
        .returning(|| Box::pin(async { Ok(3) }));
    let app = cardmarket_ids_app(mock_enqueue);

    for _ in 0..2 {
        let response = anonymous(
            app.clone(),
            Method::POST,
            "/maintenance/update-cardmarket-ids",
        )
        .await;

        assert_eq!(response.status(), StatusCode::ACCEPTED);
        assert_eq!(json_of(response).await, json!({ "enqueued": 3 }));
    }
}

// --- Update card images ---

#[tokio::test]
async fn update_card_images_enqueues_only_cards_in_fallback_or_pending() {
    let mut mock_enqueue = MockEnqueueCardImageUpdateUseCase::new();
    mock_enqueue
        .expect_enqueue_fallback_and_pending_updates()
        .times(1)
        .returning(|| Box::pin(async { Ok(5) }));
    mock_enqueue.expect_enqueue_pending_updates().times(0);

    let response = anonymous(
        card_images_app(mock_enqueue),
        Method::POST,
        "/maintenance/update-card-images",
    )
    .await;

    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert_eq!(json_of(response).await, json!({ "enqueued": 5 }));
}

#[tokio::test]
async fn update_card_images_returns_error_on_repository_error() {
    let mut mock_enqueue = MockEnqueueCardImageUpdateUseCase::new();
    mock_enqueue
        .expect_enqueue_fallback_and_pending_updates()
        .times(1)
        .returning(|| Box::pin(async { Err(repository_error("DB error")) }));

    let response = anonymous(
        card_images_app(mock_enqueue),
        Method::POST,
        "/maintenance/update-card-images",
    )
    .await;

    assert_error(response, StatusCode::INTERNAL_SERVER_ERROR, "internal").await;
}

// --- Update catalog ---

#[tokio::test]
async fn update_catalog_returns_no_content_without_authentication() {
    let mut mock_update = MockUpdateCatalogUseCase::new();
    mock_update
        .expect_update_catalog()
        .times(1)
        .returning(|| Box::pin(async { Ok(()) }));

    let response = anonymous(
        catalog_app(mock_update),
        Method::POST,
        "/maintenance/update-catalog",
    )
    .await;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn update_catalog_returns_error_on_failure() {
    let mut mock_update = MockUpdateCatalogUseCase::new();
    mock_update
        .expect_update_catalog()
        .times(1)
        .returning(|| Box::pin(async { Err(repository_error("Load failed")) }));

    let response = anonymous(
        catalog_app(mock_update),
        Method::POST,
        "/maintenance/update-catalog",
    )
    .await;

    assert_error(response, StatusCode::INTERNAL_SERVER_ERROR, "internal").await;
}
