use crate::application::error::{AppError, InfraError};
use crate::application::use_case::{
    MockCreateSealedProductUseCase, MockDeletePurchaseUseCase, MockGetPurchaseUseCase,
    MockListPurchasesUseCase, MockUpdatePurchaseUseCase,
};
use crate::domain::card::CardId;
use crate::domain::card_image::{CardImage, CardImageSource};
use crate::domain::error::FunctionalError;
use crate::domain::language_code::LanguageCode;
use crate::domain::pagination::{Paginated, Pagination};
use crate::domain::purchase::{
    Purchase, PurchaseAmount, PurchaseContentLine, PurchaseDetail, PurchaseId, PurchaseKind,
    PurchasedCard, SealedProductType,
};
use crate::domain::set_name::{SetCode, SetName};
use crate::infrastructure::testing::{
    assert_error, assert_routes_refuse_anonymous, authed, authed_json, json_of,
};
use crate::infrastructure::{AppState, create_router};
use axum::http::{Method, StatusCode};
use chrono::NaiveDate;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

const ID: &str = "7b6c1f0e-3c6d-4d1a-9a7a-1f4f3c7d2e10";

fn day() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 13).unwrap()
}

fn purchase(name: Option<&str>) -> Purchase {
    Purchase {
        id: PurchaseId(Uuid::parse_str(ID).unwrap()),
        kind: PurchaseKind::SealedProduct {
            product_type: SealedProductType::PreconstructedDeck,
            name: name.map(str::to_string),
            set: SetName {
                code: SetCode::new("FDN"),
                name: "Foundations".to_string(),
            },
        },
        amount: PurchaseAmount::try_new(4999).unwrap(),
        purchased_on: day(),
    }
}

fn purchase_json(name: serde_json::Value) -> serde_json::Value {
    json!({
        "id": ID,
        "kind": "sealed_product",
        "amount_cents": 4999,
        "purchased_on": "2026-11-13",
        "product_type": "PRECONSTRUCTED_DECK",
        "name": name,
        "set": { "code": "FDN", "name": "Foundations" },
        "card": null,
        "foil": null,
        "quantity": null,
    })
}

fn single_card_purchase() -> Purchase {
    Purchase {
        id: PurchaseId(Uuid::parse_str(ID).unwrap()),
        kind: PurchaseKind::SingleCard {
            card: PurchasedCard {
                id: CardId::new("FDN", "87", LanguageCode::FR),
                name: "Gobelin".to_string(),
                set: SetName {
                    code: SetCode::new("FDN"),
                    name: "Foundations".to_string(),
                },
                image: Some(CardImage {
                    source: CardImageSource::GathererLocalized,
                    has_back: false,
                }),
            },
            foil: true,
            quantity: 4,
        },
        amount: PurchaseAmount::try_new(800).unwrap(),
        purchased_on: day(),
    }
}

fn single_card_purchase_json() -> serde_json::Value {
    json!({
        "id": ID,
        "kind": "single_card",
        "amount_cents": 800,
        "purchased_on": "2026-11-13",
        "product_type": null,
        "name": null,
        "set": null,
        "card": {
            "set": { "code": "FDN", "name": "Foundations" },
            "collector_number": "87",
            "language_code": "FR",
            "name": "Gobelin",
            "image_url": "/card-images/FDN_87_FR.webp?v=gatherer",
            "image_back_url": null,
        },
        "foil": true,
        "quantity": 4,
    })
}

#[tokio::test]
async fn get_purchase_returns_a_single_card_with_its_card_finish_and_quantity() {
    let mut mock_uc = MockGetPurchaseUseCase::new();
    mock_uc.expect_get_purchase().times(1).returning(|_, _| {
        Box::pin(async {
            Ok(PurchaseDetail {
                purchase: single_card_purchase(),
                content: vec![],
            })
        })
    });
    let app = create_router(AppState {
        get_purchase_use_case: Arc::new(mock_uc),
        ..AppState::for_testing()
    });

    let response = authed(app, Method::GET, &format!("/purchases/{ID}")).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        json_of(response).await["purchase"],
        single_card_purchase_json()
    );
}

#[tokio::test]
async fn list_purchases_renders_a_single_card_with_its_card() {
    let mut mock_uc = MockListPurchasesUseCase::new();
    mock_uc
        .expect_list_purchases()
        .times(1)
        .returning(|_, page| {
            Box::pin(async move {
                Ok(Paginated {
                    items: vec![single_card_purchase()],
                    total: 1,
                    pagination: Pagination::try_new(page.page, page.page_size, 1_000).unwrap(),
                })
            })
        });
    let app = create_router(AppState {
        list_purchases_use_case: Arc::new(mock_uc),
        ..AppState::for_testing()
    });

    let response = authed(app, Method::GET, "/purchases").await;

    assert_eq!(
        json_of(response).await["items"],
        json!([single_card_purchase_json()])
    );
}

fn db_error() -> AppError {
    AppError::Infra(InfraError::RepositoryError("DB error".to_string()))
}

#[tokio::test]
async fn list_purchases_returns_the_page() {
    let mut mock_uc = MockListPurchasesUseCase::new();
    mock_uc
        .expect_list_purchases()
        .withf(|_, page| page.page == 1 && page.page_size == 5)
        .times(1)
        .returning(|_, page| {
            Box::pin(async move {
                Ok(Paginated {
                    items: vec![purchase(Some("Deck Bleu"))],
                    total: 6,
                    pagination: Pagination::try_new(page.page, page.page_size, 1_000).unwrap(),
                })
            })
        });
    let app = create_router(AppState {
        list_purchases_use_case: Arc::new(mock_uc),
        ..AppState::for_testing()
    });

    let response = authed(app, Method::GET, "/purchases?page=1&page_size=5").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        json_of(response).await,
        json!({
            "items": [purchase_json(json!("Deck Bleu"))],
            "total": 6,
            "page": 1,
            "page_size": 5,
        })
    );
}

#[tokio::test]
async fn list_purchases_propagates_a_pagination_refusal() {
    let mut mock_uc = MockListPurchasesUseCase::new();
    mock_uc.expect_list_purchases().times(1).returning(|_, _| {
        Box::pin(async {
            Err(FunctionalError::InvalidPageSize {
                requested: 0,
                max: 100,
            }
            .into())
        })
    });
    let app = create_router(AppState {
        list_purchases_use_case: Arc::new(mock_uc),
        ..AppState::for_testing()
    });

    let response = authed(app, Method::GET, "/purchases?page_size=0").await;

    assert_error(response, StatusCode::BAD_REQUEST, "invalid_page_size").await;
}

#[tokio::test]
async fn get_purchase_returns_the_purchase_with_its_content_and_profitability() {
    let mut mock_uc = MockGetPurchaseUseCase::new();
    mock_uc
        .expect_get_purchase()
        .withf(|_, id| *id == PurchaseId(Uuid::parse_str(ID).unwrap()))
        .times(1)
        .returning(|_, _| {
            Box::pin(async {
                Ok(PurchaseDetail {
                    purchase: purchase(None),
                    content: vec![
                        PurchaseContentLine {
                            card_id: CardId::new("FDN", "1", LanguageCode::FR),
                            name: "Card".to_string(),
                            image: Some(CardImage {
                                source: CardImageSource::GathererLocalized,
                                has_back: false,
                            }),
                            foil: true,
                            quantity: 2,
                            trend_cents: Some(1000),
                        },
                        PurchaseContentLine {
                            card_id: CardId::new("FDN", "2", LanguageCode::EN),
                            name: "No price".to_string(),
                            image: None,
                            foil: false,
                            quantity: 1,
                            trend_cents: None,
                        },
                    ],
                })
            })
        });
    let app = create_router(AppState {
        get_purchase_use_case: Arc::new(mock_uc),
        ..AppState::for_testing()
    });

    let response = authed(app, Method::GET, &format!("/purchases/{ID}")).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        json_of(response).await,
        json!({
            "purchase": purchase_json(json!(null)),
            "content": [
                {
                    "set_code": "FDN",
                    "collector_number": "1",
                    "language_code": "FR",
                    "name": "Card",
                    "image_url": "/card-images/FDN_1_FR.webp?v=gatherer",
                    "foil": true,
                    "quantity": 2,
                    "trend_cents": 1000,
                },
                {
                    "set_code": "FDN",
                    "collector_number": "2",
                    "language_code": "EN",
                    "name": "No price",
                    "image_url": null,
                    "foil": false,
                    "quantity": 1,
                    "trend_cents": null,
                },
            ],
            "content_value_cents": 2000,
            "lines_without_trend": 1,
            "profitability_cents": -2999,
            "profitability_percent": -2999.0 * 100.0 / 4999.0,
        })
    );
}

#[tokio::test]
async fn get_purchase_is_not_found_for_an_unknown_purchase() {
    let mut mock_uc = MockGetPurchaseUseCase::new();
    mock_uc
        .expect_get_purchase()
        .times(1)
        .returning(|_, _| Box::pin(async { Err(FunctionalError::PurchaseNotFound.into()) }));
    let app = create_router(AppState {
        get_purchase_use_case: Arc::new(mock_uc),
        ..AppState::for_testing()
    });

    let response = authed(app, Method::GET, &format!("/purchases/{ID}")).await;

    assert_error(response, StatusCode::NOT_FOUND, "purchase_not_found").await;
}

#[tokio::test]
async fn an_id_that_is_not_a_uuid_designates_no_purchase() {
    for method in [Method::GET, Method::DELETE] {
        let app = create_router(AppState::for_testing());

        let response = authed(app, method, "/purchases/not-a-uuid").await;

        assert_error(response, StatusCode::NOT_FOUND, "purchase_not_found").await;
    }
}

#[tokio::test]
async fn create_purchase_returns_created_with_the_purchase() {
    let mut mock_uc = MockCreateSealedProductUseCase::new();
    mock_uc
        .expect_create_sealed_product()
        .withf(|_, request| {
            request.product_type == SealedProductType::PreconstructedDeck
                && request.set_code == SetCode::new("FDN")
                && request.name.as_deref() == Some("Deck Bleu")
                && request.amount_cents == 4999
                && request.purchased_on == day()
        })
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(purchase(Some("Deck Bleu"))) }));
    let app = create_router(AppState {
        create_sealed_product_use_case: Arc::new(mock_uc),
        ..AppState::for_testing()
    });

    let response = authed_json(
        app,
        Method::POST,
        "/purchases",
        json!({
            "product_type": "PRECONSTRUCTED_DECK",
            "set_code": "fdn",
            "name": "Deck Bleu",
            "amount_cents": 4999,
            "purchased_on": "2026-11-13",
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(json_of(response).await, purchase_json(json!("Deck Bleu")));
}

#[tokio::test]
async fn create_purchase_propagates_refusals() {
    for (error, status, code) in [
        (
            FunctionalError::InvalidPurchasePrice,
            StatusCode::BAD_REQUEST,
            "invalid_purchase_price",
        ),
        (
            FunctionalError::SetNotFound,
            StatusCode::NOT_FOUND,
            "set_not_found",
        ),
    ] {
        let mut mock_uc = MockCreateSealedProductUseCase::new();
        mock_uc
            .expect_create_sealed_product()
            .times(1)
            .returning(move |_, _| {
                let error = error.clone();
                Box::pin(async move { Err(error.into()) })
            });
        let app = create_router(AppState {
            create_sealed_product_use_case: Arc::new(mock_uc),
            ..AppState::for_testing()
        });

        let response = authed_json(
            app,
            Method::POST,
            "/purchases",
            json!({
                "product_type": "BOOSTER",
                "set_code": "FDN",
                "amount_cents": 0,
                "purchased_on": "2026-11-13",
            }),
        )
        .await;

        assert_error(response, status, code).await;
    }
}

#[tokio::test]
async fn create_purchase_refuses_an_unreadable_body_and_a_bad_set_code() {
    for body in [
        json!({ "product_type": "BOX", "set_code": "FDN", "amount_cents": 1, "purchased_on": "2026-11-13" }),
        json!({ "product_type": "BOOSTER", "set_code": "FDN", "amount_cents": 1, "purchased_on": "13/11/2026" }),
        json!({ "product_type": "BOOSTER", "amount_cents": 1, "purchased_on": "2026-11-13" }),
    ] {
        let app = create_router(AppState::for_testing());

        let response = authed_json(app, Method::POST, "/purchases", body).await;

        // Unknown type, malformed date, missing set: the JSON extractor's own refusal.
        assert_error(
            response,
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_request",
        )
        .await;
    }

    let app = create_router(AppState::for_testing());
    let response = authed_json(
        app,
        Method::POST,
        "/purchases",
        json!({ "product_type": "BOOSTER", "set_code": "F", "amount_cents": 1, "purchased_on": "2026-11-13" }),
    )
    .await;
    assert_error(response, StatusCode::BAD_REQUEST, "invalid_set_code").await;
}

#[tokio::test]
async fn update_purchase_passes_only_the_given_fields() {
    let mut mock_uc = MockUpdatePurchaseUseCase::new();
    mock_uc
        .expect_update_purchase()
        .withf(|_, id, edit| {
            *id == PurchaseId(Uuid::parse_str(ID).unwrap())
                && edit.amount_cents == Some(650)
                && edit.name.as_deref() == Some("")
                && edit.purchased_on.is_none()
                && edit.product_type.is_none()
                && edit.set_code.is_none()
        })
        .times(1)
        .returning(|_, _, _| Box::pin(async { Ok(purchase(None)) }));
    let app = create_router(AppState {
        update_purchase_use_case: Arc::new(mock_uc),
        ..AppState::for_testing()
    });

    let response = authed_json(
        app,
        Method::PATCH,
        &format!("/purchases/{ID}"),
        json!({ "amount_cents": 650, "name": "" }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(json_of(response).await, purchase_json(json!(null)));
}

#[tokio::test]
async fn update_purchase_propagates_refusals() {
    for (error, status, code) in [
        (
            FunctionalError::PurchaseNotFound,
            StatusCode::NOT_FOUND,
            "purchase_not_found",
        ),
        (
            FunctionalError::InvalidPurchasePrice,
            StatusCode::BAD_REQUEST,
            "invalid_purchase_price",
        ),
        (
            FunctionalError::SetNotFound,
            StatusCode::NOT_FOUND,
            "set_not_found",
        ),
        (
            FunctionalError::InvalidRequest("a single card purchase has no type".to_string()),
            StatusCode::BAD_REQUEST,
            "invalid_request",
        ),
    ] {
        let mut mock_uc = MockUpdatePurchaseUseCase::new();
        mock_uc
            .expect_update_purchase()
            .times(1)
            .returning(move |_, _, _| {
                let error = error.clone();
                Box::pin(async move { Err(error.into()) })
            });
        let app = create_router(AppState {
            update_purchase_use_case: Arc::new(mock_uc),
            ..AppState::for_testing()
        });

        let response = authed_json(
            app,
            Method::PATCH,
            &format!("/purchases/{ID}"),
            json!({ "amount_cents": 1 }),
        )
        .await;

        assert_error(response, status, code).await;
    }
}

#[tokio::test]
async fn delete_purchase_returns_no_content() {
    let mut mock_uc = MockDeletePurchaseUseCase::new();
    mock_uc
        .expect_delete_purchase()
        .withf(|_, id| *id == PurchaseId(Uuid::parse_str(ID).unwrap()))
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(()) }));
    let app = create_router(AppState {
        delete_purchase_use_case: Arc::new(mock_uc),
        ..AppState::for_testing()
    });

    let response = authed(app, Method::DELETE, &format!("/purchases/{ID}")).await;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn delete_purchase_propagates_purchase_not_found_and_failures() {
    for (error, status, code) in [
        (
            AppError::from(FunctionalError::PurchaseNotFound),
            StatusCode::NOT_FOUND,
            "purchase_not_found",
        ),
        (db_error(), StatusCode::INTERNAL_SERVER_ERROR, "internal"),
    ] {
        let mut mock_uc = MockDeletePurchaseUseCase::new();
        mock_uc
            .expect_delete_purchase()
            .times(1)
            .returning(move |_, _| {
                let error = error.clone();
                Box::pin(async move { Err(error) })
            });
        let app = create_router(AppState {
            delete_purchase_use_case: Arc::new(mock_uc),
            ..AppState::for_testing()
        });

        let response = authed(app, Method::DELETE, &format!("/purchases/{ID}")).await;

        assert_error(response, status, code).await;
    }
}

#[tokio::test]
async fn purchase_routes_refuse_anonymous_callers() {
    let item = format!("/purchases/{ID}");
    let routes = [
        (Method::GET, "/purchases"),
        (Method::POST, "/purchases"),
        (Method::GET, item.as_str()),
        (Method::PATCH, item.as_str()),
        (Method::DELETE, item.as_str()),
    ];

    assert_routes_refuse_anonymous(|| create_router(AppState::for_testing()), &routes).await;
}
