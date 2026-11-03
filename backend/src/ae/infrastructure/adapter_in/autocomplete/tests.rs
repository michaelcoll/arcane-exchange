use crate::application::error::{AppError, InfraError};
use crate::application::use_case::MockAutocompleteUsersUseCase;
use crate::domain::user::UserSuggestion;
use crate::infrastructure::testing::{anonymous, assert_error, json_of};
use crate::infrastructure::{AppState, create_router};
use axum::http::{Method, StatusCode};
use serde_json::json;
use std::sync::Arc;

fn app_with(autocomplete_users_use_case: MockAutocompleteUsersUseCase) -> axum::Router {
    create_router(AppState {
        autocomplete_users_use_case: Arc::new(autocomplete_users_use_case),
        ..AppState::for_testing()
    })
}

#[tokio::test]
async fn autocomplete_user_delegates_to_use_case_and_maps_note_to_five() {
    let mut mock = MockAutocompleteUsersUseCase::new();
    mock.expect_autocomplete()
        .withf(|q| q.as_deref() == Some("ali"))
        .returning(|_| {
            Box::pin(async {
                Ok(vec![UserSuggestion {
                    username: "alice".to_string(),
                    card_count: 42,
                }])
            })
        });

    let response = anonymous(app_with(mock), Method::GET, "/autocomplete/user?q=ali").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        json_of(response).await,
        json!([{ "username": "alice", "note": 5, "card_count": 42 }])
    );
}

#[tokio::test]
async fn autocomplete_user_without_query_passes_none_and_returns_empty_list() {
    let mut mock = MockAutocompleteUsersUseCase::new();
    mock.expect_autocomplete()
        .withf(|q| q.is_none())
        .returning(|_| Box::pin(async { Ok(vec![]) }));

    let response = anonymous(app_with(mock), Method::GET, "/autocomplete/user").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(json_of(response).await, json!([]));
}

#[tokio::test]
async fn autocomplete_user_propagates_use_case_error() {
    let mut mock = MockAutocompleteUsersUseCase::new();
    mock.expect_autocomplete().returning(|_| {
        Box::pin(async {
            Err(AppError::Infra(InfraError::RepositoryError(
                "db error".to_string(),
            )))
        })
    });

    let response = anonymous(app_with(mock), Method::GET, "/autocomplete/user?q=ali").await;

    assert_error(response, StatusCode::INTERNAL_SERVER_ERROR, "internal").await;
}
