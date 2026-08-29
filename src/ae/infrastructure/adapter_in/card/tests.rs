use super::controller::*;
use super::dto::*;
use crate::application::error::AppError;
use crate::domain::card::CollectionEntry;
use crate::domain::card_offer::CardOfferSortField;
use crate::domain::error::FunctionalError;
use crate::domain::user::User;
use crate::infrastructure::AppState;
use crate::infrastructure::adapter_in::auth_extractor::AuthenticatedUser;
use axum::extract::{Query, State};
use chrono::NaiveDate;
use serde_json::json;
use std::sync::Arc;

fn make_app_state_with_card_price_history(
    mock: crate::application::use_case::MockGetCardPriceHistoryUseCase,
) -> AppState {
    AppState {
        get_card_price_history_use_case: Arc::new(mock),
        ..AppState::for_testing(Arc::new(
            crate::application::use_case::MockStatsUseCase::new(),
        ))
    }
}

fn make_app_state_with_card_offers(
    mock: crate::application::use_case::MockGetCardOffersUseCase,
) -> AppState {
    AppState::for_testing_with_card_offers(
        Arc::new(crate::application::use_case::MockStatsUseCase::new()),
        Arc::new(mock),
    )
}

fn valid_offers_params() -> CardOffersParams {
    CardOffersParams {
        set_code: "FDN".to_string(),
        collector_number: "87".to_string(),
        language_code: "FR".to_string(),
        foil: false,
        sort_by: CardOffersSortByParam::default(),
        page: 0,
        page_size: 20,
    }
}

// --- Tests for get_card_price_history ---

#[tokio::test]
async fn get_card_price_history_returns_entries() {
    use crate::application::use_case::MockGetCardPriceHistoryUseCase;
    use crate::domain::price::{Price, PriceGuide, PriceHistoryEntry};
    use uuid::Uuid;

    let scryfall_id = Uuid::new_v4();

    let mut mock = MockGetCardPriceHistoryUseCase::new();
    mock.expect_get_card_price_history().returning(|_, _, _| {
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

    let app_state = make_app_state_with_card_price_history(mock);
    let params = PriceHistoryParams {
        start_date: Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
        end_date: Some(NaiveDate::from_ymd_opt(2025, 1, 31).unwrap()),
    };

    let result = get_card_price_history(
        AuthenticatedUser(User::for_testing()),
        State(app_state),
        axum::extract::Path(scryfall_id),
        Query(params),
    )
    .await;

    assert!(result.is_ok());
    let axum::Json(entries) = result.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].low, 100);
    assert_eq!(entries[0].trend, 150);
    assert_eq!(entries[0].avg, 130);
}

#[tokio::test]
async fn get_card_price_history_returns_404_when_card_not_found() {
    use crate::application::use_case::MockGetCardPriceHistoryUseCase;
    use uuid::Uuid;

    let mut mock = MockGetCardPriceHistoryUseCase::new();
    mock.expect_get_card_price_history().returning(|_, _, _| {
        Box::pin(async { Err(AppError::Functional(FunctionalError::CardNotFound)) })
    });

    let app_state = make_app_state_with_card_price_history(mock);

    let result = get_card_price_history(
        AuthenticatedUser(User::for_testing()),
        State(app_state),
        axum::extract::Path(Uuid::new_v4()),
        Query(PriceHistoryParams {
            start_date: None,
            end_date: None,
        }),
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::Functional(FunctionalError::CardNotFound) => {}
        other => panic!("Expected CardNotFound, got {:?}", other),
    }
}

#[tokio::test]
async fn get_card_price_history_propagates_wrong_format_error() {
    use crate::application::use_case::MockGetCardPriceHistoryUseCase;
    use uuid::Uuid;

    let mut mock = MockGetCardPriceHistoryUseCase::new();
    mock.expect_get_card_price_history().returning(|_, _, _| {
        Box::pin(async {
            Err(AppError::Functional(FunctionalError::WrongFormat(
                "start_date must be before or equal to end_date".to_string(),
            )))
        })
    });

    let app_state = make_app_state_with_card_price_history(mock);
    let params = PriceHistoryParams {
        start_date: Some(NaiveDate::from_ymd_opt(2025, 2, 1).unwrap()),
        end_date: Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
    };

    let result = get_card_price_history(
        AuthenticatedUser(User::for_testing()),
        State(app_state),
        axum::extract::Path(Uuid::new_v4()),
        Query(params),
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::Functional(FunctionalError::WrongFormat(msg)) => {
            assert_eq!(msg, "start_date must be before or equal to end_date");
        }
        _ => panic!("Expected WrongFormat error"),
    }
}

#[tokio::test]
async fn get_card_price_history_returns_empty_list() {
    use crate::application::use_case::MockGetCardPriceHistoryUseCase;
    use uuid::Uuid;

    let mut mock = MockGetCardPriceHistoryUseCase::new();
    mock.expect_get_card_price_history()
        .returning(|_, _, _| Box::pin(async { Ok(vec![]) }));

    let app_state = make_app_state_with_card_price_history(mock);

    let result = get_card_price_history(
        AuthenticatedUser(User::for_testing()),
        State(app_state),
        axum::extract::Path(Uuid::new_v4()),
        Query(PriceHistoryParams {
            start_date: None,
            end_date: None,
        }),
    )
    .await;

    assert!(result.is_ok());
    let axum::Json(entries) = result.unwrap();
    assert!(entries.is_empty());
}

// --- Tests for get_card_offers ---

#[tokio::test]
async fn get_card_offers_returns_paginated_offers() {
    use crate::application::use_case::MockGetCardOffersUseCase;
    use crate::domain::card::CollectionEntry;
    use crate::domain::pagination::Paginated;

    let mut mock = MockGetCardOffersUseCase::new();
    mock.expect_get_card_offers()
        .returning(|_, _, _, pagination| {
            Box::pin(async move {
                Ok(Paginated {
                    items: vec![CollectionEntry::Owned {
                        owner_username: "Bob".to_string(),
                        quantity: 3,
                        selling_price: Some(1500),
                        reserved: false,
                    }],
                    total: 1,
                    pagination,
                })
            })
        });

    let app_state = make_app_state_with_card_offers(mock);

    let result = get_card_offers(
        AuthenticatedUser(User::for_testing()),
        State(app_state),
        Query(valid_offers_params()),
    )
    .await;

    assert!(result.is_ok());
    let axum::Json(offers) = result.unwrap();
    assert_eq!(offers.total, 1);
    assert_eq!(offers.items.len(), 1);
    assert_eq!(offers.items[0].owner_username, "Bob");
    assert_eq!(offers.items[0].quantity, 3);
    assert_eq!(offers.items[0].selling_price, Some(1500));
    assert!(!offers.items[0].reserved);
}

#[tokio::test]
async fn get_card_offers_returns_404_when_card_not_found() {
    use crate::application::use_case::MockGetCardOffersUseCase;

    let mut mock = MockGetCardOffersUseCase::new();
    mock.expect_get_card_offers().returning(|_, _, _, _| {
        Box::pin(async { Err(AppError::Functional(FunctionalError::CardNotFound)) })
    });

    let app_state = make_app_state_with_card_offers(mock);

    let result = get_card_offers(
        AuthenticatedUser(User::for_testing()),
        State(app_state),
        Query(valid_offers_params()),
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::Functional(FunctionalError::CardNotFound) => {}
        other => panic!("Expected CardNotFound, got {:?}", other),
    }
}

#[tokio::test]
async fn get_card_offers_returns_400_for_invalid_language_code() {
    use crate::application::use_case::MockGetCardOffersUseCase;

    let mock = MockGetCardOffersUseCase::new();
    let app_state = make_app_state_with_card_offers(mock);

    let mut params = valid_offers_params();
    params.language_code = "XX".to_string();

    let result = get_card_offers(
        AuthenticatedUser(User::for_testing()),
        State(app_state),
        Query(params),
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::Functional(FunctionalError::InvalidLanguageCode(code)) => {
            assert_eq!(code, "XX");
        }
        other => panic!("Expected InvalidLanguageCode, got {:?}", other),
    }
}

#[tokio::test]
async fn get_card_offers_returns_400_for_collector_number_too_long() {
    use crate::application::use_case::MockGetCardOffersUseCase;

    let mock = MockGetCardOffersUseCase::new();
    let app_state = make_app_state_with_card_offers(mock);

    let mut params = valid_offers_params();
    params.collector_number = "12345678901".to_string();

    let result = get_card_offers(
        AuthenticatedUser(User::for_testing()),
        State(app_state),
        Query(params),
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::Functional(FunctionalError::InvalidCollectorNumber(_)) => {}
        other => panic!("Expected InvalidCollectorNumber, got {:?}", other),
    }
}

#[tokio::test]
async fn get_card_offers_returns_empty_items_with_nonzero_total_for_page_beyond_last_result() {
    use crate::application::use_case::MockGetCardOffersUseCase;
    use crate::domain::pagination::{Paginated, Pagination};

    let mut mock = MockGetCardOffersUseCase::new();
    mock.expect_get_card_offers().returning(|_, _, _, _| {
        Box::pin(async move {
            Ok(Paginated {
                items: vec![],
                total: 3,
                pagination: Pagination::try_new(2, 20, 60).unwrap(),
            })
        })
    });

    let app_state = make_app_state_with_card_offers(mock);

    let mut params = valid_offers_params();
    params.page = 2;

    let result = get_card_offers(
        AuthenticatedUser(User::for_testing()),
        State(app_state),
        Query(params),
    )
    .await;

    assert!(result.is_ok());
    let axum::Json(offers) = result.unwrap();
    assert!(offers.items.is_empty());
    assert_eq!(offers.total, 3);
}

#[tokio::test]
async fn get_card_offers_rejects_page_size_above_max() {
    use crate::application::use_case::MockGetCardOffersUseCase;

    // The use case must never be reached: the pagination is rejected before that.
    let mock = MockGetCardOffersUseCase::new();
    let app_state = make_app_state_with_card_offers(mock);

    let mut params = valid_offers_params();
    params.page_size = 1000;

    let result = get_card_offers(
        AuthenticatedUser(User::for_testing()),
        State(app_state),
        Query(params),
    )
    .await;

    match result.unwrap_err() {
        AppError::Functional(FunctionalError::InvalidPageSize {
            requested: 1000,
            max: 100,
        }) => {}
        other => panic!("Expected InvalidPageSize, got {:?}", other),
    }
}

#[tokio::test]
async fn get_card_offers_rejects_page_size_zero() {
    use crate::application::use_case::MockGetCardOffersUseCase;

    let mock = MockGetCardOffersUseCase::new();
    let app_state = make_app_state_with_card_offers(mock);

    let mut params = valid_offers_params();
    params.page_size = 0;

    let result = get_card_offers(
        AuthenticatedUser(User::for_testing()),
        State(app_state),
        Query(params),
    )
    .await;

    match result.unwrap_err() {
        AppError::Functional(FunctionalError::InvalidPageSize {
            requested: 0,
            max: 100,
        }) => {}
        other => panic!("Expected InvalidPageSize, got {:?}", other),
    }
}

#[tokio::test]
async fn get_card_offers_rejects_offset_beyond_max() {
    use crate::application::service::card_offer_service::CARD_OFFERS_MAX_OFFSET;
    use crate::application::use_case::MockGetCardOffersUseCase;

    let mock = MockGetCardOffersUseCase::new();
    let app_state = make_app_state_with_card_offers(mock);

    let mut params = valid_offers_params();
    params.page = 1000;
    params.page_size = 20;

    let result = get_card_offers(
        AuthenticatedUser(User::for_testing()),
        State(app_state),
        Query(params),
    )
    .await;

    match result.unwrap_err() {
        AppError::Functional(FunctionalError::PaginationTooDeep {
            requested_offset: 20000,
            max,
        }) => {
            assert_eq!(max, CARD_OFFERS_MAX_OFFSET);
        }
        other => panic!("Expected PaginationTooDeep, got {:?}", other),
    }
}

#[tokio::test]
async fn get_card_offers_accepts_low_page_size_with_high_page_under_the_offset_limit() {
    use crate::application::use_case::MockGetCardOffersUseCase;
    use crate::domain::pagination::Paginated;

    // page 29 * page_size 2 = offset 58, within CARD_OFFERS_MAX_OFFSET (60) — the page number
    // alone must never cause a rejection.
    let mut mock = MockGetCardOffersUseCase::new();
    mock.expect_get_card_offers()
        .returning(|_, _, _, pagination| {
            Box::pin(async move {
                Ok(Paginated {
                    items: vec![],
                    total: 0,
                    pagination,
                })
            })
        });

    let app_state = make_app_state_with_card_offers(mock);

    let mut params = valid_offers_params();
    params.page = 29;
    params.page_size = 2;

    let result = get_card_offers(
        AuthenticatedUser(User::for_testing()),
        State(app_state),
        Query(params),
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn card_offers_params_rejects_negative_page_with_bad_request() {
    use axum::extract::FromRequestParts;
    use axum::response::IntoResponse;

    let request = axum::http::Request::builder()
        .uri("/card/offers?set_code=FDN&collector_number=87&language_code=FR&foil=false&page=-1&page_size=20")
        .body(())
        .unwrap();
    let (mut parts, ()) = request.into_parts();

    let response = match Query::<CardOffersParams>::from_request_parts(&mut parts, &()).await {
        Err(rejection) => rejection.into_response(),
        Ok(_) => panic!("expected a rejection for page=-1"),
    };
    assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn card_offers_params_rejects_non_numeric_page_size_with_bad_request() {
    use axum::extract::FromRequestParts;
    use axum::response::IntoResponse;

    let request = axum::http::Request::builder()
        .uri("/card/offers?set_code=FDN&collector_number=87&language_code=FR&foil=false&page=0&page_size=abc")
        .body(())
        .unwrap();
    let (mut parts, ()) = request.into_parts();

    let response = match Query::<CardOffersParams>::from_request_parts(&mut parts, &()).await {
        Err(rejection) => rejection.into_response(),
        Ok(_) => panic!("expected a rejection for page_size=abc"),
    };
    assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
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
        purchase_price: 100,
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
