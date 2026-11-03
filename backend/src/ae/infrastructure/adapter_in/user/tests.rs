use crate::application::error::{AppError, InfraError};
use crate::application::token_validator::MockTokenValidator;
use crate::application::use_case::{MockGetUserProfileUseCase, MockRegisterUserUseCase};
use crate::domain::error::FunctionalError;
use crate::domain::user::User;
use crate::infrastructure::testing::{
    assert_error, assert_routes_refuse_anonymous, authed, json_of,
};
use crate::infrastructure::{AppState, create_router};
use axum::http::{Method, StatusCode};
use mockall::predicate::eq;
use serde_json::json;
use std::sync::Arc;

fn register_app(register_user_use_case: MockRegisterUserUseCase) -> axum::Router {
    create_router(AppState {
        register_user_use_case: Arc::new(register_user_use_case),
        ..AppState::for_testing()
    })
}

fn profile_app(get_user_profile_use_case: MockGetUserProfileUseCase) -> axum::Router {
    create_router(AppState {
        get_user_profile_use_case: Arc::new(get_user_profile_use_case),
        ..AppState::for_testing()
    })
}

#[tokio::test]
async fn register_returns_no_content_when_username_present() {
    let mut mock_register = MockRegisterUserUseCase::new();
    mock_register
        .expect_register_user()
        .withf(|user| *user == User::for_testing())
        .times(1)
        .returning(|_| Box::pin(async { Ok(()) }));

    let response = authed(register_app(mock_register), Method::POST, "/user").await;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn register_returns_bad_request_when_username_missing() {
    let mut mock_auth = MockTokenValidator::new();
    mock_auth
        .expect_validate_token()
        .returning(|_| Ok(User::new("user_clerk123".to_string(), None, None, None)));
    let app = create_router(AppState {
        token_validator: Arc::new(mock_auth),
        ..AppState::for_testing()
    });

    let response = authed(app, Method::POST, "/user").await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = json_of(response).await;
    assert_eq!(body["code"], "wrong_format");
    assert_eq!(body["error"], "Missing username claim in token");
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

    let response = authed(register_app(mock_register), Method::POST, "/user").await;

    assert_error(response, StatusCode::INTERNAL_SERVER_ERROR, "internal").await;
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

    let response = authed(profile_app(mock_uc), Method::GET, "/user/alice").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        json_of(response).await,
        json!({
            "id": "user_clerk123",
            "username": "alice",
            "avatar_url": "https://img.example.com/avatar.png",
        })
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

    let response = authed(profile_app(mock_uc), Method::GET, "/user/bob").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(json_of(response).await["avatar_url"], json!(null));
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

    let response = authed(profile_app(mock_uc), Method::GET, "/user/nobody").await;

    assert_error(response, StatusCode::NOT_FOUND, "user_not_found").await;
}

#[tokio::test]
async fn every_route_refuses_an_anonymous_request() {
    let routes = [(Method::POST, "/user"), (Method::GET, "/user/alice")];

    assert_routes_refuse_anonymous(|| create_router(AppState::for_testing()), &routes).await;
}
