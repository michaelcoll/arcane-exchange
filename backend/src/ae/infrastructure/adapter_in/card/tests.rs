use super::dto::*;
use crate::application::error::AppError;
use crate::application::use_case::{MockGetCardOffersUseCase, MockGetCardPriceHistoryUseCase};
use crate::domain::card::CollectionEntry;
use crate::domain::card_offer::CardOfferSortField;
use crate::domain::error::FunctionalError;
use crate::domain::pagination::{PageRequest, Paginated, Pagination};
use crate::domain::price::{Price, PriceGuide, PriceHistoryEntry};
use crate::domain::user::User;
use crate::infrastructure::testing::{
    assert_error, assert_routes_refuse_anonymous, authed, json_of,
};
use crate::infrastructure::{AppState, create_router};
use axum::http::{Method, StatusCode};
use chrono::NaiveDate;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

fn price_history_app(mock: MockGetCardPriceHistoryUseCase) -> axum::Router {
    create_router(AppState {
        get_card_price_history_use_case: Arc::new(mock),
        ..AppState::for_testing()
    })
}

fn offers_app(mock: MockGetCardOffersUseCase) -> axum::Router {
    create_router(AppState {
        get_card_offers_use_case: Arc::new(mock),
        ..AppState::for_testing()
    })
}

const OFFERS_URI: &str =
    "/card/offers?set_code=FDN&collector_number=87&language_code=FR&foil=false&page=0&page_size=20";

// --- Price history ---

#[tokio::test]
async fn get_card_price_history_returns_entries() {
    let scryfall_id = Uuid::new_v4();

    let mut mock = MockGetCardPriceHistoryUseCase::new();
    mock.expect_get_card_price_history()
        .times(1)
        .withf(move |id, foil, start, end| {
            *id == scryfall_id
                && !*foil
                && *start == NaiveDate::from_ymd_opt(2025, 1, 1)
                && *end == NaiveDate::from_ymd_opt(2025, 1, 31)
        })
        .returning(|_, _, _, _| {
            Box::pin(async {
                Ok(vec![PriceHistoryEntry {
                    date: NaiveDate::from_ymd_opt(2025, 1, 15).unwrap(),
                    price_guide: PriceGuide {
                        low: Price { value: Some(100) },
                        trend: Price { value: Some(150) },
                        avg: Price { value: Some(130) },
                    },
                }])
            })
        });

    let uri = format!(
        "/card/{scryfall_id}/price-history?foil=false&start_date=2025-01-01&end_date=2025-01-31"
    );
    let response = authed(price_history_app(mock), Method::GET, &uri).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        json_of(response).await,
        json!([{ "date": "2025-01-15", "low": 100, "trend": 150, "avg": 130 }])
    );
}

#[tokio::test]
async fn get_card_price_history_returns_empty_list() {
    let mut mock = MockGetCardPriceHistoryUseCase::new();
    mock.expect_get_card_price_history()
        .times(1)
        .withf(|_, foil, start, end| *foil && start.is_none() && end.is_none())
        .returning(|_, _, _, _| Box::pin(async { Ok(vec![]) }));

    let uri = format!("/card/{}/price-history?foil=true", Uuid::new_v4());
    let response = authed(price_history_app(mock), Method::GET, &uri).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(json_of(response).await, json!([]));
}

#[tokio::test]
async fn get_card_price_history_returns_404_when_card_not_found() {
    let mut mock = MockGetCardPriceHistoryUseCase::new();
    mock.expect_get_card_price_history()
        .times(1)
        .returning(|_, _, _, _| {
            Box::pin(async { Err(AppError::Functional(FunctionalError::CardNotFound)) })
        });

    let uri = format!("/card/{}/price-history?foil=false", Uuid::new_v4());
    let response = authed(price_history_app(mock), Method::GET, &uri).await;

    assert_error(response, StatusCode::NOT_FOUND, "card_not_found").await;
}

#[tokio::test]
async fn get_card_price_history_propagates_wrong_format_error() {
    let mut mock = MockGetCardPriceHistoryUseCase::new();
    mock.expect_get_card_price_history()
        .times(1)
        .returning(|_, _, _, _| {
            Box::pin(async {
                Err(AppError::Functional(FunctionalError::WrongFormat(
                    "start_date must be before or equal to end_date".to_string(),
                )))
            })
        });

    let uri = format!(
        "/card/{}/price-history?foil=false&start_date=2025-02-01&end_date=2025-01-01",
        Uuid::new_v4()
    );
    let response = authed(price_history_app(mock), Method::GET, &uri).await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = json_of(response).await;
    assert_eq!(body["code"], "wrong_format");
    assert_eq!(
        body["error"],
        "start_date must be before or equal to end_date"
    );
}

#[tokio::test]
async fn get_card_price_history_rejects_missing_foil_with_bad_request() {
    // The catalog doesn't know a card's finish, so `foil` is required on this endpoint.
    let uri = format!("/card/{}/price-history", Uuid::new_v4());
    let response = authed(
        price_history_app(MockGetCardPriceHistoryUseCase::new()),
        Method::GET,
        &uri,
    )
    .await;

    assert_error(response, StatusCode::BAD_REQUEST, "invalid_request").await;
}

// --- Offers ---

#[tokio::test]
async fn get_card_offers_returns_paginated_offers() {
    let mut mock = MockGetCardOffersUseCase::new();
    mock.expect_get_card_offers()
        .times(1)
        .withf(|user_id, _, _, page| {
            *user_id == User::for_testing().id
                && *page
                    == PageRequest {
                        page: 0,
                        page_size: 20,
                    }
        })
        .returning(|_, _, _, _| {
            Box::pin(async move {
                Ok(Paginated {
                    items: vec![CollectionEntry::Owned {
                        owner_username: "Bob".to_string(),
                        quantity: 3,
                        selling_price: Some(1500),
                        reserved: false,
                    }],
                    total: 1,
                    pagination: Pagination::default(),
                })
            })
        });

    let response = authed(offers_app(mock), Method::GET, OFFERS_URI).await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_of(response).await;
    assert_eq!(body["total"], 1);
    assert_eq!(
        body["items"],
        json!([{
            "owner_username": "Bob",
            "quantity": 3,
            "selling_price": 1500,
            "reserved": false,
        }])
    );
}

#[tokio::test]
async fn get_card_offers_returns_404_when_card_not_found() {
    let mut mock = MockGetCardOffersUseCase::new();
    mock.expect_get_card_offers()
        .times(1)
        .returning(|_, _, _, _| {
            Box::pin(async { Err(AppError::Functional(FunctionalError::CardNotFound)) })
        });

    let response = authed(offers_app(mock), Method::GET, OFFERS_URI).await;

    assert_error(response, StatusCode::NOT_FOUND, "card_not_found").await;
}

#[tokio::test]
async fn get_card_offers_returns_400_for_invalid_language_code() {
    let uri = "/card/offers?set_code=FDN&collector_number=87&language_code=XX&foil=false";
    let response = authed(
        offers_app(MockGetCardOffersUseCase::new()),
        Method::GET,
        uri,
    )
    .await;

    assert_error(response, StatusCode::BAD_REQUEST, "invalid_language_code").await;
}

#[tokio::test]
async fn get_card_offers_returns_400_for_collector_number_too_long() {
    let uri = "/card/offers?set_code=FDN&collector_number=12345678901&language_code=FR&foil=false";
    let response = authed(
        offers_app(MockGetCardOffersUseCase::new()),
        Method::GET,
        uri,
    )
    .await;

    assert_error(
        response,
        StatusCode::BAD_REQUEST,
        "invalid_collector_number",
    )
    .await;
}

#[tokio::test]
async fn get_card_offers_returns_empty_items_with_nonzero_total_for_page_beyond_last_result() {
    let mut mock = MockGetCardOffersUseCase::new();
    mock.expect_get_card_offers()
        .times(1)
        .withf(|_, _, _, page| {
            *page
                == PageRequest {
                    page: 2,
                    page_size: 20,
                }
        })
        .returning(|_, _, _, _| {
            Box::pin(async move {
                Ok(Paginated {
                    items: vec![],
                    total: 3,
                    pagination: Pagination::try_new(2, 20, 60).unwrap(),
                })
            })
        });

    let uri = "/card/offers?set_code=FDN&collector_number=87&language_code=FR&foil=false&page=2";
    let response = authed(offers_app(mock), Method::GET, uri).await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_of(response).await;
    assert_eq!(body["items"], json!([]));
    assert_eq!(body["total"], 3);
}

#[tokio::test]
async fn get_card_offers_passes_the_requested_page_to_the_use_case_unvalidated() {
    // Validating the page against the offers' depth limit is the use case's job: the controller
    // forwards whatever the client asked for, even a page the use case will reject.
    let mut mock = MockGetCardOffersUseCase::new();
    mock.expect_get_card_offers()
        .times(1)
        .withf(|_, _, _, page| {
            *page
                == PageRequest {
                    page: 1000,
                    page_size: 1000,
                }
        })
        .returning(|_, _, _, _| {
            Box::pin(async {
                Err(AppError::Functional(FunctionalError::InvalidPageSize {
                    requested: 1000,
                    max: 100,
                }))
            })
        });

    let uri = "/card/offers?set_code=FDN&collector_number=87&language_code=FR&foil=false&page=1000&page_size=1000";
    let response = authed(offers_app(mock), Method::GET, uri).await;

    assert_error(response, StatusCode::BAD_REQUEST, "invalid_page_size").await;
}

#[tokio::test]
async fn get_card_offers_rejects_negative_page_with_bad_request() {
    let uri = "/card/offers?set_code=FDN&collector_number=87&language_code=FR&foil=false&page=-1&page_size=20";
    let response = authed(
        offers_app(MockGetCardOffersUseCase::new()),
        Method::GET,
        uri,
    )
    .await;

    assert_error(response, StatusCode::BAD_REQUEST, "invalid_request").await;
}

#[tokio::test]
async fn get_card_offers_rejects_non_numeric_page_size_with_bad_request() {
    let uri = "/card/offers?set_code=FDN&collector_number=87&language_code=FR&foil=false&page=0&page_size=abc";
    let response = authed(
        offers_app(MockGetCardOffersUseCase::new()),
        Method::GET,
        uri,
    )
    .await;

    assert_error(response, StatusCode::BAD_REQUEST, "invalid_request").await;
}

// --- Authentication ---

#[tokio::test]
async fn every_route_refuses_an_anonymous_request() {
    let price_history_uri = format!("/card/{}/price-history?foil=false", Uuid::new_v4());
    let routes = [
        (Method::POST, "/card/card-info"),
        (Method::GET, price_history_uri.as_str()),
        (Method::GET, OFFERS_URI),
    ];

    assert_routes_refuse_anonymous(|| create_router(AppState::for_testing()), &routes).await;
}

// --- Unit tests for dto.rs ---

#[test]
fn card_offers_params_deserializes_with_all_fields() {
    let json = json!({
        "set_code": "FDN",
        "collector_number": "87",
        "language_code": "FR",
        "foil": true,
        "sort_by": "selling_price",
        "page": 2,
        "page_size": 20
    });

    let params: CardOffersParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.set_code, "FDN");
    assert_eq!(params.collector_number, "87");
    assert_eq!(params.language_code, "FR");
    assert!(params.foil);
    // sort_by deserialized from "selling_price" via serde(rename)
    let sorted: CardOfferSortField = params.sort_by.into();
    assert!(matches!(sorted, CardOfferSortField::SellingPrice));
    assert_eq!(params.page, 2);
    assert_eq!(params.page_size, 20);
}

#[test]
fn card_offers_params_deserializes_with_default_page_size() {
    let json = json!({
        "set_code": "FDN",
        "collector_number": "87",
        "language_code": "FR",
        "foil": false
    });

    let params: CardOffersParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.page_size, 20);
}

#[test]
fn card_offers_params_deserializes_sort_by_snake_case() {
    let json = json!({
        "set_code": "FDN",
        "collector_number": "87",
        "language_code": "EN",
        "foil": false,
        "sort_by": "selling_price"
    });

    let params: CardOffersParams = serde_json::from_value(json).unwrap();
    let sorted: CardOfferSortField = params.sort_by.into();
    assert!(matches!(sorted, CardOfferSortField::SellingPrice));
}

#[test]
fn card_offers_params_deserializes_with_default_sort_by() {
    let json = json!({
        "set_code": "FDN",
        "collector_number": "87",
        "language_code": "EN",
        "foil": false
    });

    let params: CardOffersParams = serde_json::from_value(json).unwrap();
    let sorted: CardOfferSortField = params.sort_by.into();
    assert!(matches!(sorted, CardOfferSortField::SellingPrice));
}

#[test]
fn card_offers_params_deserializes_with_default_page() {
    let json = json!({
        "set_code": "FDN",
        "collector_number": "87",
        "language_code": "EN",
        "foil": false
    });

    let params: CardOffersParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.page, 0);
}

#[test]
fn from_collection_entry_owned_converts_correctly() {
    let entry = CollectionEntry::Owned {
        owner_username: "alice".to_string(),
        quantity: 5,
        selling_price: Some(2500),
        reserved: false,
    };

    let response: CardOfferResponse = entry.into();
    assert_eq!(response.owner_username, "alice");
    assert_eq!(response.quantity, 5);
    assert_eq!(response.selling_price, Some(2500));
    assert!(!response.reserved);
}

#[test]
fn from_collection_entry_owned_with_none_selling_price() {
    let entry = CollectionEntry::Owned {
        owner_username: "bob".to_string(),
        quantity: 1,
        selling_price: None,
        reserved: false,
    };

    let response: CardOfferResponse = entry.into();
    assert_eq!(response.owner_username, "bob");
    assert_eq!(response.quantity, 1);
    assert!(response.selling_price.is_none());
}

#[test]
fn from_collection_entry_owned_propagates_reserved() {
    let entry = CollectionEntry::Owned {
        owner_username: "carol".to_string(),
        quantity: 1,
        selling_price: Some(100),
        reserved: true,
    };

    let response: CardOfferResponse = entry.into();
    assert!(response.reserved);
}

#[test]
#[should_panic(expected = "get_offers only ever returns CollectionEntry::Owned entries")]
fn from_collection_entry_mine_panics() {
    let entry = CollectionEntry::Mine {
        quantity: 2,
        purchase_price: Some(100),
        added_at: chrono::Utc::now(),
        reserved: false,
    };
    let _response: CardOfferResponse = entry.into();
}

#[test]
#[should_panic(expected = "get_offers only ever returns CollectionEntry::Owned entries")]
fn from_collection_entry_public_panics() {
    let entry = CollectionEntry::Public {
        owner_count: 42,
        reserved: false,
    };
    let _response: CardOfferResponse = entry.into();
}

#[test]
fn price_history_params_deserializes_with_both_dates() {
    let json = json!({
        "start_date": "2025-01-01",
        "end_date": "2025-01-31"
    });

    let params: PriceHistoryParams = serde_json::from_value(json).unwrap();
    assert_eq!(
        params.start_date,
        Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap())
    );
    assert_eq!(
        params.end_date,
        Some(NaiveDate::from_ymd_opt(2025, 1, 31).unwrap())
    );
}

#[test]
fn price_history_params_deserializes_with_no_dates() {
    let json = json!({});

    let params: PriceHistoryParams = serde_json::from_value(json).unwrap();
    assert!(params.start_date.is_none());
    assert!(params.end_date.is_none());
}

#[test]
fn price_history_params_deserializes_with_start_date_only() {
    let json = json!({
        "start_date": "2025-06-01"
    });

    let params: PriceHistoryParams = serde_json::from_value(json).unwrap();
    assert_eq!(
        params.start_date,
        Some(NaiveDate::from_ymd_opt(2025, 6, 1).unwrap())
    );
    assert!(params.end_date.is_none());
}

#[test]
fn price_history_params_deserializes_with_end_date_only() {
    let json = json!({
        "end_date": "2025-12-31"
    });

    let params: PriceHistoryParams = serde_json::from_value(json).unwrap();
    assert!(params.start_date.is_none());
    assert_eq!(
        params.end_date,
        Some(NaiveDate::from_ymd_opt(2025, 12, 31).unwrap())
    );
}

#[test]
fn paginated_card_offers_response_serializes_correctly() {
    let response = PaginatedCardOffersResponse {
        items: vec![CardOfferResponse {
            owner_username: "alice".to_string(),
            quantity: 3,
            selling_price: Some(1500),
            reserved: false,
        }],
        total: 10,
        page: 0,
        page_size: 6,
    };

    let json_str = serde_json::to_string(&response).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert_eq!(parsed["total"], 10);
    assert_eq!(parsed["page"], 0);
    assert_eq!(parsed["page_size"], 6);
    assert_eq!(parsed["items"][0]["owner_username"], "alice");
    assert_eq!(parsed["items"][0]["selling_price"], 1500);
}

#[test]
fn price_history_entry_response_serializes_correctly() {
    let entry = PriceHistoryEntryResponse {
        date: "2025-01-15".to_string(),
        low: 1000,
        trend: 1500,
        avg: 1300,
    };

    let json_str = serde_json::to_string(&entry).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert_eq!(parsed["date"], "2025-01-15");
    assert_eq!(parsed["low"], 1000);
    assert_eq!(parsed["trend"], 1500);
    assert_eq!(parsed["avg"], 1300);
}
