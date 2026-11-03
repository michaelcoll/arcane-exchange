use crate::application::error::{AppError, InfraError};
use crate::application::use_case::{
    MockAddTradeBinderUseCase, MockGetCollectionVisibilityUseCase,
    MockGetRarityTradeFiltersUseCase, MockGetTradeBindersUseCase, MockRemoveTradeBinderUseCase,
    MockSetCollectionVisibilityUseCase, MockSetRarityTradeFilterUseCase,
};
use crate::domain::error::FunctionalError;
use crate::domain::rarity_code::RarityCode;
use crate::domain::rarity_trade_filter::RarityTradeFilter;
use crate::domain::user::CollectionVisibility;
use crate::infrastructure::testing::{
    assert_error, assert_routes_refuse_anonymous, authed, authed_json, json_of,
};
use crate::infrastructure::{AppState, create_router};
use axum::http::{Method, StatusCode};
use serde_json::json;
use std::sync::Arc;

fn db_error() -> AppError {
    AppError::Infra(InfraError::RepositoryError("DB error".to_string()))
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
    let app = create_router(AppState {
        get_collection_visibility_use_case: Arc::new(mock_uc),
        ..AppState::for_testing()
    });

    let response = authed(app, Method::GET, "/collection/trade-settings/visibility").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(json_of(response).await, json!({ "visibility": "trade" }));
}

#[tokio::test]
async fn get_trade_visibility_propagates_user_not_found() {
    let mut mock_uc = MockGetCollectionVisibilityUseCase::new();
    mock_uc
        .expect_get_visibility()
        .times(1)
        .returning(|_| Box::pin(async { Err(FunctionalError::UserNotFound.into()) }));
    let app = create_router(AppState {
        get_collection_visibility_use_case: Arc::new(mock_uc),
        ..AppState::for_testing()
    });

    let response = authed(app, Method::GET, "/collection/trade-settings/visibility").await;

    assert_error(response, StatusCode::NOT_FOUND, "user_not_found").await;
}

#[tokio::test]
async fn set_trade_visibility_returns_no_content_on_success() {
    let mut mock_uc = MockSetCollectionVisibilityUseCase::new();
    mock_uc
        .expect_set_visibility()
        .withf(|_, visibility| *visibility == CollectionVisibility::Public)
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(()) }));
    let app = create_router(AppState {
        set_collection_visibility_use_case: Arc::new(mock_uc),
        ..AppState::for_testing()
    });

    let response = authed_json(
        app,
        Method::PUT,
        "/collection/trade-settings/visibility",
        json!({ "visibility": "public" }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn set_trade_visibility_propagates_user_not_found() {
    let mut mock_uc = MockSetCollectionVisibilityUseCase::new();
    mock_uc
        .expect_set_visibility()
        .times(1)
        .returning(|_, _| Box::pin(async { Err(FunctionalError::UserNotFound.into()) }));
    let app = create_router(AppState {
        set_collection_visibility_use_case: Arc::new(mock_uc),
        ..AppState::for_testing()
    });

    let response = authed_json(
        app,
        Method::PUT,
        "/collection/trade-settings/visibility",
        json!({ "visibility": "public" }),
    )
    .await;

    assert_error(response, StatusCode::NOT_FOUND, "user_not_found").await;
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
    let app = create_router(AppState {
        get_trade_binders_use_case: Arc::new(mock_uc),
        ..AppState::for_testing()
    });

    let response = authed(app, Method::GET, "/collection/trade-settings/binders").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        json_of(response).await,
        json!({ "binders": ["Trade Binder", "Bulk"] })
    );
}

#[tokio::test]
async fn get_trade_binders_propagates_use_case_error() {
    let mut mock_uc = MockGetTradeBindersUseCase::new();
    mock_uc
        .expect_get_trade_binders()
        .times(1)
        .returning(|_| Box::pin(async { Err(db_error()) }));
    let app = create_router(AppState {
        get_trade_binders_use_case: Arc::new(mock_uc),
        ..AppState::for_testing()
    });

    let response = authed(app, Method::GET, "/collection/trade-settings/binders").await;

    assert_error(response, StatusCode::INTERNAL_SERVER_ERROR, "internal").await;
}

#[tokio::test]
async fn add_trade_binder_returns_no_content_on_success() {
    let mut mock_uc = MockAddTradeBinderUseCase::new();
    mock_uc
        .expect_add_trade_binder()
        .withf(|_, name| name == "Trade Binder")
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(()) }));
    let app = create_router(AppState {
        add_trade_binder_use_case: Arc::new(mock_uc),
        ..AppState::for_testing()
    });

    let response = authed_json(
        app,
        Method::POST,
        "/collection/trade-settings/binders",
        json!({ "binder_name": "Trade Binder" }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn add_trade_binder_propagates_binder_not_found() {
    let mut mock_uc = MockAddTradeBinderUseCase::new();
    mock_uc
        .expect_add_trade_binder()
        .times(1)
        .returning(|_, _| Box::pin(async { Err(FunctionalError::BinderNotFound.into()) }));
    let app = create_router(AppState {
        add_trade_binder_use_case: Arc::new(mock_uc),
        ..AppState::for_testing()
    });

    let response = authed_json(
        app,
        Method::POST,
        "/collection/trade-settings/binders",
        json!({ "binder_name": "Unknown" }),
    )
    .await;

    assert_error(response, StatusCode::NOT_FOUND, "binder_not_found").await;
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
    let app = create_router(AppState {
        add_trade_binder_use_case: Arc::new(mock_uc),
        ..AppState::for_testing()
    });

    let response = authed_json(
        app,
        Method::POST,
        "/collection/trade-settings/binders",
        json!({ "binder_name": "   " }),
    )
    .await;

    assert_error(response, StatusCode::BAD_REQUEST, "wrong_format").await;
}

#[tokio::test]
async fn remove_trade_binder_returns_no_content_on_success() {
    let mut mock_uc = MockRemoveTradeBinderUseCase::new();
    mock_uc
        .expect_remove_trade_binder()
        .withf(|_, name| name == "Trade Binder")
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(()) }));
    let app = create_router(AppState {
        remove_trade_binder_use_case: Arc::new(mock_uc),
        ..AppState::for_testing()
    });

    let response = authed(
        app,
        Method::DELETE,
        "/collection/trade-settings/binders/Trade%20Binder",
    )
    .await;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn remove_trade_binder_propagates_use_case_error() {
    let mut mock_uc = MockRemoveTradeBinderUseCase::new();
    mock_uc
        .expect_remove_trade_binder()
        .times(1)
        .returning(|_, _| Box::pin(async { Err(db_error()) }));
    let app = create_router(AppState {
        remove_trade_binder_use_case: Arc::new(mock_uc),
        ..AppState::for_testing()
    });

    let response = authed(
        app,
        Method::DELETE,
        "/collection/trade-settings/binders/Trade%20Binder",
    )
    .await;

    assert_error(response, StatusCode::INTERNAL_SERVER_ERROR, "internal").await;
}

// ============================================================
// rarities
// ============================================================

#[tokio::test]
async fn get_trade_rarity_filters_returns_filters_from_use_case() {
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
    let app = create_router(AppState {
        get_rarity_trade_filters_use_case: Arc::new(mock),
        ..AppState::for_testing()
    });

    let response = authed(app, Method::GET, "/collection/trade-settings/rarities").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        json_of(response).await,
        json!({
            "rarities": [{
                "rarity": "R",
                "is_open": true,
                "kept_copies": 1,
                "copies": 4,
                "proposed": 2,
            }]
        })
    );
}

#[tokio::test]
async fn get_trade_rarity_filters_returns_empty_list() {
    let mut mock = MockGetRarityTradeFiltersUseCase::new();
    mock.expect_get_rarity_trade_filters()
        .returning(|_| Box::pin(async { Ok(vec![]) }));
    let app = create_router(AppState {
        get_rarity_trade_filters_use_case: Arc::new(mock),
        ..AppState::for_testing()
    });

    let response = authed(app, Method::GET, "/collection/trade-settings/rarities").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(json_of(response).await, json!({ "rarities": [] }));
}

#[tokio::test]
async fn get_trade_rarity_filters_propagates_error_from_use_case() {
    let mut mock = MockGetRarityTradeFiltersUseCase::new();
    mock.expect_get_rarity_trade_filters()
        .returning(|_| Box::pin(async { Err(db_error()) }));
    let app = create_router(AppState {
        get_rarity_trade_filters_use_case: Arc::new(mock),
        ..AppState::for_testing()
    });

    let response = authed(app, Method::GET, "/collection/trade-settings/rarities").await;

    assert_error(response, StatusCode::INTERNAL_SERVER_ERROR, "internal").await;
}

#[tokio::test]
async fn set_trade_rarity_filter_forwards_payload_to_use_case() {
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
    let app = create_router(AppState {
        set_rarity_trade_filter_use_case: Arc::new(mock),
        ..AppState::for_testing()
    });

    let response = authed_json(
        app,
        Method::POST,
        "/collection/trade-settings/rarities",
        json!({ "rarity": "M", "is_open": true, "kept_copies": 2 }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn set_trade_rarity_filter_rejects_unknown_rarity_without_calling_use_case() {
    // No expectation: the mock panics if the controller calls it.
    let app = create_router(AppState {
        set_rarity_trade_filter_use_case: Arc::new(MockSetRarityTradeFilterUseCase::new()),
        ..AppState::for_testing()
    });

    let response = authed_json(
        app,
        Method::POST,
        "/collection/trade-settings/rarities",
        json!({ "rarity": "X", "is_open": true, "kept_copies": 1 }),
    )
    .await;

    assert_error(response, StatusCode::BAD_REQUEST, "invalid_rarity_code").await;
}

#[tokio::test]
async fn set_trade_rarity_filter_rejects_negative_kept_copies_without_calling_use_case() {
    let app = create_router(AppState {
        set_rarity_trade_filter_use_case: Arc::new(MockSetRarityTradeFilterUseCase::new()),
        ..AppState::for_testing()
    });

    let response = authed_json(
        app,
        Method::POST,
        "/collection/trade-settings/rarities",
        json!({ "rarity": "M", "is_open": true, "kept_copies": -1 }),
    )
    .await;

    assert_error(response, StatusCode::BAD_REQUEST, "wrong_format").await;
}

#[tokio::test]
async fn set_trade_rarity_filter_propagates_error_from_use_case() {
    let mut mock = MockSetRarityTradeFilterUseCase::new();
    mock.expect_set_rarity_trade_filter().returning(|_, _| {
        Box::pin(async {
            Err(AppError::Functional(FunctionalError::WrongFormat(
                "Kept copies must be between 0 and 4".to_string(),
            )))
        })
    });
    let app = create_router(AppState {
        set_rarity_trade_filter_use_case: Arc::new(mock),
        ..AppState::for_testing()
    });

    let response = authed_json(
        app,
        Method::POST,
        "/collection/trade-settings/rarities",
        json!({ "rarity": "M", "is_open": true, "kept_copies": 5 }),
    )
    .await;

    assert_error(response, StatusCode::BAD_REQUEST, "wrong_format").await;
}

// ============================================================
// authentication
// ============================================================

#[tokio::test]
async fn every_route_refuses_an_anonymous_request() {
    let routes = [
        (Method::GET, "/collection/trade-settings/visibility"),
        (Method::PUT, "/collection/trade-settings/visibility"),
        (Method::GET, "/collection/trade-settings/binders"),
        (Method::POST, "/collection/trade-settings/binders"),
        (Method::DELETE, "/collection/trade-settings/binders/Bulk"),
        (Method::GET, "/collection/trade-settings/rarities"),
        (Method::POST, "/collection/trade-settings/rarities"),
    ];

    assert_routes_refuse_anonymous(|| create_router(AppState::for_testing()), &routes).await;
}
