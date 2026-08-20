use super::controller::*;
use super::dto::{AddTradeBinderRequest, CollectionVisibilityParam, SetVisibilityRequest};
use crate::application::error::{AppError, InfraError};
use crate::application::use_case::{
    MockAddTradeBinderUseCase, MockGetCollectionVisibilityUseCase, MockGetTradeBindersUseCase,
    MockGetUserProfileUseCase, MockRegisterUserUseCase, MockRemoveTradeBinderUseCase,
    MockSetCollectionVisibilityUseCase, MockStatsUseCase,
};
use crate::domain::error::FunctionalError;
use crate::domain::user::{CollectionVisibility, User};
use crate::infrastructure::AppState;
use crate::infrastructure::adapter_in::auth_extractor::AuthenticatedUser;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use mockall::predicate::eq;
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

fn make_app_state_get_trade_binders(uc: MockGetTradeBindersUseCase) -> AppState {
    AppState {
        get_trade_binders_use_case: Arc::new(uc),
        ..AppState::for_testing(Arc::new(MockStatsUseCase::new()))
    }
}

fn make_app_state_add_trade_binder(uc: MockAddTradeBinderUseCase) -> AppState {
    AppState {
        add_trade_binder_use_case: Arc::new(uc),
        ..AppState::for_testing(Arc::new(MockStatsUseCase::new()))
    }
}

fn make_app_state_remove_trade_binder(uc: MockRemoveTradeBinderUseCase) -> AppState {
    AppState {
        remove_trade_binder_use_case: Arc::new(uc),
        ..AppState::for_testing(Arc::new(MockStatsUseCase::new()))
    }
}

fn make_app_state_get_profile(uc: MockGetUserProfileUseCase) -> AppState {
    AppState {
        get_user_profile_use_case: Arc::new(uc),
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
        None,
    );

    let result = register(State(state), AuthenticatedUser(user)).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn register_returns_bad_request_when_username_missing() {
    let mock_register = MockRegisterUserUseCase::new();
    let state = make_app_state(mock_register);
    let user = User::new("user_clerk123".to_string(), None, None, None);

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
        None,
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
        None,
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
        None,
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
        None,
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
        None,
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

#[tokio::test]
async fn get_trade_binders_returns_binders_on_success() {
    let mut mock_uc = MockGetTradeBindersUseCase::new();
    mock_uc.expect_get_trade_binders().times(1).returning(|_| {
        Box::pin(async { Ok(vec!["Trade Binder".to_string(), "Bulk".to_string()]) })
    });

    let state = make_app_state_get_trade_binders(mock_uc);
    let user = User::new(
        "user_clerk123".to_string(),
        None,
        Some("testuser".to_string()),
        None,
    );

    let result = get_trade_binders(State(state), AuthenticatedUser(user)).await;

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
    let user = User::new(
        "user_clerk123".to_string(),
        None,
        Some("testuser".to_string()),
        None,
    );

    let result = get_trade_binders(State(state), AuthenticatedUser(user)).await;

    assert!(result.is_err());
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
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(()) }));

    let state = make_app_state_add_trade_binder(mock_uc);
    let user = User::new(
        "user_clerk123".to_string(),
        None,
        Some("testuser".to_string()),
        None,
    );

    let result = add_trade_binder(
        State(state),
        AuthenticatedUser(user),
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
    let user = User::new(
        "user_clerk123".to_string(),
        None,
        Some("testuser".to_string()),
        None,
    );

    let result = add_trade_binder(
        State(state),
        AuthenticatedUser(user),
        axum::Json(AddTradeBinderRequest {
            binder_name: "Unknown".to_string(),
        }),
    )
    .await;

    assert!(result.is_err());
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
    let user = User::new(
        "user_clerk123".to_string(),
        None,
        Some("testuser".to_string()),
        None,
    );

    let result = add_trade_binder(
        State(state),
        AuthenticatedUser(user),
        axum::Json(AddTradeBinderRequest {
            binder_name: "   ".to_string(),
        }),
    )
    .await;

    assert!(result.is_err());
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
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(()) }));

    let state = make_app_state_remove_trade_binder(mock_uc);
    let user = User::new(
        "user_clerk123".to_string(),
        None,
        Some("testuser".to_string()),
        None,
    );

    let result = remove_trade_binder(
        State(state),
        AuthenticatedUser(user),
        Path("Trade Binder".to_string()),
    )
    .await;

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
    let user = User::new(
        "user_clerk123".to_string(),
        None,
        Some("testuser".to_string()),
        None,
    );

    let result = remove_trade_binder(
        State(state),
        AuthenticatedUser(user),
        Path("Trade Binder".to_string()),
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::Infra(InfraError::RepositoryError(msg)) => assert_eq!(msg, "DB error"),
        _ => panic!("Expected RepositoryError"),
    }
}

#[tokio::test]
async fn get_user_profile_returns_profile_with_avatar() {
    let mut mock_uc = MockGetUserProfileUseCase::new();
    mock_uc
        .expect_get_user_profile()
        .times(1)
        .with(eq("alice"))
        .returning(|_| {
            Box::pin(async {
                Ok(User::new(
                    "user_clerk123".to_string(),
                    None,
                    Some("alice".to_string()),
                    Some("https://img.example.com/avatar.png".to_string()),
                ))
            })
        });

    let state = make_app_state_get_profile(mock_uc);
    let user = User::new(
        "user_clerk123".to_string(),
        None,
        Some("testuser".to_string()),
        None,
    );

    let result = get_user_profile(
        State(state),
        AuthenticatedUser(user),
        Path("alice".to_string()),
    )
    .await;

    let response = result.unwrap();
    assert_eq!(response.0.id, "user_clerk123");
    assert_eq!(response.0.username, Some("alice".to_string()));
    assert_eq!(
        response.0.avatar_url,
        Some("https://img.example.com/avatar.png".to_string())
    );
}

#[tokio::test]
async fn get_user_profile_returns_null_avatar_when_none_stored() {
    let mut mock_uc = MockGetUserProfileUseCase::new();
    mock_uc
        .expect_get_user_profile()
        .times(1)
        .with(eq("bob"))
        .returning(|_| {
            Box::pin(async {
                Ok(User::new(
                    "user_bob".to_string(),
                    None,
                    Some("bob".to_string()),
                    None,
                ))
            })
        });

    let state = make_app_state_get_profile(mock_uc);
    let user = User::new(
        "user_clerk123".to_string(),
        None,
        Some("testuser".to_string()),
        None,
    );

    let result = get_user_profile(
        State(state),
        AuthenticatedUser(user),
        Path("bob".to_string()),
    )
    .await;

    let response = result.unwrap();
    assert_eq!(response.0.avatar_url, None);
}

#[tokio::test]
async fn get_user_profile_propagates_user_not_found() {
    let mut mock_uc = MockGetUserProfileUseCase::new();
    mock_uc
        .expect_get_user_profile()
        .times(1)
        .with(eq("nobody"))
        .returning(|_| {
            Box::pin(async { Err(AppError::Functional(FunctionalError::UserNotFound)) })
        });

    let state = make_app_state_get_profile(mock_uc);
    let user = User::new(
        "user_clerk123".to_string(),
        None,
        Some("testuser".to_string()),
        None,
    );

    let result = get_user_profile(
        State(state),
        AuthenticatedUser(user),
        Path("nobody".to_string()),
    )
    .await;

    assert!(matches!(
        result.unwrap_err(),
        AppError::Functional(FunctionalError::UserNotFound)
    ));
}
