use super::controller::*;
use super::dto::{
    AddTradeCardRequest, CreateTradeRequest, ListTradesParams, RateTradeRequest,
    RemoveTradeCardRequest, TradeStatusParam,
};
use crate::application::error::AppError;
use crate::application::service::trade_service::TRADES_MAX_OFFSET;
use crate::application::use_case::{
    MockAbandonTradeUseCase, MockAcceptTradeUseCase, MockAddTradeCardUseCase,
    MockConfirmTradeUseCase, MockCreateTradeUseCase, MockGetTradeUseCase, MockListTradesUseCase,
    MockRateTradeUseCase, MockRemoveTradeCardUseCase, MockStatsUseCase,
};
use crate::domain::card::CardId;
use crate::domain::error::FunctionalError;
use crate::domain::language_code::LanguageCode;
use crate::domain::pagination::{Paginated, Pagination};
use crate::domain::price::PriceGuide;
use crate::domain::trade::{
    TradeCardDetail, TradeDetail, TradeId, TradePartyState, TradeStatus, TradeSummary,
};
use crate::domain::user::User;
use crate::infrastructure::AppState;
use crate::infrastructure::adapter_in::auth_extractor::AuthenticatedUser;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum_extra::extract::Query;
use std::sync::Arc;

fn make_app_state(create_trade_use_case: MockCreateTradeUseCase) -> AppState {
    AppState::for_testing_with_create_trade(
        Arc::new(MockStatsUseCase::new()),
        Arc::new(create_trade_use_case),
    )
}

fn make_payload() -> CreateTradeRequest {
    CreateTradeRequest {
        respondent_username: "bob".to_string(),
    }
}

#[tokio::test]
async fn create_trade_returns_created_on_nominal_payload() {
    let mut mock_use_case = MockCreateTradeUseCase::new();
    mock_use_case
        .expect_create_trade()
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(TradeId::new()) }));

    let state = make_app_state(mock_use_case);
    let user = User::for_testing();

    let result = create_trade(
        AuthenticatedUser(user),
        State(state),
        axum::Json(make_payload()),
    )
    .await;

    let (status, body) = result.unwrap();
    assert_eq!(status, StatusCode::CREATED);
    assert!(!body.0.id.is_empty());
}

#[tokio::test]
async fn create_trade_propagates_user_not_found_from_use_case() {
    let mut mock_use_case = MockCreateTradeUseCase::new();
    mock_use_case
        .expect_create_trade()
        .times(1)
        .returning(|_, _| {
            Box::pin(async { Err(AppError::Functional(FunctionalError::UserNotFound)) })
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
        Err(AppError::Functional(FunctionalError::UserNotFound))
    ));
}

#[tokio::test]
async fn create_trade_propagates_self_trade_from_use_case() {
    let mut mock_use_case = MockCreateTradeUseCase::new();
    mock_use_case
        .expect_create_trade()
        .times(1)
        .returning(|_, _| {
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

// --- add_trade_card ---

fn make_app_state_add_trade_card(add_trade_card_use_case: MockAddTradeCardUseCase) -> AppState {
    AppState::for_testing_with_add_trade_card(
        Arc::new(MockStatsUseCase::new()),
        Arc::new(add_trade_card_use_case),
    )
}

fn make_add_card_payload() -> AddTradeCardRequest {
    AddTradeCardRequest {
        set_code: "FDN".to_string(),
        collector_number: "87".to_string(),
        language_code: "FR".to_string(),
        foil: false,
        owner_username: "bob".to_string(),
        quantity: 1,
    }
}

#[tokio::test]
async fn add_trade_card_returns_no_content_on_success() {
    let mut mock_use_case = MockAddTradeCardUseCase::new();
    mock_use_case
        .expect_add_card()
        .times(1)
        .returning(|_, _, _, _, _| Box::pin(async { Ok(()) }));

    let state = make_app_state_add_trade_card(mock_use_case);
    let result = add_trade_card(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Path(uuid::Uuid::new_v4()),
        axum::Json(make_add_card_payload()),
    )
    .await;

    assert_eq!(result.unwrap(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn add_trade_card_returns_bad_request_on_invalid_language_code() {
    let mock_use_case = MockAddTradeCardUseCase::new();
    let state = make_app_state_add_trade_card(mock_use_case);
    let mut payload = make_add_card_payload();
    payload.language_code = "XX".to_string();

    let result = add_trade_card(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Path(uuid::Uuid::new_v4()),
        axum::Json(payload),
    )
    .await;

    assert!(matches!(
        result,
        Err(AppError::Functional(FunctionalError::InvalidLanguageCode(
            _
        )))
    ));
}

#[tokio::test]
async fn add_trade_card_returns_bad_request_when_quantity_is_zero() {
    let mock_use_case = MockAddTradeCardUseCase::new();
    let state = make_app_state_add_trade_card(mock_use_case);
    let mut payload = make_add_card_payload();
    payload.quantity = 0;

    let result = add_trade_card(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Path(uuid::Uuid::new_v4()),
        axum::Json(payload),
    )
    .await;

    match result.unwrap_err() {
        AppError::Functional(FunctionalError::WrongFormat(msg)) => {
            assert_eq!(msg, "quantity must be at least 1")
        }
        _ => panic!("Expected WrongFormat"),
    }
}

#[tokio::test]
async fn add_trade_card_propagates_trade_not_found_from_use_case() {
    let mut mock_use_case = MockAddTradeCardUseCase::new();
    mock_use_case
        .expect_add_card()
        .times(1)
        .returning(|_, _, _, _, _| {
            Box::pin(async { Err(AppError::Functional(FunctionalError::TradeNotFound)) })
        });

    let state = make_app_state_add_trade_card(mock_use_case);
    let result = add_trade_card(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Path(uuid::Uuid::new_v4()),
        axum::Json(make_add_card_payload()),
    )
    .await;

    assert!(matches!(
        result,
        Err(AppError::Functional(FunctionalError::TradeNotFound))
    ));
}

#[tokio::test]
async fn add_trade_card_propagates_card_not_found_from_use_case() {
    // Covers both a nonexistent card and a card the owner doesn't actually offer to trade
    // (visibility/binders/rarity filters) — the use case surfaces both as `CardNotFound`.
    let mut mock_use_case = MockAddTradeCardUseCase::new();
    mock_use_case
        .expect_add_card()
        .times(1)
        .returning(|_, _, _, _, _| {
            Box::pin(async { Err(AppError::Functional(FunctionalError::CardNotFound)) })
        });

    let state = make_app_state_add_trade_card(mock_use_case);
    let result = add_trade_card(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Path(uuid::Uuid::new_v4()),
        axum::Json(make_add_card_payload()),
    )
    .await;

    assert!(matches!(
        result,
        Err(AppError::Functional(FunctionalError::CardNotFound))
    ));
}

#[tokio::test]
async fn add_trade_card_propagates_trade_access_denied_from_use_case() {
    let mut mock_use_case = MockAddTradeCardUseCase::new();
    mock_use_case
        .expect_add_card()
        .times(1)
        .returning(|_, _, _, _, _| {
            Box::pin(async { Err(AppError::Functional(FunctionalError::TradeAccessDenied)) })
        });

    let state = make_app_state_add_trade_card(mock_use_case);
    let result = add_trade_card(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Path(uuid::Uuid::new_v4()),
        axum::Json(make_add_card_payload()),
    )
    .await;

    assert!(matches!(
        result,
        Err(AppError::Functional(FunctionalError::TradeAccessDenied))
    ));
}

#[tokio::test]
async fn add_trade_card_propagates_trade_not_modifiable_from_use_case() {
    let mut mock_use_case = MockAddTradeCardUseCase::new();
    mock_use_case
        .expect_add_card()
        .times(1)
        .returning(|_, _, _, _, _| {
            Box::pin(async { Err(AppError::Functional(FunctionalError::TradeNotModifiable)) })
        });

    let state = make_app_state_add_trade_card(mock_use_case);
    let result = add_trade_card(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Path(uuid::Uuid::new_v4()),
        axum::Json(make_add_card_payload()),
    )
    .await;

    assert!(matches!(
        result,
        Err(AppError::Functional(FunctionalError::TradeNotModifiable))
    ));
}

// --- remove_trade_card ---

fn make_app_state_remove_trade_card(
    remove_trade_card_use_case: MockRemoveTradeCardUseCase,
) -> AppState {
    AppState::for_testing_with_remove_trade_card(
        Arc::new(MockStatsUseCase::new()),
        Arc::new(remove_trade_card_use_case),
    )
}

fn make_remove_card_payload() -> RemoveTradeCardRequest {
    RemoveTradeCardRequest {
        set_code: "FDN".to_string(),
        collector_number: "87".to_string(),
        language_code: "FR".to_string(),
        foil: false,
        owner_username: "bob".to_string(),
    }
}

#[tokio::test]
async fn remove_trade_card_returns_no_content_on_success() {
    let mut mock_use_case = MockRemoveTradeCardUseCase::new();
    mock_use_case
        .expect_remove_card()
        .times(1)
        .returning(|_, _, _, _| Box::pin(async { Ok(()) }));

    let state = make_app_state_remove_trade_card(mock_use_case);
    let result = remove_trade_card(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Path(uuid::Uuid::new_v4()),
        axum::Json(make_remove_card_payload()),
    )
    .await;

    assert_eq!(result.unwrap(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn remove_trade_card_returns_bad_request_on_invalid_language_code() {
    let mock_use_case = MockRemoveTradeCardUseCase::new();
    let state = make_app_state_remove_trade_card(mock_use_case);
    let mut payload = make_remove_card_payload();
    payload.language_code = "XX".to_string();

    let result = remove_trade_card(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Path(uuid::Uuid::new_v4()),
        axum::Json(payload),
    )
    .await;

    assert!(matches!(
        result,
        Err(AppError::Functional(FunctionalError::InvalidLanguageCode(
            _
        )))
    ));
}

#[tokio::test]
async fn remove_trade_card_propagates_trade_card_not_found_from_use_case() {
    let mut mock_use_case = MockRemoveTradeCardUseCase::new();
    mock_use_case
        .expect_remove_card()
        .times(1)
        .returning(|_, _, _, _| {
            Box::pin(async { Err(AppError::Functional(FunctionalError::TradeCardNotFound)) })
        });

    let state = make_app_state_remove_trade_card(mock_use_case);
    let result = remove_trade_card(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Path(uuid::Uuid::new_v4()),
        axum::Json(make_remove_card_payload()),
    )
    .await;

    assert!(matches!(
        result,
        Err(AppError::Functional(FunctionalError::TradeCardNotFound))
    ));
}

#[tokio::test]
async fn remove_trade_card_propagates_trade_not_modifiable_from_use_case() {
    let mut mock_use_case = MockRemoveTradeCardUseCase::new();
    mock_use_case
        .expect_remove_card()
        .times(1)
        .returning(|_, _, _, _| {
            Box::pin(async { Err(AppError::Functional(FunctionalError::TradeNotModifiable)) })
        });

    let state = make_app_state_remove_trade_card(mock_use_case);
    let result = remove_trade_card(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Path(uuid::Uuid::new_v4()),
        axum::Json(make_remove_card_payload()),
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

// --- get_trade ---

fn make_app_state_get_trade(get_trade_use_case: MockGetTradeUseCase) -> AppState {
    AppState::for_testing_with_get_trade(
        Arc::new(MockStatsUseCase::new()),
        Arc::new(get_trade_use_case),
    )
}

fn make_trade_detail() -> TradeDetail {
    TradeDetail {
        id: TradeId::new(),
        status: crate::domain::trade::TradeStatus::Pending,
        partner_username: "bob".to_string(),
        my_cards: vec![],
        partner_cards: vec![],
        me: TradePartyState {
            accepted: false,
            confirmed: false,
            rating: None,
        },
        partner: TradePartyState {
            accepted: false,
            confirmed: false,
            rating: None,
        },
    }
}

#[tokio::test]
async fn get_trade_returns_detail_on_success() {
    let mut mock_use_case = MockGetTradeUseCase::new();
    mock_use_case
        .expect_get_trade()
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(make_trade_detail()) }));

    let state = make_app_state_get_trade(mock_use_case);
    let result = get_trade(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Path(uuid::Uuid::new_v4()),
    )
    .await;

    let response = result.unwrap();
    assert_eq!(response.0.partner_username, "bob");
}

#[tokio::test]
async fn get_trade_propagates_trade_not_found() {
    let mut mock_use_case = MockGetTradeUseCase::new();
    mock_use_case.expect_get_trade().times(1).returning(|_, _| {
        Box::pin(async { Err(AppError::Functional(FunctionalError::TradeNotFound)) })
    });

    let state = make_app_state_get_trade(mock_use_case);
    let result = get_trade(
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
async fn get_trade_propagates_trade_access_denied() {
    let mut mock_use_case = MockGetTradeUseCase::new();
    mock_use_case.expect_get_trade().times(1).returning(|_, _| {
        Box::pin(async { Err(AppError::Functional(FunctionalError::TradeAccessDenied)) })
    });

    let state = make_app_state_get_trade(mock_use_case);
    let result = get_trade(
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
async fn get_trade_response_maps_card_details() {
    let card = TradeCardDetail {
        card_id: CardId::new("FDN", "87", LanguageCode::FR, false),
        owner_user_id: crate::domain::user::UserId::new("bob"),
        name: "Goblin Boarders".to_string(),
        quantity: 3,
        price_guide: Some(PriceGuide::new(150u32, 220u32, 200u32)),
        scryfall_id: uuid::Uuid::new_v4(),
        the_gatherer_id: Some("12345".to_string()),
    };
    let mut mock_use_case = MockGetTradeUseCase::new();
    mock_use_case
        .expect_get_trade()
        .times(1)
        .returning(move |_, _| {
            let card = card.clone();
            Box::pin(async move {
                Ok(TradeDetail {
                    partner_cards: vec![card],
                    ..make_trade_detail()
                })
            })
        });

    let state = make_app_state_get_trade(mock_use_case);
    let result = get_trade(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Path(uuid::Uuid::new_v4()),
    )
    .await;

    let response = result.unwrap();
    assert_eq!(response.0.partner_cards.len(), 1);
    let card = &response.0.partner_cards[0];
    assert_eq!(card.set_code, "FDN");
    assert_eq!(card.collector_number, "87");
    assert_eq!(card.name, "Goblin Boarders");
    assert_eq!(card.quantity, 3);
    assert_eq!(card.the_gatherer_id, Some("12345".to_string()));
    let price_guide = card.price_guide.as_ref().unwrap();
    assert_eq!(price_guide.low, Some(150));
    assert_eq!(price_guide.avg, Some(200));
    assert_eq!(price_guide.trend, Some(220));
}

// --- list_trades ---

fn make_app_state_list_trades(list_trades_use_case: MockListTradesUseCase) -> AppState {
    AppState::for_testing_with_list_trades(
        Arc::new(MockStatsUseCase::new()),
        Arc::new(list_trades_use_case),
    )
}

fn make_list_params() -> ListTradesParams {
    ListTradesParams {
        page: 0,
        page_size: 20,
        status: vec![],
    }
}

#[test]
fn list_trades_params_deserializes_with_default_page_and_page_size() {
    let params: ListTradesParams = serde_json::from_value(serde_json::json!({})).unwrap();
    assert_eq!(params.page, 0);
    assert_eq!(params.page_size, 20);
}

#[tokio::test]
async fn list_trades_returns_paginated_response_on_success() {
    let mut mock_use_case = MockListTradesUseCase::new();
    mock_use_case
        .expect_list_trades()
        .times(1)
        .returning(|_, _| {
            Box::pin(async {
                Ok(Paginated {
                    items: vec![],
                    total: 0,
                    pagination: Pagination::try_new(0, 20, TRADES_MAX_OFFSET).unwrap(),
                })
            })
        });

    let state = make_app_state_list_trades(mock_use_case);
    let result = list_trades(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Query(make_list_params()),
    )
    .await;

    let response = result.unwrap();
    assert_eq!(response.0.total, 0);
    assert_eq!(response.0.page_size, 20);
}

#[tokio::test]
async fn list_trades_returns_empty_items_with_nonzero_total_for_page_beyond_last_result() {
    let mut mock_use_case = MockListTradesUseCase::new();
    mock_use_case
        .expect_list_trades()
        .times(1)
        .returning(|_, _| {
            Box::pin(async {
                Ok(Paginated {
                    items: vec![],
                    total: 9,
                    pagination: Pagination::try_new(5, 20, TRADES_MAX_OFFSET).unwrap(),
                })
            })
        });

    let state = make_app_state_list_trades(mock_use_case);
    let result = list_trades(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Query(ListTradesParams {
            page: 5,
            page_size: 20,
            status: vec![],
        }),
    )
    .await;

    let response = result.unwrap();
    assert!(response.0.items.is_empty());
    assert_eq!(response.0.total, 9);
}

#[tokio::test]
async fn list_trades_rejects_page_size_above_max() {
    // The use case must never be reached: the pagination is rejected before that.
    let mock_use_case = MockListTradesUseCase::new();

    let state = make_app_state_list_trades(mock_use_case);
    let result = list_trades(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Query(ListTradesParams {
            page: 0,
            page_size: 500,
            status: vec![],
        }),
    )
    .await;

    match result.err().unwrap() {
        AppError::Functional(FunctionalError::InvalidPageSize {
            requested: 500,
            max: 100,
        }) => {}
        other => panic!("Expected InvalidPageSize, got {:?}", other),
    }
}

#[tokio::test]
async fn list_trades_rejects_offset_beyond_max() {
    let mock_use_case = MockListTradesUseCase::new();

    let state = make_app_state_list_trades(mock_use_case);
    let result = list_trades(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Query(ListTradesParams {
            page: TRADES_MAX_OFFSET,
            page_size: 100,
            status: vec![],
        }),
    )
    .await;

    match result.err().unwrap() {
        AppError::Functional(FunctionalError::PaginationTooDeep { max, .. }) => {
            assert_eq!(max, TRADES_MAX_OFFSET);
        }
        other => panic!("Expected PaginationTooDeep, got {:?}", other),
    }
}

#[tokio::test]
async fn list_trades_accepts_low_page_size_with_high_page_under_the_offset_limit() {
    let mut mock_use_case = MockListTradesUseCase::new();
    mock_use_case
        .expect_list_trades()
        .times(1)
        .returning(|_, query| {
            Box::pin(async move {
                Ok(Paginated {
                    items: vec![],
                    total: 0,
                    pagination: query.pagination,
                })
            })
        });

    let state = make_app_state_list_trades(mock_use_case);
    let result = list_trades(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Query(ListTradesParams {
            page: 200,
            page_size: 10,
            status: vec![],
        }),
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn list_trades_response_maps_trade_summaries() {
    let trade_id = TradeId::new();
    let updated_at = chrono::Utc::now();
    let mut mock_use_case = MockListTradesUseCase::new();
    mock_use_case
        .expect_list_trades()
        .times(1)
        .returning(move |_, _| {
            Box::pin(async move {
                Ok(Paginated {
                    items: vec![TradeSummary {
                        id: trade_id,
                        status: TradeStatus::OneAccepted,
                        partner_username: "bob".to_string(),
                        my_card_count: 2,
                        partner_card_count: 5,
                        updated_at,
                    }],
                    total: 1,
                    pagination: Pagination::try_new(0, 20, TRADES_MAX_OFFSET).unwrap(),
                })
            })
        });

    let state = make_app_state_list_trades(mock_use_case);
    let result = list_trades(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Query(make_list_params()),
    )
    .await;

    let response = result.unwrap();
    assert_eq!(response.0.items.len(), 1);
    let summary = &response.0.items[0];
    assert_eq!(summary.id, trade_id.to_string());
    assert_eq!(summary.status, "ONE_ACCEPTED");
    assert_eq!(summary.partner_username, "bob");
    assert_eq!(summary.my_card_count, 2);
    assert_eq!(summary.partner_card_count, 5);
    assert_eq!(summary.updated_at, updated_at.to_rfc3339());
}

#[tokio::test]
async fn list_trades_maps_every_status_param_variant_to_domain_status() {
    let mut mock_use_case = MockListTradesUseCase::new();
    mock_use_case
        .expect_list_trades()
        .times(1)
        .withf(|_, query| {
            query.statuses
                == vec![
                    TradeStatus::Pending,
                    TradeStatus::OneAccepted,
                    TradeStatus::FullyAccepted,
                    TradeStatus::Completed,
                    TradeStatus::Closed,
                    TradeStatus::Abandoned,
                ]
        })
        .returning(|_, query| {
            Box::pin(async move {
                Ok(Paginated {
                    items: vec![],
                    total: 0,
                    pagination: query.pagination,
                })
            })
        });

    let state = make_app_state_list_trades(mock_use_case);
    let result = list_trades(
        AuthenticatedUser(User::for_testing()),
        State(state),
        Query(ListTradesParams {
            page: 0,
            page_size: 20,
            status: vec![
                TradeStatusParam::Pending,
                TradeStatusParam::OneAccepted,
                TradeStatusParam::FullyAccepted,
                TradeStatusParam::Completed,
                TradeStatusParam::Closed,
                TradeStatusParam::Abandoned,
            ],
        }),
    )
    .await;

    assert!(result.is_ok());
}
