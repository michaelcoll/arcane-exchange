use super::controller::*;
use crate::application::error::{AppError, InfraError};
use crate::application::use_case::{MockAutocompleteUsersUseCase, MockStatsUseCase};
use crate::domain::user::UserSuggestion;
use crate::infrastructure::AppState;
use crate::infrastructure::adapter_in::autocomplete::dto::AutocompleteUserParams;
use axum::extract::State;
use axum_extra::extract::Query;
use std::sync::Arc;

fn make_app_state(autocomplete_users_use_case: MockAutocompleteUsersUseCase) -> AppState {
    AppState {
        autocomplete_users_use_case: Arc::new(autocomplete_users_use_case),
        ..AppState::for_testing(Arc::new(MockStatsUseCase::new()))
    }
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

    let state = make_app_state(mock);
    let params = AutocompleteUserParams {
        q: Some("ali".to_string()),
    };

    let result = autocomplete_user(State(state), Query(params)).await;

    assert!(result.is_ok());
    let body = result.unwrap().0;
    assert_eq!(body.len(), 1);
    assert_eq!(body[0].username, "alice");
    assert_eq!(body[0].note, 5);
    assert_eq!(body[0].card_count, 42);
}

#[tokio::test]
async fn autocomplete_user_returns_empty_list_when_use_case_returns_empty() {
    let mut mock = MockAutocompleteUsersUseCase::new();
    mock.expect_autocomplete()
        .returning(|_| Box::pin(async { Ok(vec![]) }));

    let state = make_app_state(mock);
    let params = AutocompleteUserParams { q: None };

    let result = autocomplete_user(State(state), Query(params)).await;

    assert!(result.is_ok());
    assert!(result.unwrap().0.is_empty());
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

    let state = make_app_state(mock);
    let params = AutocompleteUserParams {
        q: Some("ali".to_string()),
    };

    let result = autocomplete_user(State(state), Query(params)).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::Infra(InfraError::RepositoryError(msg)) => assert_eq!(msg, "db error"),
        _ => panic!("Expected RepositoryError"),
    }
}
