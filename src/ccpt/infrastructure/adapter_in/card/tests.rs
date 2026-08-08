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
    use crate::domain::card_offer::PaginatedCardOffers;

    let mut mock = MockGetCardOffersUseCase::new();
    mock.expect_get_card_offers()
        .returning(|_, _, _, page, page_size| {
            Box::pin(async move {
                Ok(PaginatedCardOffers {
                    items: vec![CollectionEntry::Owned {
                        owner_username: "Bob".to_string(),
                        quantity: 3,
                        selling_price: Some(1500),
                    }],
                    total: 1,
                    page,
                    page_size,
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
}

#[tokio::test]
async fn get_card_offers_returns_404_when_card_not_found() {
    use crate::application::use_case::MockGetCardOffersUseCase;

    let mut mock = MockGetCardOffersUseCase::new();
    mock.expect_get_card_offers().returning(|_, _, _, _, _| {
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
async fn get_card_offers_caps_page_size_at_max() {
    use crate::application::use_case::MockGetCardOffersUseCase;
    use crate::domain::card_offer::PaginatedCardOffers;

    let mut mock = MockGetCardOffersUseCase::new();
    mock.expect_get_card_offers()
        .withf(|_, _, _, _, page_size| *page_size == 100)
        .returning(|_, _, _, page, page_size| {
            Box::pin(async move {
                Ok(PaginatedCardOffers {
                    items: vec![],
                    total: 0,
                    page,
                    page_size,
                })
            })
        });

    let app_state = make_app_state_with_card_offers(mock);

    let mut params = valid_offers_params();
    params.page_size = 1000;

    let result = get_card_offers(
        AuthenticatedUser(User::for_testing()),
        State(app_state),
        Query(params),
    )
    .await;

    assert!(result.is_ok());
    let axum::Json(offers) = result.unwrap();
    assert_eq!(offers.page_size, 100);
}

#[tokio::test]
async fn get_card_offers_floors_page_size_at_min() {
    use crate::application::use_case::MockGetCardOffersUseCase;
    use crate::domain::card_offer::PaginatedCardOffers;

    let mut mock = MockGetCardOffersUseCase::new();
    mock.expect_get_card_offers()
        .withf(|_, _, _, _, page_size| *page_size == 1)
        .returning(|_, _, _, page, page_size| {
            Box::pin(async move {
                Ok(PaginatedCardOffers {
                    items: vec![],
                    total: 0,
                    page,
                    page_size,
                })
            })
        });

    let app_state = make_app_state_with_card_offers(mock);

    let mut params = valid_offers_params();
    params.page_size = 0;

    let result = get_card_offers(
        AuthenticatedUser(User::for_testing()),
        State(app_state),
        Query(params),
    )
    .await;

    assert!(result.is_ok());
    let axum::Json(offers) = result.unwrap();
    assert_eq!(offers.page_size, 1);
}

#[tokio::test]
async fn get_card_offers_caps_page_at_max() {
    use crate::application::use_case::MockGetCardOffersUseCase;
    use crate::domain::card_offer::PaginatedCardOffers;

    let mut mock = MockGetCardOffersUseCase::new();
    mock.expect_get_card_offers()
        .withf(|_, _, _, page, _| *page == 10)
        .returning(|_, _, _, page, page_size| {
            Box::pin(async move {
                Ok(PaginatedCardOffers {
                    items: vec![],
                    total: 0,
                    page,
                    page_size,
                })
            })
        });

    let app_state = make_app_state_with_card_offers(mock);

    let mut params = valid_offers_params();
    params.page = 1000;

    let result = get_card_offers(
        AuthenticatedUser(User::for_testing()),
        State(app_state),
        Query(params),
    )
    .await;

    assert!(result.is_ok());
    let axum::Json(offers) = result.unwrap();
    assert_eq!(offers.page, 10);
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
    assert_eq!(params.page_size, default_page_size());
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
    };

    let response: CardOfferResponse = entry.into();
    assert_eq!(response.owner_username, "alice");
    assert_eq!(response.quantity, 5);
    assert_eq!(response.selling_price, Some(2500));
}

#[test]
fn from_collection_entry_owned_with_none_selling_price() {
    let entry = CollectionEntry::Owned {
        owner_username: "bob".to_string(),
        quantity: 1,
        selling_price: None,
    };

    let response: CardOfferResponse = entry.into();
    assert_eq!(response.owner_username, "bob");
    assert_eq!(response.quantity, 1);
    assert!(response.selling_price.is_none());
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
    let entry = CollectionEntry::Public { owner_count: 42 };
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
