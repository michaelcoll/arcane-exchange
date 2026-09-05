use super::controller::*;
use crate::application::error::{AppError, InfraError};
use crate::application::use_case::{MockGetSetUseCase, MockListSetsUseCase, MockStatsUseCase};
use crate::domain::set_name::{SetCode, SetName};
use crate::infrastructure::AppState;
use axum::extract::{Path, State};
use std::sync::Arc;

fn make_app_state(
    list_sets_use_case: MockListSetsUseCase,
    get_set_use_case: MockGetSetUseCase,
) -> AppState {
    AppState {
        list_sets_use_case: Arc::new(list_sets_use_case),
        get_set_use_case: Arc::new(get_set_use_case),
        ..AppState::for_testing(Arc::new(MockStatsUseCase::new()))
    }
}

#[tokio::test]
async fn list_sets_delegates_to_use_case() {
    let mut mock_list = MockListSetsUseCase::new();
    mock_list.expect_list_sets().returning(|| {
        Box::pin(async {
            Ok(vec![SetName {
                code: SetCode::new("ECL"),
                name: "Eclipsed".to_string(),
            }])
        })
    });

    let state = make_app_state(mock_list, MockGetSetUseCase::new());

    let result = list_sets(State(state)).await;

    assert!(result.is_ok());
    let body = result.unwrap().0;
    assert_eq!(body.len(), 1);
    assert_eq!(body[0].code, "ECL");
    assert_eq!(body[0].name, "Eclipsed");
}

#[tokio::test]
async fn list_sets_propagates_use_case_error() {
    let mut mock_list = MockListSetsUseCase::new();
    mock_list.expect_list_sets().returning(|| {
        Box::pin(async {
            Err(AppError::Infra(InfraError::RepositoryError(
                "db error".to_string(),
            )))
        })
    });

    let state = make_app_state(mock_list, MockGetSetUseCase::new());

    let result = list_sets(State(state)).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn get_set_delegates_to_use_case() {
    let mut mock_get = MockGetSetUseCase::new();
    mock_get.expect_get_set().returning(|code| {
        let code = code.clone();
        Box::pin(async move {
            Ok(SetName {
                code,
                name: "Eclipsed".to_string(),
            })
        })
    });

    let state = make_app_state(MockListSetsUseCase::new(), mock_get);

    let result = get_set(State(state), Path("ECL".to_string())).await;

    assert!(result.is_ok());
    let body = result.unwrap().0;
    assert_eq!(body.code, "ECL");
    assert_eq!(body.name, "Eclipsed");
}

#[tokio::test]
async fn get_set_returns_error_for_invalid_set_code() {
    let state = make_app_state(MockListSetsUseCase::new(), MockGetSetUseCase::new());

    let result = get_set(State(state), Path("AB".to_string())).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn get_set_propagates_not_found_from_use_case() {
    use crate::domain::error::FunctionalError;

    let mut mock_get = MockGetSetUseCase::new();
    mock_get
        .expect_get_set()
        .returning(|_| Box::pin(async { Err(FunctionalError::SetNotFound.into()) }));

    let state = make_app_state(MockListSetsUseCase::new(), mock_get);

    let result = get_set(State(state), Path("XXX".to_string())).await;

    assert!(matches!(
        result.unwrap_err(),
        AppError::Functional(FunctionalError::SetNotFound)
    ));
}
