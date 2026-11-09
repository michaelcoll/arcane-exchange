use crate::application::error::{AppError, InfraError};
use crate::application::use_case::{
    MockCreateBinderUseCase, MockDeleteBinderUseCase, MockListBindersUseCase,
    MockRenameBinderUseCase,
};
use crate::domain::binder::{Binder, BinderId};
use crate::domain::error::FunctionalError;
use crate::infrastructure::testing::{
    assert_error, assert_routes_refuse_anonymous, authed, authed_json, json_of,
};
use crate::infrastructure::{AppState, create_router};
use axum::http::{Method, StatusCode};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

const ID: &str = "7b6c1f0e-3c6d-4d1a-9a7a-1f4f3c7d2e10";

fn binder(name: &str, copies: u64, is_open: bool) -> Binder {
    Binder {
        id: BinderId(Uuid::parse_str(ID).unwrap()),
        name: name.to_string(),
        copies,
        is_open,
    }
}

fn db_error() -> AppError {
    AppError::Infra(InfraError::RepositoryError("DB error".to_string()))
}

#[tokio::test]
async fn list_binders_returns_the_binders() {
    let mut mock_uc = MockListBindersUseCase::new();
    mock_uc.expect_list_binders().times(1).returning(|_| {
        Box::pin(async { Ok(vec![binder("Bulk", 0, false), binder("Trade", 4, true)]) })
    });
    let app = create_router(AppState {
        list_binders_use_case: Arc::new(mock_uc),
        ..AppState::for_testing()
    });

    let response = authed(app, Method::GET, "/collection/binders").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        json_of(response).await,
        json!({ "binders": [
            { "id": ID, "name": "Bulk", "copies": 0, "is_open": false },
            { "id": ID, "name": "Trade", "copies": 4, "is_open": true },
        ] })
    );
}

#[tokio::test]
async fn list_binders_propagates_use_case_error() {
    let mut mock_uc = MockListBindersUseCase::new();
    mock_uc
        .expect_list_binders()
        .times(1)
        .returning(|_| Box::pin(async { Err(db_error()) }));
    let app = create_router(AppState {
        list_binders_use_case: Arc::new(mock_uc),
        ..AppState::for_testing()
    });

    let response = authed(app, Method::GET, "/collection/binders").await;

    assert_error(response, StatusCode::INTERNAL_SERVER_ERROR, "internal").await;
}

#[tokio::test]
async fn create_binder_returns_created_with_the_binder() {
    let mut mock_uc = MockCreateBinderUseCase::new();
    mock_uc
        .expect_create_binder()
        .withf(|_, name| name == "Trade")
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(binder("Trade", 0, false)) }));
    let app = create_router(AppState {
        create_binder_use_case: Arc::new(mock_uc),
        ..AppState::for_testing()
    });

    let response = authed_json(
        app,
        Method::POST,
        "/collection/binders",
        json!({ "name": "Trade" }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(
        json_of(response).await,
        json!({ "id": ID, "name": "Trade", "copies": 0, "is_open": false })
    );
}

#[tokio::test]
async fn create_binder_propagates_refusals() {
    for (error, status, code) in [
        (
            FunctionalError::BinderNameTaken,
            StatusCode::CONFLICT,
            "binder_name_taken",
        ),
        (
            FunctionalError::InvalidBinderName,
            StatusCode::BAD_REQUEST,
            "invalid_binder_name",
        ),
    ] {
        let mut mock_uc = MockCreateBinderUseCase::new();
        mock_uc
            .expect_create_binder()
            .times(1)
            .returning(move |_, _| {
                let error = error.clone();
                Box::pin(async move { Err(error.into()) })
            });
        let app = create_router(AppState {
            create_binder_use_case: Arc::new(mock_uc),
            ..AppState::for_testing()
        });

        let response = authed_json(
            app,
            Method::POST,
            "/collection/binders",
            json!({ "name": "x" }),
        )
        .await;

        assert_error(response, status, code).await;
    }
}

#[tokio::test]
async fn rename_binder_returns_the_renamed_binder() {
    let mut mock_uc = MockRenameBinderUseCase::new();
    mock_uc
        .expect_rename_binder()
        .withf(|_, id, name| *id == BinderId(Uuid::parse_str(ID).unwrap()) && name == "Bulk")
        .times(1)
        .returning(|_, _, _| Box::pin(async { Ok(binder("Bulk", 2, true)) }));
    let app = create_router(AppState {
        rename_binder_use_case: Arc::new(mock_uc),
        ..AppState::for_testing()
    });

    let response = authed_json(
        app,
        Method::PATCH,
        &format!("/collection/binders/{ID}"),
        json!({ "name": "Bulk" }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        json_of(response).await,
        json!({ "id": ID, "name": "Bulk", "copies": 2, "is_open": true })
    );
}

#[tokio::test]
async fn rename_binder_propagates_refusals() {
    for (error, status, code) in [
        (
            FunctionalError::BinderNotFound,
            StatusCode::NOT_FOUND,
            "binder_not_found",
        ),
        (
            FunctionalError::BinderNameTaken,
            StatusCode::CONFLICT,
            "binder_name_taken",
        ),
        (
            FunctionalError::InvalidBinderName,
            StatusCode::BAD_REQUEST,
            "invalid_binder_name",
        ),
    ] {
        let mut mock_uc = MockRenameBinderUseCase::new();
        mock_uc
            .expect_rename_binder()
            .times(1)
            .returning(move |_, _, _| {
                let error = error.clone();
                Box::pin(async move { Err(error.into()) })
            });
        let app = create_router(AppState {
            rename_binder_use_case: Arc::new(mock_uc),
            ..AppState::for_testing()
        });

        let response = authed_json(
            app,
            Method::PATCH,
            &format!("/collection/binders/{ID}"),
            json!({ "name": "x" }),
        )
        .await;

        assert_error(response, status, code).await;
    }
}

#[tokio::test]
async fn rename_binder_treats_an_id_that_is_not_a_uuid_as_unknown() {
    let app = create_router(AppState::for_testing());

    let response = authed_json(
        app,
        Method::PATCH,
        "/collection/binders/not-a-uuid",
        json!({ "name": "x" }),
    )
    .await;

    assert_error(response, StatusCode::NOT_FOUND, "binder_not_found").await;
}

#[tokio::test]
async fn delete_binder_treats_an_id_that_is_not_a_uuid_as_unknown() {
    let app = create_router(AppState::for_testing());

    let response = authed(app, Method::DELETE, "/collection/binders/not-a-uuid").await;

    assert_error(response, StatusCode::NOT_FOUND, "binder_not_found").await;
}

#[tokio::test]
async fn delete_binder_returns_no_content() {
    let mut mock_uc = MockDeleteBinderUseCase::new();
    mock_uc
        .expect_delete_binder()
        .withf(|_, id| *id == BinderId(Uuid::parse_str(ID).unwrap()))
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(()) }));
    let app = create_router(AppState {
        delete_binder_use_case: Arc::new(mock_uc),
        ..AppState::for_testing()
    });

    let response = authed(app, Method::DELETE, &format!("/collection/binders/{ID}")).await;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn delete_binder_propagates_refusals() {
    for (error, status, code) in [
        (
            FunctionalError::BinderNotFound,
            StatusCode::NOT_FOUND,
            "binder_not_found",
        ),
        (
            FunctionalError::BinderNotEmpty,
            StatusCode::CONFLICT,
            "binder_not_empty",
        ),
    ] {
        let mut mock_uc = MockDeleteBinderUseCase::new();
        mock_uc
            .expect_delete_binder()
            .times(1)
            .returning(move |_, _| {
                let error = error.clone();
                Box::pin(async move { Err(error.into()) })
            });
        let app = create_router(AppState {
            delete_binder_use_case: Arc::new(mock_uc),
            ..AppState::for_testing()
        });

        let response = authed(app, Method::DELETE, &format!("/collection/binders/{ID}")).await;

        assert_error(response, status, code).await;
    }
}

#[tokio::test]
async fn binder_routes_refuse_anonymous_requests() {
    let item = format!("/collection/binders/{ID}");
    let routes = [
        (Method::GET, "/collection/binders"),
        (Method::POST, "/collection/binders"),
        (Method::PATCH, item.as_str()),
        (Method::DELETE, item.as_str()),
    ];

    assert_routes_refuse_anonymous(|| create_router(AppState::for_testing()), &routes).await;
}
