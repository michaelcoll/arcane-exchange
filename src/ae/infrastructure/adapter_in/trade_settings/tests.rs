use super::controller::*;
use super::dto::{
    AddTradeBinderRequest, CollectionVisibilityParam, SetRarityFilterRequest, SetVisibilityRequest,
};
use crate::application::error::{AppError, InfraError};
use crate::application::use_case::{
    MockAddTradeBinderUseCase, MockGetCollectionVisibilityUseCase,
    MockGetRarityTradeFiltersUseCase, MockGetTradeBindersUseCase, MockRemoveTradeBinderUseCase,
    MockSetCollectionVisibilityUseCase, MockSetRarityTradeFilterUseCase,
};
use crate::domain::error::FunctionalError;
use crate::domain::rarity_code::RarityCode;
use crate::domain::rarity_trade_filter::RarityTradeFilter;
use crate::domain::user::{CollectionVisibility, User};
use crate::infrastructure::AppState;
use crate::infrastructure::adapter_in::auth_extractor::AuthenticatedUser;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use std::sync::Arc;

fn test_user() -> AuthenticatedUser {
    AuthenticatedUser(User::for_testing())
}

fn make_app_state_get_visibility(uc: MockGetCollectionVisibilityUseCase) -> AppState {
    AppState {
        get_collection_visibility_use_case: Arc::new(uc),
        ..AppState::for_testing()
    }
}

fn make_app_state_set_visibility(uc: MockSetCollectionVisibilityUseCase) -> AppState {
    AppState {
        set_collection_visibility_use_case: Arc::new(uc),
        ..AppState::for_testing()
    }
}

fn make_app_state_get_trade_binders(uc: MockGetTradeBindersUseCase) -> AppState {
    AppState {
        get_trade_binders_use_case: Arc::new(uc),
        ..AppState::for_testing()
    }
}

fn make_app_state_add_trade_binder(uc: MockAddTradeBinderUseCase) -> AppState {
    AppState {
        add_trade_binder_use_case: Arc::new(uc),
        ..AppState::for_testing()
    }
}

fn make_app_state_remove_trade_binder(uc: MockRemoveTradeBinderUseCase) -> AppState {
    AppState {
        remove_trade_binder_use_case: Arc::new(uc),
        ..AppState::for_testing()
    }
}

fn make_app_state_get_rarities(uc: MockGetRarityTradeFiltersUseCase) -> AppState {
    AppState {
        get_rarity_trade_filters_use_case: Arc::new(uc),
        ..AppState::for_testing()
    }
}

fn make_app_state_set_rarity(uc: MockSetRarityTradeFilterUseCase) -> AppState {
    AppState {
        set_rarity_trade_filter_use_case: Arc::new(uc),
        ..AppState::for_testing()
    }
}

// ============================================================
// visibility
// ============================================================

#[tokio::test]
async fn get_trade_visibility_returns_value_on_success() {
    let mut mock_uc = MockGetCollectionVisibilityUseCase::new();
    mock_uc
        .expect_get_visibility()
        .times(1)
        .returning(|_| Box::pin(async { Ok(CollectionVisibility::Trade) }));

    let state = make_app_state_get_visibility(mock_uc);

    let result = get_trade_visibility(State(state), test_user()).await;

    let response = result.unwrap();
    assert_eq!(response.0.visibility, CollectionVisibilityParam::Trade);
}

#[tokio::test]
async fn get_trade_visibility_propagates_user_not_found() {
    let mut mock_uc = MockGetCollectionVisibilityUseCase::new();
    mock_uc
        .expect_get_visibility()
        .times(1)
        .returning(|_| Box::pin(async { Err(FunctionalError::UserNotFound.into()) }));

    let state = make_app_state_get_visibility(mock_uc);

    let result = get_trade_visibility(State(state), test_user()).await;

    match result.unwrap_err() {
        AppError::Functional(FunctionalError::UserNotFound) => {}
        _ => panic!("Expected UserNotFound"),
    }
}

#[tokio::test]
async fn set_trade_visibility_returns_no_content_on_success() {
    let mut mock_uc = MockSetCollectionVisibilityUseCase::new();
    mock_uc
        .expect_set_visibility()
        .withf(|_, visibility| *visibility == CollectionVisibility::Public)
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(()) }));

    let state = make_app_state_set_visibility(mock_uc);

    let result = set_trade_visibility(
        State(state),
        test_user(),
        axum::Json(SetVisibilityRequest {
            visibility: CollectionVisibilityParam::Public,
        }),
    )
    .await;

    assert_eq!(result.unwrap(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn set_trade_visibility_propagates_user_not_found() {
    let mut mock_uc = MockSetCollectionVisibilityUseCase::new();
    mock_uc
        .expect_set_visibility()
        .times(1)
        .returning(|_, _| Box::pin(async { Err(FunctionalError::UserNotFound.into()) }));

    let state = make_app_state_set_visibility(mock_uc);

    let result = set_trade_visibility(
        State(state),
        test_user(),
        axum::Json(SetVisibilityRequest {
            visibility: CollectionVisibilityParam::Public,
        }),
    )
    .await;

    match result.unwrap_err() {
        AppError::Functional(FunctionalError::UserNotFound) => {}
        _ => panic!("Expected UserNotFound"),
    }
}

// ============================================================
// binders
// ============================================================

#[tokio::test]
async fn get_trade_binders_returns_binders_on_success() {
    let mut mock_uc = MockGetTradeBindersUseCase::new();
    mock_uc.expect_get_trade_binders().times(1).returning(|_| {
        Box::pin(async { Ok(vec!["Trade Binder".to_string(), "Bulk".to_string()]) })
    });

    let state = make_app_state_get_trade_binders(mock_uc);

    let result = get_trade_binders(State(state), test_user()).await;

    let response = result.unwrap();
    assert_eq!(
        response.0.binders,
        vec!["Trade Binder".to_string(), "Bulk".to_string()]
    );
}

#[tokio::test]
async fn get_trade_binders_propagates_use_case_error() {
    let mut mock_uc = MockGetTradeBindersUseCase::new();
    mock_uc.expect_get_trade_binders().times(1).returning(|_| {
        Box::pin(async {
            Err(AppError::Infra(InfraError::RepositoryError(
                "DB error".to_string(),
            )))
        })
    });

    let state = make_app_state_get_trade_binders(mock_uc);

    let result = get_trade_binders(State(state), test_user()).await;

    match result.unwrap_err() {
        AppError::Infra(InfraError::RepositoryError(msg)) => assert_eq!(msg, "DB error"),
        _ => panic!("Expected RepositoryError"),
    }
}

#[tokio::test]
async fn add_trade_binder_returns_no_content_on_success() {
    let mut mock_uc = MockAddTradeBinderUseCase::new();
    mock_uc
        .expect_add_trade_binder()
        .withf(|_, name| name == "Trade Binder")
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(()) }));

    let state = make_app_state_add_trade_binder(mock_uc);

    let result = add_trade_binder(
        State(state),
        test_user(),
        axum::Json(AddTradeBinderRequest {
            binder_name: "Trade Binder".to_string(),
        }),
    )
    .await;

    assert_eq!(result.unwrap(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn add_trade_binder_propagates_binder_not_found() {
    let mut mock_uc = MockAddTradeBinderUseCase::new();
    mock_uc
        .expect_add_trade_binder()
        .times(1)
        .returning(|_, _| Box::pin(async { Err(FunctionalError::BinderNotFound.into()) }));

    let state = make_app_state_add_trade_binder(mock_uc);

    let result = add_trade_binder(
        State(state),
        test_user(),
        axum::Json(AddTradeBinderRequest {
            binder_name: "Unknown".to_string(),
        }),
    )
    .await;

    match result.unwrap_err() {
        AppError::Functional(FunctionalError::BinderNotFound) => {}
        _ => panic!("Expected BinderNotFound"),
    }
}

#[tokio::test]
async fn add_trade_binder_propagates_wrong_format() {
    let mut mock_uc = MockAddTradeBinderUseCase::new();
    mock_uc
        .expect_add_trade_binder()
        .times(1)
        .returning(|_, _| {
            Box::pin(async {
                Err(FunctionalError::WrongFormat("Binder name is empty".to_string()).into())
            })
        });

    let state = make_app_state_add_trade_binder(mock_uc);

    let result = add_trade_binder(
        State(state),
        test_user(),
        axum::Json(AddTradeBinderRequest {
            binder_name: "   ".to_string(),
        }),
    )
    .await;

    match result.unwrap_err() {
        AppError::Functional(FunctionalError::WrongFormat(_)) => {}
        _ => panic!("Expected WrongFormat"),
    }
}

#[tokio::test]
async fn remove_trade_binder_returns_no_content_on_success() {
    let mut mock_uc = MockRemoveTradeBinderUseCase::new();
    mock_uc
        .expect_remove_trade_binder()
        .withf(|_, name| name == "Trade Binder")
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(()) }));

    let state = make_app_state_remove_trade_binder(mock_uc);

    let result =
        remove_trade_binder(State(state), test_user(), Path("Trade Binder".to_string())).await;

    assert_eq!(result.unwrap(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn remove_trade_binder_propagates_use_case_error() {
    let mut mock_uc = MockRemoveTradeBinderUseCase::new();
    mock_uc
        .expect_remove_trade_binder()
        .times(1)
        .returning(|_, _| {
            Box::pin(async {
                Err(AppError::Infra(InfraError::RepositoryError(
                    "DB error".to_string(),
                )))
            })
        });

    let state = make_app_state_remove_trade_binder(mock_uc);

    let result =
        remove_trade_binder(State(state), test_user(), Path("Trade Binder".to_string())).await;

    match result.unwrap_err() {
        AppError::Infra(InfraError::RepositoryError(msg)) => assert_eq!(msg, "DB error"),
        _ => panic!("Expected RepositoryError"),
    }
}

// ============================================================
// rarities
// ============================================================

#[tokio::test]
async fn get_trade_rarities_returns_filters_from_use_case() {
    let mut mock = MockGetRarityTradeFiltersUseCase::new();
    mock.expect_get_rarity_trade_filters().returning(|_| {
        Box::pin(async {
            Ok(vec![RarityTradeFilter {
                rarity: RarityCode::R,
                is_open: true,
                kept_copies: 1,
                copies: 4,
                proposed: 2,
            }])
        })
    });

    let state = make_app_state_get_rarities(mock);
    let result = get_trade_rarities(State(state), test_user()).await;

    let axum::Json(response) = result.unwrap();
    assert_eq!(response.rarities.len(), 1);
    assert_eq!(response.rarities[0].rarity, "R");
    assert!(response.rarities[0].is_open);
    assert_eq!(response.rarities[0].kept_copies, 1);
    assert_eq!(response.rarities[0].copies, 4);
    assert_eq!(response.rarities[0].proposed, 2);
}

#[tokio::test]
async fn get_trade_rarities_returns_empty_list() {
    let mut mock = MockGetRarityTradeFiltersUseCase::new();
    mock.expect_get_rarity_trade_filters()
        .returning(|_| Box::pin(async { Ok(vec![]) }));

    let state = make_app_state_get_rarities(mock);
    let result = get_trade_rarities(State(state), test_user()).await;

    let axum::Json(response) = result.unwrap();
    assert!(response.rarities.is_empty());
}

#[tokio::test]
async fn get_trade_rarities_propagates_error_from_use_case() {
    let mut mock = MockGetRarityTradeFiltersUseCase::new();
    mock.expect_get_rarity_trade_filters().returning(|_| {
        Box::pin(async {
            Err(AppError::Infra(InfraError::RepositoryError(
                "db failure".to_string(),
            )))
        })
    });

    let state = make_app_state_get_rarities(mock);
    let result = get_trade_rarities(State(state), test_user()).await;

    match result.unwrap_err() {
        AppError::Infra(InfraError::RepositoryError(msg)) => assert_eq!(msg, "db failure"),
        _ => panic!("Expected RepositoryError"),
    }
}

#[tokio::test]
async fn set_trade_rarity_forwards_payload_to_use_case() {
    let mut mock = MockSetRarityTradeFilterUseCase::new();
    mock.expect_set_rarity_trade_filter()
        .withf(|user_id, rule| {
            user_id.as_str() == "test-user-id"
                && rule.rarity == RarityCode::M
                && rule.is_open
                && rule.kept_copies == 2
        })
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(()) }));

    let state = make_app_state_set_rarity(mock);
    let result = set_trade_rarity(
        State(state),
        test_user(),
        axum::Json(SetRarityFilterRequest {
            rarity: "M".to_string(),
            is_open: true,
            kept_copies: 2,
        }),
    )
    .await;

    assert_eq!(result.unwrap(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn set_trade_rarity_rejects_unknown_rarity_without_calling_use_case() {
    let mock = MockSetRarityTradeFilterUseCase::new();

    let state = make_app_state_set_rarity(mock);
    let result = set_trade_rarity(
        State(state),
        test_user(),
        axum::Json(SetRarityFilterRequest {
            rarity: "X".to_string(),
            is_open: true,
            kept_copies: 1,
        }),
    )
    .await;

    match result.unwrap_err() {
        AppError::Functional(FunctionalError::InvalidRarityCode(msg)) => assert_eq!(msg, "X"),
        other => panic!("Expected InvalidRarityCode, got {other:?}"),
    }
}

#[tokio::test]
async fn set_trade_rarity_rejects_negative_kept_copies_without_calling_use_case() {
    let mock = MockSetRarityTradeFilterUseCase::new();

    let state = make_app_state_set_rarity(mock);
    let result = set_trade_rarity(
        State(state),
        test_user(),
        axum::Json(SetRarityFilterRequest {
            rarity: "M".to_string(),
            is_open: true,
            kept_copies: -1,
        }),
    )
    .await;

    match result.unwrap_err() {
        AppError::Functional(FunctionalError::WrongFormat(_)) => {}
        other => panic!("Expected WrongFormat, got {other:?}"),
    }
}

#[tokio::test]
async fn set_trade_rarity_propagates_error_from_use_case() {
    let mut mock = MockSetRarityTradeFilterUseCase::new();
    mock.expect_set_rarity_trade_filter().returning(|_, _| {
        Box::pin(async {
            Err(AppError::Functional(FunctionalError::WrongFormat(
                "Kept copies must be between 0 and 4".to_string(),
            )))
        })
    });

    let state = make_app_state_set_rarity(mock);
    let result = set_trade_rarity(
        State(state),
        test_user(),
        axum::Json(SetRarityFilterRequest {
            rarity: "M".to_string(),
            is_open: true,
            kept_copies: 5,
        }),
    )
    .await;

    match result.unwrap_err() {
        AppError::Functional(FunctionalError::WrongFormat(_)) => {}
        other => panic!("Expected WrongFormat, got {other:?}"),
    }
}
