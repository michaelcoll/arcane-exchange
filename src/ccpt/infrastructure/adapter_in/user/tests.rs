use super::controller::*;
use super::dto::{CollectionVisibilityParam, SetVisibilityRequest};
use crate::application::error::{AppError, InfraError};
use crate::application::use_case::{
    MockGetCollectionVisibilityUseCase, MockRegisterUserUseCase,
    MockSetCollectionVisibilityUseCase, MockStatsUseCase,
};
use crate::domain::error::FunctionalError;
use crate::domain::user::{CollectionVisibility, User};
use crate::infrastructure::AppState;
use crate::infrastructure::adapter_in::auth_extractor::AuthenticatedUser;
use axum::extract::State;
use axum::http::StatusCode;
use std::sync::Arc;

fn make_app_state(register_user_use_case: MockRegisterUserUseCase) -> AppState {
    AppState {
        register_user_use_case: Arc::new(register_user_use_case),
        ..AppState::for_testing(Arc::new(MockStatsUseCase::new()))
    }
}

fn make_app_state_get_visibility(uc: MockGetCollectionVisibilityUseCase) -> AppState {
    AppState {
        get_collection_visibility_use_case: Arc::new(uc),
        ..AppState::for_testing(Arc::new(MockStatsUseCase::new()))
    }
}

fn make_app_state_set_visibility(uc: MockSetCollectionVisibilityUseCase) -> AppState {
    AppState {
        set_collection_visibility_use_case: Arc::new(uc),
        ..AppState::for_testing(Arc::new(MockStatsUseCase::new()))
    }
}

#[tokio::test]
async fn register_returns_no_content_when_username_present() {
    let mut mock_register = MockRegisterUserUseCase::new();
    mock_register
        .expect_register_user()
        .times(1)
        .returning(|_| Box::pin(async { Ok(()) }));

    let state = make_app_state(mock_register);
    let user = User::new(
        "user_clerk123".to_string(),
        None,
        Some("testuser".to_string()),
    );

    let result = register(State(state), AuthenticatedUser(user)).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn register_returns_bad_request_when_username_missing() {
    let mock_register = MockRegisterUserUseCase::new();
    let state = make_app_state(mock_register);
    let user = User::new("user_clerk123".to_string(), None, None);

    let result = register(State(state), AuthenticatedUser(user)).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::Functional(FunctionalError::WrongFormat(msg)) => {
            assert_eq!(msg, "Missing username claim in token")
        }
        _ => panic!("Expected WrongFormat"),
    }
}

#[tokio::test]
async fn register_propagates_use_case_error() {
    let mut mock_register = MockRegisterUserUseCase::new();
    mock_register
        .expect_register_user()
        .times(1)
        .returning(|_| {
            Box::pin(async {
                Err(AppError::Infra(InfraError::RepositoryError(
                    "DB error".to_string(),
                )))
            })
        });

    let state = make_app_state(mock_register);
    let user = User::new(
        "user_clerk123".to_string(),
        None,
        Some("testuser".to_string()),
    );

    let result = register(State(state), AuthenticatedUser(user)).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::Infra(InfraError::RepositoryError(msg)) => assert_eq!(msg, "DB error"),
        _ => panic!("Expected RepositoryError"),
    }
}

#[tokio::test]
async fn get_visibility_returns_value_on_success() {
    let mut mock_uc = MockGetCollectionVisibilityUseCase::new();
    mock_uc
        .expect_get_visibility()
        .times(1)
        .returning(|_| Box::pin(async { Ok(CollectionVisibility::Trade) }));

    let state = make_app_state_get_visibility(mock_uc);
    let user = User::new(
        "user_clerk123".to_string(),
        None,
        Some("testuser".to_string()),
    );

    let result = get_visibility(State(state), AuthenticatedUser(user)).await;

    let response = result.unwrap();
    assert_eq!(response.0.visibility, CollectionVisibilityParam::Trade);
}

#[tokio::test]
async fn get_visibility_propagates_user_not_found() {
    let mut mock_uc = MockGetCollectionVisibilityUseCase::new();
    mock_uc
        .expect_get_visibility()
        .times(1)
        .returning(|_| Box::pin(async { Err(FunctionalError::UserNotFound.into()) }));

    let state = make_app_state_get_visibility(mock_uc);
    let user = User::new(
        "user_clerk123".to_string(),
        None,
        Some("testuser".to_string()),
    );

    let result = get_visibility(State(state), AuthenticatedUser(user)).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::Functional(FunctionalError::UserNotFound) => {}
        _ => panic!("Expected UserNotFound"),
    }
}

#[tokio::test]
async fn set_visibility_returns_no_content_on_success() {
    let mut mock_uc = MockSetCollectionVisibilityUseCase::new();
    mock_uc
        .expect_set_visibility()
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(()) }));

    let state = make_app_state_set_visibility(mock_uc);
    let user = User::new(
        "user_clerk123".to_string(),
        None,
        Some("testuser".to_string()),
    );

    let result = set_visibility(
        State(state),
        AuthenticatedUser(user),
        axum::Json(SetVisibilityRequest {
            visibility: CollectionVisibilityParam::Public,
        }),
    )
    .await;

    assert_eq!(result.unwrap(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn set_visibility_propagates_user_not_found() {
    let mut mock_uc = MockSetCollectionVisibilityUseCase::new();
    mock_uc
        .expect_set_visibility()
        .times(1)
        .returning(|_, _| Box::pin(async { Err(FunctionalError::UserNotFound.into()) }));

    let state = make_app_state_set_visibility(mock_uc);
    let user = User::new(
        "user_clerk123".to_string(),
        None,
        Some("testuser".to_string()),
    );

    let result = set_visibility(
        State(state),
        AuthenticatedUser(user),
        axum::Json(SetVisibilityRequest {
            visibility: CollectionVisibilityParam::Public,
        }),
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::Functional(FunctionalError::UserNotFound) => {}
        _ => panic!("Expected UserNotFound"),
    }
}
