use crate::application::error::{AppError, InfraError};
use crate::application::use_case::{MockGetSetUseCase, MockListSetsUseCase};
use crate::domain::error::FunctionalError;
use crate::domain::set_name::{SetCode, SetName};
use crate::infrastructure::testing::{anonymous, assert_error, json_of};
use crate::infrastructure::{AppState, create_router};
use axum::http::{Method, StatusCode};
use serde_json::json;
use std::sync::Arc;

fn app_with(
    list_sets_use_case: MockListSetsUseCase,
    get_set_use_case: MockGetSetUseCase,
) -> axum::Router {
    create_router(AppState {
        list_sets_use_case: Arc::new(list_sets_use_case),
        get_set_use_case: Arc::new(get_set_use_case),
        ..AppState::for_testing()
    })
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

    let app = app_with(mock_list, MockGetSetUseCase::new());
    let response = anonymous(app, Method::GET, "/sets").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        json_of(response).await,
        json!([{ "code": "ECL", "name": "Eclipsed" }])
    );
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

    let app = app_with(mock_list, MockGetSetUseCase::new());
    let response = anonymous(app, Method::GET, "/sets").await;

    assert_error(response, StatusCode::INTERNAL_SERVER_ERROR, "internal").await;
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

    let app = app_with(MockListSetsUseCase::new(), mock_get);
    let response = anonymous(app, Method::GET, "/sets/ECL").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        json_of(response).await,
        json!({ "code": "ECL", "name": "Eclipsed" })
    );
}

#[tokio::test]
async fn get_set_returns_bad_request_for_invalid_set_code() {
    let app = app_with(MockListSetsUseCase::new(), MockGetSetUseCase::new());
    let response = anonymous(app, Method::GET, "/sets/AB").await;

    assert_error(response, StatusCode::BAD_REQUEST, "invalid_set_code").await;
}

#[tokio::test]
async fn get_set_propagates_not_found_from_use_case() {
    let mut mock_get = MockGetSetUseCase::new();
    mock_get
        .expect_get_set()
        .returning(|_| Box::pin(async { Err(FunctionalError::SetNotFound.into()) }));

    let app = app_with(MockListSetsUseCase::new(), mock_get);
    let response = anonymous(app, Method::GET, "/sets/XXX").await;

    assert_error(response, StatusCode::NOT_FOUND, "set_not_found").await;
}
