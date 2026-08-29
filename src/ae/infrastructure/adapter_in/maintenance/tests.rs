use super::controller::*;
use super::dto::*;
use crate::application::error::{AppError, InfraError};
use crate::application::use_case::{
    MockEnqueueCardMarketIdUpdateUseCase, MockImportPriceUseCase, MockStatsUseCase,
};
use crate::domain::error::FunctionalError;
use crate::domain::stats::Stats;
use crate::infrastructure::AppState;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use std::sync::Arc;

// --- Stats ---

#[test]
fn test_stats_response_from_stats_with_values() {
    let stats = Stats {
        card_number: 100,
        card_price_number: 150,
        db_size_mb: 500,
    };
    let response: StatsResponse = stats.into();
    assert_eq!(response.card_number, 100);
    assert_eq!(response.card_price_number, 150);
    assert_eq!(response.db_size_mb, 500);
}

#[tokio::test]
async fn test_get_stats_returns_stats_response_on_success() {
    let mut mock_stats_use_case = MockStatsUseCase::new();
    mock_stats_use_case
        .expect_get_stats()
        .times(1)
        .returning(|| {
            Box::pin(async {
                Ok(Stats {
                    card_number: 42,
                    card_price_number: 85,
                    db_size_mb: 128,
                })
            })
        });

    let result = get_stats(State(AppState::for_testing(Arc::new(mock_stats_use_case)))).await;

    assert!(result.is_ok());
    let Json(response) = result.unwrap();
    assert_eq!(response.card_number, 42);
    assert_eq!(response.card_price_number, 85);
    assert_eq!(response.db_size_mb, 128);
}

#[tokio::test]
async fn test_get_stats_returns_error_on_repository_error() {
    let mut mock_stats_use_case = MockStatsUseCase::new();
    mock_stats_use_case
        .expect_get_stats()
        .times(1)
        .returning(|| {
            Box::pin(async {
                Err(AppError::Infra(InfraError::RepositoryError(
                    "DB error".to_string(),
                )))
            })
        });

    let result = get_stats(State(AppState::for_testing(Arc::new(mock_stats_use_case)))).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::Infra(InfraError::RepositoryError(msg)) => assert_eq!(msg, "DB error"),
        _ => panic!("Expected RepositoryError"),
    }
}

#[tokio::test]
async fn test_get_stats_returns_error_on_price_not_found() {
    let mut mock_stats_use_case = MockStatsUseCase::new();
    mock_stats_use_case
        .expect_get_stats()
        .times(1)
        .returning(|| {
            Box::pin(async { Err(AppError::Functional(FunctionalError::PriceNotFound)) })
        });

    let result = get_stats(State(AppState::for_testing(Arc::new(mock_stats_use_case)))).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::Functional(FunctionalError::PriceNotFound) => {}
        _ => panic!("Expected PriceNotFound"),
    }
}

#[tokio::test]
async fn test_get_stats_with_multiple_calls() {
    let mut mock_stats_use_case = MockStatsUseCase::new();
    mock_stats_use_case
        .expect_get_stats()
        .times(2)
        .returning(|| {
            Box::pin(async {
                Ok(Stats {
                    card_number: 10,
                    card_price_number: 20,
                    db_size_mb: 30,
                })
            })
        });

    let app_state = AppState::for_testing(Arc::new(mock_stats_use_case));
    assert!(get_stats(State(app_state.clone())).await.is_ok());
    assert!(get_stats(State(app_state)).await.is_ok());
}

// --- Trigger price update ---

#[tokio::test]
async fn test_trigger_price_update_returns_no_content_on_success() {
    let mut mock_import_price = MockImportPriceUseCase::new();
    mock_import_price
        .expect_import_prices_for_current_date()
        .times(1)
        .returning(|| Box::pin(async { Ok(()) }));

    let app_state = AppState::for_testing_with_import_price(
        Arc::new(MockStatsUseCase::new()),
        Arc::new(mock_import_price),
    );

    let result = trigger_price_update(State(app_state)).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), StatusCode::NO_CONTENT);
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

    let app_state = AppState::for_testing_with_import_price(
        Arc::new(MockStatsUseCase::new()),
        Arc::new(mock_import_price),
    );

    let result = trigger_price_update(State(app_state)).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::Functional(FunctionalError::PriceNotFound) => {}
        _ => panic!("Expected PriceNotFound"),
    }
}

#[tokio::test]
async fn trigger_price_update_can_be_called_multiple_times_successfully() {
    let mut mock_import_price = MockImportPriceUseCase::new();
    mock_import_price
        .expect_import_prices_for_current_date()
        .times(2)
        .returning(|| Box::pin(async { Ok(()) }));

    let app_state = AppState::for_testing_with_import_price(
        Arc::new(MockStatsUseCase::new()),
        Arc::new(mock_import_price),
    );

    assert_eq!(
        trigger_price_update(State(app_state.clone()))
            .await
            .unwrap(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        trigger_price_update(State(app_state)).await.unwrap(),
        StatusCode::NO_CONTENT
    );
}

#[tokio::test]
async fn get_stats_returns_error_when_use_case_returns_call_error() {
    let mut mock_stats_use_case = MockStatsUseCase::new();
    mock_stats_use_case
        .expect_get_stats()
        .times(1)
        .returning(|| {
            Box::pin(async {
                Err(AppError::Infra(InfraError::CallError(
                    "external service down".to_string(),
                )))
            })
        });

    let result = get_stats(State(AppState::for_testing(Arc::new(mock_stats_use_case)))).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::Infra(InfraError::CallError(msg)) => assert_eq!(msg, "external service down"),
        _ => panic!("Expected CallError"),
    }
}

#[tokio::test]
async fn test_trigger_price_update_returns_error_on_failure() {
    let mut mock_import_price = MockImportPriceUseCase::new();
    mock_import_price
        .expect_import_prices_for_current_date()
        .times(1)
        .returning(|| {
            Box::pin(async {
                Err(AppError::Infra(InfraError::RepositoryError(
                    "Import failed".to_string(),
                )))
            })
        });

    let app_state = AppState::for_testing_with_import_price(
        Arc::new(MockStatsUseCase::new()),
        Arc::new(mock_import_price),
    );

    let result = trigger_price_update(State(app_state)).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::Infra(InfraError::RepositoryError(msg)) => assert_eq!(msg, "Import failed"),
        _ => panic!("Expected RepositoryError"),
    }
}

// --- Update CardMarket IDs (nouveau comportement asynchrone) ---

#[tokio::test]
async fn test_update_cardmarket_ids_returns_accepted_with_enqueued_count() {
    let mut mock_enqueue = MockEnqueueCardMarketIdUpdateUseCase::new();
    mock_enqueue
        .expect_enqueue_pending_updates()
        .times(1)
        .returning(|| Box::pin(async { Ok(5) }));

    let app_state = AppState::for_testing_with_enqueue_cardmarket_id(
        Arc::new(MockStatsUseCase::new()),
        Arc::new(mock_enqueue),
    );

    let result = update_cardmarket_ids(State(app_state)).await;
    assert!(result.is_ok());
    let (status, Json(body)) = result.unwrap();
    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(body.enqueued, 5);
}

#[tokio::test]
async fn test_update_cardmarket_ids_returns_accepted_with_zero_when_all_deduplicated() {
    let mut mock_enqueue = MockEnqueueCardMarketIdUpdateUseCase::new();
    mock_enqueue
        .expect_enqueue_pending_updates()
        .times(1)
        .returning(|| Box::pin(async { Ok(0) }));

    let app_state = AppState::for_testing_with_enqueue_cardmarket_id(
        Arc::new(MockStatsUseCase::new()),
        Arc::new(mock_enqueue),
    );

    let result = update_cardmarket_ids(State(app_state)).await;
    assert!(result.is_ok());
    let (status, Json(body)) = result.unwrap();
    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(body.enqueued, 0);
}

#[tokio::test]
async fn test_update_cardmarket_ids_returns_error_on_repository_error() {
    let mut mock_enqueue = MockEnqueueCardMarketIdUpdateUseCase::new();
    mock_enqueue
        .expect_enqueue_pending_updates()
        .times(1)
        .returning(|| {
            Box::pin(async {
                Err(AppError::Infra(InfraError::RepositoryError(
                    "DB error".to_string(),
                )))
            })
        });

    let app_state = AppState::for_testing_with_enqueue_cardmarket_id(
        Arc::new(MockStatsUseCase::new()),
        Arc::new(mock_enqueue),
    );

    let result = update_cardmarket_ids(State(app_state)).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::Infra(InfraError::RepositoryError(msg)) => assert_eq!(msg, "DB error"),
        _ => panic!("Expected RepositoryError"),
    }
}

#[tokio::test]
async fn test_update_cardmarket_ids_can_be_called_multiple_times() {
    let mut mock_enqueue = MockEnqueueCardMarketIdUpdateUseCase::new();
    mock_enqueue
        .expect_enqueue_pending_updates()
        .times(2)
        .returning(|| Box::pin(async { Ok(3) }));

    let app_state = AppState::for_testing_with_enqueue_cardmarket_id(
        Arc::new(MockStatsUseCase::new()),
        Arc::new(mock_enqueue),
    );

    let (s1, Json(b1)) = update_cardmarket_ids(State(app_state.clone()))
        .await
        .unwrap();
    let (s2, Json(b2)) = update_cardmarket_ids(State(app_state)).await.unwrap();
    assert_eq!(s1, StatusCode::ACCEPTED);
    assert_eq!(s2, StatusCode::ACCEPTED);
    assert_eq!(b1.enqueued, 3);
    assert_eq!(b2.enqueued, 3);
}

// --- Update Gatherer IDs ---

#[tokio::test]
async fn test_update_gatherer_ids_returns_accepted_with_enqueued_count() {
    use crate::application::use_case::MockEnqueueGathererIdUpdateUseCase;

    let mut mock_enqueue = MockEnqueueGathererIdUpdateUseCase::new();
    mock_enqueue
        .expect_enqueue_pending_updates()
        .times(1)
        .returning(|| Box::pin(async { Ok(5) }));

    let app_state = AppState::for_testing_with_enqueue_gatherer_id(
        Arc::new(MockStatsUseCase::new()),
        Arc::new(mock_enqueue),
    );

    let result = update_gatherer_ids(State(app_state)).await;
    assert!(result.is_ok());
    let (status, Json(body)) = result.unwrap();
    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(body.enqueued, 5);
}

#[tokio::test]
async fn test_update_gatherer_ids_returns_error_on_repository_error() {
    use crate::application::use_case::MockEnqueueGathererIdUpdateUseCase;

    let mut mock_enqueue = MockEnqueueGathererIdUpdateUseCase::new();
    mock_enqueue
        .expect_enqueue_pending_updates()
        .times(1)
        .returning(|| {
            Box::pin(async {
                Err(AppError::Infra(InfraError::RepositoryError(
                    "DB error".to_string(),
                )))
            })
        });

    let app_state = AppState::for_testing_with_enqueue_gatherer_id(
        Arc::new(MockStatsUseCase::new()),
        Arc::new(mock_enqueue),
    );

    let result = update_gatherer_ids(State(app_state)).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::Infra(InfraError::RepositoryError(msg)) => assert_eq!(msg, "DB error"),
        _ => panic!("Expected RepositoryError"),
    }
}
