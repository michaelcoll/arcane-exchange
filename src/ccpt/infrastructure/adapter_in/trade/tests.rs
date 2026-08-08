use super::controller::*;
use super::dto::{CreateTradeRequest, RateTradeRequest};
use crate::application::error::AppError;
use crate::application::use_case::{
    MockAbandonTradeUseCase, MockAcceptTradeUseCase, MockConfirmTradeUseCase,
    MockCreateTradeUseCase, MockRateTradeUseCase, MockStatsUseCase,
};
use crate::domain::error::FunctionalError;
use crate::domain::trade::TradeId;
use crate::domain::user::User;
use crate::infrastructure::AppState;
use crate::infrastructure::adapter_in::auth_extractor::AuthenticatedUser;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use std::sync::Arc;

fn make_app_state(create_trade_use_case: MockCreateTradeUseCase) -> AppState {
    AppState::for_testing_with_create_trade(
        Arc::new(MockStatsUseCase::new()),
        Arc::new(create_trade_use_case),
    )
}

fn make_payload() -> CreateTradeRequest {
    CreateTradeRequest {
        set_code: "FDN".to_string(),
        collector_number: "87".to_string(),
        language_code: "FR".to_string(),
        foil: false,
        respondent_user_id: "user_respondent".to_string(),
        quantity: 1,
    }
}

#[tokio::test]
async fn create_trade_returns_created_on_nominal_payload() {
    let mut mock_use_case = MockCreateTradeUseCase::new();
    mock_use_case
        .expect_create_trade()
        .times(1)
        .returning(|_, _, _, _| Box::pin(async { Ok(TradeId::new()) }));

    let state = make_app_state(mock_use_case);
    let user = User::for_testing();

    let result = create_trade(
        AuthenticatedUser(user),
        State(state),
        axum::Json(make_payload()),
    )
    .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), StatusCode::CREATED);
}

#[tokio::test]
async fn create_trade_returns_bad_request_on_invalid_language_code() {
    let mock_use_case = MockCreateTradeUseCase::new();
    let state = make_app_state(mock_use_case);
    let mut payload = make_payload();
    payload.language_code = "XX".to_string();

    let result = create_trade(
        AuthenticatedUser(User::for_testing()),
        State(state),
        axum::Json(payload),
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::Functional(FunctionalError::InvalidLanguageCode(msg)) => {
            assert_eq!(msg, "XX")
        }
        _ => panic!("Expected InvalidLanguageCode"),
    }
}

#[tokio::test]
async fn create_trade_returns_bad_request_on_invalid_card_id() {
    let mock_use_case = MockCreateTradeUseCase::new();
    let state = make_app_state(mock_use_case);
    let mut payload = make_payload();
    payload.collector_number = "12345678901".to_string();

    let result = create_trade(
        AuthenticatedUser(User::for_testing()),
        State(state),
        axum::Json(payload),
    )
    .await;

    assert!(matches!(
        result,
        Err(AppError::Functional(
            FunctionalError::InvalidCollectorNumber(_)
        ))
    ));
}

#[tokio::test]
async fn create_trade_returns_bad_request_when_quantity_is_zero() {
    let mock_use_case = MockCreateTradeUseCase::new();
    let state = make_app_state(mock_use_case);
    let mut payload = make_payload();
    payload.quantity = 0;

    let result = create_trade(
        AuthenticatedUser(User::for_testing()),
        State(state),
        axum::Json(payload),
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::Functional(FunctionalError::WrongFormat(msg)) => {
            assert_eq!(msg, "quantity must be at least 1")
        }
        _ => panic!("Expected WrongFormat"),
    }
}

#[tokio::test]
async fn create_trade_propagates_card_not_found_from_use_case() {
    let mut mock_use_case = MockCreateTradeUseCase::new();
    mock_use_case
        .expect_create_trade()
        .times(1)
        .returning(|_, _, _, _| {
            Box::pin(async { Err(AppError::Functional(FunctionalError::CardNotFound)) })
        });

    let state = make_app_state(mock_use_case);
    let result = create_trade(
        AuthenticatedUser(User::for_testing()),
        State(state),
        axum::Json(make_payload()),
    )
    .await;

    assert!(matches!(
        result,
        Err(AppError::Functional(FunctionalError::CardNotFound))
    ));
}

#[tokio::test]
async fn create_trade_propagates_self_trade_from_use_case() {
    let mut mock_use_case = MockCreateTradeUseCase::new();
    mock_use_case
        .expect_create_trade()
        .times(1)
        .returning(|_, _, _, _| {
            Box::pin(async { Err(AppError::Functional(FunctionalError::SelfTrade)) })
        });

    let state = make_app_state(mock_use_case);
    let result = create_trade(
        AuthenticatedUser(User::for_testing()),
        State(state),
        axum::Json(make_payload()),
    )
    .await;

    assert!(matches!(
        result,
        Err(AppError::Functional(FunctionalError::SelfTrade))
    ));
}

#[tokio::test]
async fn create_trade_propagates_trade_not_modifiable_from_use_case() {
    let mut mock_use_case = MockCreateTradeUseCase::new();
    mock_use_case
        .expect_create_trade()
        .times(1)
        .returning(|_, _, _, _| {
            Box::pin(async { Err(AppError::Functional(FunctionalError::TradeNotModifiable)) })
        });

    let state = make_app_state(mock_use_case);
    let result = create_trade(
        AuthenticatedUser(User::for_testing()),
        State(state),
        axum::Json(make_payload()),
    )
    .await;

    assert!(matches!(
        result,
        Err(AppError::Functional(FunctionalError::TradeNotModifiable))
    ));
}

// --- accept_trade ---

fn make_app_state_accept(accept_trade_use_case: MockAcceptTradeUseCase) -> AppState {
    AppState::for_testing_with_accept_trade(
        Arc::new(MockStatsUseCase::new()),
        Arc::new(accept_trade_use_case),
    )
}

#[tokio::test]
async fn accept_trade_returns_no_content_on_success() {
    let mut mock_use_case = MockAcceptTradeUseCase::new();
    mock_use_case
        .expect_accept()
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(()) }));

    let state = make_app_state_accept(mock_use_case);
    let result = accept_trade(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Path(uuid::Uuid::new_v4()),
    )
    .await;

    assert_eq!(result.unwrap(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn accept_trade_propagates_trade_not_found_from_use_case() {
    let mut mock_use_case = MockAcceptTradeUseCase::new();
    mock_use_case.expect_accept().times(1).returning(|_, _| {
        Box::pin(async { Err(AppError::Functional(FunctionalError::TradeNotFound)) })
    });

    let state = make_app_state_accept(mock_use_case);
    let result = accept_trade(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Path(uuid::Uuid::new_v4()),
    )
    .await;

    assert!(matches!(
        result,
        Err(AppError::Functional(FunctionalError::TradeNotFound))
    ));
}

#[tokio::test]
async fn accept_trade_propagates_trade_access_denied_from_use_case() {
    let mut mock_use_case = MockAcceptTradeUseCase::new();
    mock_use_case.expect_accept().times(1).returning(|_, _| {
        Box::pin(async { Err(AppError::Functional(FunctionalError::TradeAccessDenied)) })
    });

    let state = make_app_state_accept(mock_use_case);
    let result = accept_trade(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Path(uuid::Uuid::new_v4()),
    )
    .await;

    assert!(matches!(
        result,
        Err(AppError::Functional(FunctionalError::TradeAccessDenied))
    ));
}

#[tokio::test]
async fn accept_trade_propagates_trade_already_accepted_from_use_case() {
    let mut mock_use_case = MockAcceptTradeUseCase::new();
    mock_use_case.expect_accept().times(1).returning(|_, _| {
        Box::pin(async { Err(AppError::Functional(FunctionalError::TradeAlreadyAccepted)) })
    });

    let state = make_app_state_accept(mock_use_case);
    let result = accept_trade(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Path(uuid::Uuid::new_v4()),
    )
    .await;

    assert!(matches!(
        result,
        Err(AppError::Functional(FunctionalError::TradeAlreadyAccepted))
    ));
}

#[tokio::test]
async fn accept_trade_propagates_trade_not_acceptable_from_use_case() {
    let mut mock_use_case = MockAcceptTradeUseCase::new();
    mock_use_case.expect_accept().times(1).returning(|_, _| {
        Box::pin(async { Err(AppError::Functional(FunctionalError::TradeNotAcceptable)) })
    });

    let state = make_app_state_accept(mock_use_case);
    let result = accept_trade(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Path(uuid::Uuid::new_v4()),
    )
    .await;

    assert!(matches!(
        result,
        Err(AppError::Functional(FunctionalError::TradeNotAcceptable))
    ));
}

// --- abandon_trade ---

fn make_app_state_abandon(abandon_trade_use_case: MockAbandonTradeUseCase) -> AppState {
    AppState::for_testing_with_abandon_trade(
        Arc::new(MockStatsUseCase::new()),
        Arc::new(abandon_trade_use_case),
    )
}

#[tokio::test]
async fn abandon_trade_returns_no_content_on_success() {
    let mut mock_use_case = MockAbandonTradeUseCase::new();
    mock_use_case
        .expect_abandon()
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(()) }));

    let state = make_app_state_abandon(mock_use_case);
    let result = abandon_trade(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Path(uuid::Uuid::new_v4()),
    )
    .await;

    assert_eq!(result.unwrap(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn abandon_trade_propagates_trade_not_found_from_use_case() {
    let mut mock_use_case = MockAbandonTradeUseCase::new();
    mock_use_case.expect_abandon().times(1).returning(|_, _| {
        Box::pin(async { Err(AppError::Functional(FunctionalError::TradeNotFound)) })
    });

    let state = make_app_state_abandon(mock_use_case);
    let result = abandon_trade(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Path(uuid::Uuid::new_v4()),
    )
    .await;

    assert!(matches!(
        result,
        Err(AppError::Functional(FunctionalError::TradeNotFound))
    ));
}

#[tokio::test]
async fn abandon_trade_propagates_trade_access_denied_from_use_case() {
    let mut mock_use_case = MockAbandonTradeUseCase::new();
    mock_use_case.expect_abandon().times(1).returning(|_, _| {
        Box::pin(async { Err(AppError::Functional(FunctionalError::TradeAccessDenied)) })
    });

    let state = make_app_state_abandon(mock_use_case);
    let result = abandon_trade(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Path(uuid::Uuid::new_v4()),
    )
    .await;

    assert!(matches!(
        result,
        Err(AppError::Functional(FunctionalError::TradeAccessDenied))
    ));
}

#[tokio::test]
async fn abandon_trade_propagates_trade_already_finalized_from_use_case() {
    let mut mock_use_case = MockAbandonTradeUseCase::new();
    mock_use_case.expect_abandon().times(1).returning(|_, _| {
        Box::pin(async { Err(AppError::Functional(FunctionalError::TradeAlreadyFinalized)) })
    });

    let state = make_app_state_abandon(mock_use_case);
    let result = abandon_trade(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Path(uuid::Uuid::new_v4()),
    )
    .await;

    assert!(matches!(
        result,
        Err(AppError::Functional(FunctionalError::TradeAlreadyFinalized))
    ));
}

// --- confirm_trade ---

fn make_app_state_confirm(confirm_trade_use_case: MockConfirmTradeUseCase) -> AppState {
    AppState::for_testing_with_confirm_trade(
        Arc::new(MockStatsUseCase::new()),
        Arc::new(confirm_trade_use_case),
    )
}

#[tokio::test]
async fn confirm_trade_returns_no_content_on_success() {
    let mut mock_use_case = MockConfirmTradeUseCase::new();
    mock_use_case
        .expect_confirm()
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(()) }));

    let state = make_app_state_confirm(mock_use_case);
    let result = confirm_trade(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Path(uuid::Uuid::new_v4()),
    )
    .await;

    assert_eq!(result.unwrap(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn confirm_trade_propagates_trade_not_found_from_use_case() {
    let mut mock_use_case = MockConfirmTradeUseCase::new();
    mock_use_case.expect_confirm().times(1).returning(|_, _| {
        Box::pin(async { Err(AppError::Functional(FunctionalError::TradeNotFound)) })
    });

    let state = make_app_state_confirm(mock_use_case);
    let result = confirm_trade(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Path(uuid::Uuid::new_v4()),
    )
    .await;

    assert!(matches!(
        result,
        Err(AppError::Functional(FunctionalError::TradeNotFound))
    ));
}

#[tokio::test]
async fn confirm_trade_propagates_trade_access_denied_from_use_case() {
    let mut mock_use_case = MockConfirmTradeUseCase::new();
    mock_use_case.expect_confirm().times(1).returning(|_, _| {
        Box::pin(async { Err(AppError::Functional(FunctionalError::TradeAccessDenied)) })
    });

    let state = make_app_state_confirm(mock_use_case);
    let result = confirm_trade(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Path(uuid::Uuid::new_v4()),
    )
    .await;

    assert!(matches!(
        result,
        Err(AppError::Functional(FunctionalError::TradeAccessDenied))
    ));
}

#[tokio::test]
async fn confirm_trade_propagates_trade_already_confirmed_from_use_case() {
    let mut mock_use_case = MockConfirmTradeUseCase::new();
    mock_use_case.expect_confirm().times(1).returning(|_, _| {
        Box::pin(async { Err(AppError::Functional(FunctionalError::TradeAlreadyConfirmed)) })
    });

    let state = make_app_state_confirm(mock_use_case);
    let result = confirm_trade(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Path(uuid::Uuid::new_v4()),
    )
    .await;

    assert!(matches!(
        result,
        Err(AppError::Functional(FunctionalError::TradeAlreadyConfirmed))
    ));
}

#[tokio::test]
async fn confirm_trade_propagates_trade_not_fully_accepted_from_use_case() {
    let mut mock_use_case = MockConfirmTradeUseCase::new();
    mock_use_case.expect_confirm().times(1).returning(|_, _| {
        Box::pin(async { Err(AppError::Functional(FunctionalError::TradeNotFullyAccepted)) })
    });

    let state = make_app_state_confirm(mock_use_case);
    let result = confirm_trade(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Path(uuid::Uuid::new_v4()),
    )
    .await;

    assert!(matches!(
        result,
        Err(AppError::Functional(FunctionalError::TradeNotFullyAccepted))
    ));
}

// --- rate_trade ---

fn make_app_state_rate(rate_trade_use_case: MockRateTradeUseCase) -> AppState {
    AppState::for_testing_with_rate_trade(
        Arc::new(MockStatsUseCase::new()),
        Arc::new(rate_trade_use_case),
    )
}

fn make_rate_payload(rating: u8) -> RateTradeRequest {
    RateTradeRequest { rating }
}

#[tokio::test]
async fn rate_trade_returns_no_content_on_success() {
    let mut mock_use_case = MockRateTradeUseCase::new();
    mock_use_case
        .expect_rate()
        .times(1)
        .returning(|_, _, _| Box::pin(async { Ok(()) }));

    let state = make_app_state_rate(mock_use_case);
    let result = rate_trade(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Path(uuid::Uuid::new_v4()),
        axum::Json(make_rate_payload(5)),
    )
    .await;

    assert_eq!(result.unwrap(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn rate_trade_returns_bad_request_when_rating_out_of_range() {
    let mock_use_case = MockRateTradeUseCase::new();
    let state = make_app_state_rate(mock_use_case);

    let result = rate_trade(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Path(uuid::Uuid::new_v4()),
        axum::Json(make_rate_payload(6)),
    )
    .await;

    assert!(matches!(
        result,
        Err(AppError::Functional(FunctionalError::WrongFormat(_)))
    ));
}

#[tokio::test]
async fn rate_trade_propagates_trade_not_found_from_use_case() {
    let mut mock_use_case = MockRateTradeUseCase::new();
    mock_use_case.expect_rate().times(1).returning(|_, _, _| {
        Box::pin(async { Err(AppError::Functional(FunctionalError::TradeNotFound)) })
    });

    let state = make_app_state_rate(mock_use_case);
    let result = rate_trade(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Path(uuid::Uuid::new_v4()),
        axum::Json(make_rate_payload(5)),
    )
    .await;

    assert!(matches!(
        result,
        Err(AppError::Functional(FunctionalError::TradeNotFound))
    ));
}

#[tokio::test]
async fn rate_trade_propagates_trade_access_denied_from_use_case() {
    let mut mock_use_case = MockRateTradeUseCase::new();
    mock_use_case.expect_rate().times(1).returning(|_, _, _| {
        Box::pin(async { Err(AppError::Functional(FunctionalError::TradeAccessDenied)) })
    });

    let state = make_app_state_rate(mock_use_case);
    let result = rate_trade(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Path(uuid::Uuid::new_v4()),
        axum::Json(make_rate_payload(5)),
    )
    .await;

    assert!(matches!(
        result,
        Err(AppError::Functional(FunctionalError::TradeAccessDenied))
    ));
}

#[tokio::test]
async fn rate_trade_propagates_trade_already_rated_from_use_case() {
    let mut mock_use_case = MockRateTradeUseCase::new();
    mock_use_case.expect_rate().times(1).returning(|_, _, _| {
        Box::pin(async { Err(AppError::Functional(FunctionalError::TradeAlreadyRated)) })
    });

    let state = make_app_state_rate(mock_use_case);
    let result = rate_trade(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Path(uuid::Uuid::new_v4()),
        axum::Json(make_rate_payload(5)),
    )
    .await;

    assert!(matches!(
        result,
        Err(AppError::Functional(FunctionalError::TradeAlreadyRated))
    ));
}

#[tokio::test]
async fn rate_trade_propagates_trade_not_completed_from_use_case() {
    let mut mock_use_case = MockRateTradeUseCase::new();
    mock_use_case.expect_rate().times(1).returning(|_, _, _| {
        Box::pin(async { Err(AppError::Functional(FunctionalError::TradeNotCompleted)) })
    });

    let state = make_app_state_rate(mock_use_case);
    let result = rate_trade(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Path(uuid::Uuid::new_v4()),
        axum::Json(make_rate_payload(5)),
    )
    .await;

    assert!(matches!(
        result,
        Err(AppError::Functional(FunctionalError::TradeNotCompleted))
    ));
}
