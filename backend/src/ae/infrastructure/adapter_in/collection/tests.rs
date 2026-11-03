use super::dto::*;
use crate::application::error::{AppError, InfraError};
use crate::application::use_case::{
    MockGetCardImportUseCase, MockGetCollectionPriceHistoryUseCase, MockGetCollectionStatsUseCase,
    MockGetCollectionUseCase, MockImportCardUseCase,
};
use crate::domain::card::{Card, CollectionEntry};
use crate::domain::card_import::{CardImport, CardImportId, CardImportStatus};
use crate::domain::collection::{CollectionSortField, SortDirection};
use crate::domain::collection_stats::{BinderInfo, CollectionStats};
use crate::domain::error::FunctionalError;
use crate::domain::language_code::LanguageCode;
use crate::domain::pagination::{PageRequest, Paginated, Pagination};
use crate::domain::price::{Price, PriceGuide, PriceHistoryEntry};
use crate::domain::rarity_code::RarityCode;
use crate::domain::set_name::{SetCode, SetName};
use crate::domain::user::User;
use crate::infrastructure::testing::{
    BEARER, assert_error, assert_routes_refuse_anonymous, authed, json_of, send,
};
use crate::infrastructure::{AppState, create_router};
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use chrono::NaiveDate;
use serde_json::json;
use std::sync::Arc;

const CSV_HEADER: &str = "Binder Name,Binder Type,Name,Set code,Set name,Collector number,Foil,Rarity,Quantity,ManaBox ID,Scryfall ID,Purchase price,Misprint,Altered,Signed,Condition,Language,Proxy,Purchase price currency,Added";

fn collection_app(mock: MockGetCollectionUseCase) -> axum::Router {
    create_router(AppState {
        get_collection_use_case: Arc::new(mock),
        ..AppState::for_testing()
    })
}

fn stats_app(mock: MockGetCollectionStatsUseCase) -> axum::Router {
    create_router(AppState {
        get_collection_stats_use_case: Arc::new(mock),
        ..AppState::for_testing()
    })
}

fn import_card_app(mock: MockImportCardUseCase) -> axum::Router {
    create_router(AppState {
        import_card_use_case: Arc::new(mock),
        ..AppState::for_testing()
    })
}

fn card_import_query_app(mock: MockGetCardImportUseCase) -> axum::Router {
    create_router(AppState {
        card_import_query_use_case: Arc::new(mock),
        ..AppState::for_testing()
    })
}

fn price_history_app(mock: MockGetCollectionPriceHistoryUseCase) -> axum::Router {
    create_router(AppState {
        get_collection_price_history_use_case: Arc::new(mock),
        ..AppState::for_testing()
    })
}

fn make_card(set_code: &str, collector_number: &str) -> Card {
    Card::new(
        set_code,
        format!("Set {}", set_code),
        collector_number,
        LanguageCode::EN,
        false,
        "Test Card",
        RarityCode::C,
        1,
        100,
    )
}

fn make_paginated(items: Vec<Card>, page: u32, page_size: u32) -> Paginated<Card> {
    let total = items.len() as u64;
    let pagination = Pagination::try_new(page, page_size, u32::MAX).unwrap();
    Paginated {
        items,
        total,
        pagination,
    }
}

/// A mock of the collection use case that answers an empty first page to the queries matching
/// `matches`, and fails the test (no expectation matches) for any other.
fn expecting_collection(
    matches: impl Fn(&crate::domain::collection::CollectionQuery<PageRequest>) -> bool + Send + 'static,
) -> MockGetCollectionUseCase {
    let mut mock = MockGetCollectionUseCase::new();
    mock.expect_get_collection()
        .withf(move |_, q| matches(q))
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(make_paginated(vec![], 0, 20)) }));
    mock
}

// ============================================================
// Authentication
// ============================================================

#[tokio::test]
async fn every_route_refuses_an_anonymous_request() {
    let routes = [
        (Method::GET, "/collection"),
        (Method::POST, "/collection/import"),
        (Method::GET, "/collection/import"),
        (
            Method::GET,
            "/collection/import/6f2f0e5e-6b52-4b3e-9f0c-0d2c3f1d9a11",
        ),
        (Method::GET, "/collection/stats"),
        (Method::GET, "/collection/price-history"),
    ];

    assert_routes_refuse_anonymous(|| create_router(AppState::for_testing()), &routes).await;
}

// ============================================================
// GET /collection
// ============================================================

#[tokio::test]
async fn get_collection_returns_empty_response_when_collection_is_empty() {
    let mut mock = MockGetCollectionUseCase::new();
    mock.expect_get_collection()
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(make_paginated(vec![], 0, 20)) }));

    let response = authed(collection_app(mock), Method::GET, "/collection").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        json_of(response).await,
        json!({ "items": [], "total": 0, "page": 0, "page_size": 20 })
    );
}

#[tokio::test]
async fn get_collection_returns_empty_items_with_nonzero_total_for_page_beyond_last_result() {
    let mut mock = MockGetCollectionUseCase::new();
    mock.expect_get_collection().times(1).returning(|_, _| {
        Box::pin(async {
            Ok(Paginated {
                items: vec![],
                total: 12,
                pagination: Pagination::try_new(5, 20, u32::MAX).unwrap(),
            })
        })
    });

    let response = authed(collection_app(mock), Method::GET, "/collection?page=5").await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_of(response).await;
    assert_eq!(body["items"], json!([]));
    assert_eq!(body["total"], 12);
}

#[tokio::test]
async fn get_collection_returns_cards_from_use_case() {
    let mut mock = MockGetCollectionUseCase::new();
    mock.expect_get_collection()
        .withf(|user_id, _| *user_id == User::for_testing().id)
        .times(1)
        .returning(|_, _| {
            Box::pin(async {
                Ok(make_paginated(
                    vec![make_card("FDN", "1"), make_card("GPT", "32")],
                    0,
                    20,
                ))
            })
        });

    let response = authed(collection_app(mock), Method::GET, "/collection").await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_of(response).await;
    assert_eq!(body["items"].as_array().unwrap().len(), 2);
    assert_eq!(body["total"], 2);
}

#[tokio::test]
async fn get_collection_propagates_error_from_use_case() {
    let mut mock = MockGetCollectionUseCase::new();
    mock.expect_get_collection().times(1).returning(|_, _| {
        Box::pin(async {
            Err(AppError::Infra(InfraError::RepositoryError(
                "db failure".to_string(),
            )))
        })
    });

    let response = authed(collection_app(mock), Method::GET, "/collection").await;

    assert_error(response, StatusCode::INTERNAL_SERVER_ERROR, "internal").await;
}

#[tokio::test]
async fn get_collection_rejects_negative_page_with_bad_request() {
    let response = authed(
        create_router(AppState::for_testing()),
        Method::GET,
        "/collection?page=-1&page_size=20",
    )
    .await;

    assert_error(response, StatusCode::BAD_REQUEST, "invalid_request").await;
}

#[tokio::test]
async fn get_collection_rejects_non_numeric_page_size_with_bad_request() {
    let response = authed(
        create_router(AppState::for_testing()),
        Method::GET,
        "/collection?page=0&page_size=abc",
    )
    .await;

    assert_error(response, StatusCode::BAD_REQUEST, "invalid_request").await;
}

#[tokio::test]
async fn get_collection_passes_the_requested_page_to_the_use_case_unvalidated() {
    // Validating the page against the collection's depth limit is the use case's job: the
    // controller forwards whatever the client asked for, even a page the use case will reject.
    let mut mock = MockGetCollectionUseCase::new();
    mock.expect_get_collection()
        .withf(|_, q| {
            q.pagination
                == PageRequest {
                    page: 1000,
                    page_size: 9999,
                }
        })
        .times(1)
        .returning(|_, _| {
            Box::pin(async {
                Err(AppError::Functional(FunctionalError::InvalidPageSize {
                    requested: 9999,
                    max: 100,
                }))
            })
        });

    let response = authed(
        collection_app(mock),
        Method::GET,
        "/collection?page=1000&page_size=9999",
    )
    .await;

    assert_error(response, StatusCode::BAD_REQUEST, "invalid_page_size").await;
}

#[tokio::test]
async fn get_collection_passes_pagination_params_to_use_case() {
    let mut mock = MockGetCollectionUseCase::new();
    mock.expect_get_collection()
        .withf(|_, q| {
            q.pagination
                == PageRequest {
                    page: 3,
                    page_size: 5,
                }
        })
        .times(1)
        .returning(|_, q| {
            let PageRequest { page, page_size } = q.pagination;
            Box::pin(async move { Ok(make_paginated(vec![], page, page_size)) })
        });

    let response = authed(
        collection_app(mock),
        Method::GET,
        "/collection?page=3&page_size=5",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_of(response).await;
    assert_eq!(body["page"], 3);
    assert_eq!(body["page_size"], 5);
}

#[tokio::test]
async fn get_collection_maps_card_fields_correctly() {
    let mut mock = MockGetCollectionUseCase::new();
    mock.expect_get_collection().times(1).returning(|_, _| {
        Box::pin(async { Ok(make_paginated(vec![make_card("FDN", "42")], 0, 20)) })
    });

    let response = authed(collection_app(mock), Method::GET, "/collection").await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_of(response).await;
    let item = &body["items"][0];
    assert_eq!(item["set_code"], "FDN");
    assert_eq!(item["collector_number"], "42");
    assert_eq!(item["language_code"], "EN");
    assert_eq!(item["foil"], false);
    assert_eq!(item["name"], "Test Card");
    assert_eq!(item["collection_entry"]["quantity"], 1);
    assert_eq!(item["collection_entry"]["purchase_price"], 100);
    assert_eq!(item["owner_count"], json!(null));
    assert_eq!(item["price_guide"], json!(null));
}

#[tokio::test]
async fn get_collection_maps_foil_card_correctly() {
    let card = Card::new(
        "FDN",
        "Set FDN",
        "99",
        LanguageCode::EN,
        true,
        "Foil Card",
        RarityCode::R,
        2,
        500,
    );

    let mut mock = MockGetCollectionUseCase::new();
    mock.expect_get_collection()
        .times(1)
        .returning(move |_, _| {
            let c = card.clone();
            Box::pin(async move { Ok(make_paginated(vec![c], 0, 20)) })
        });

    let response = authed(collection_app(mock), Method::GET, "/collection").await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_of(response).await;
    let item = &body["items"][0];
    assert_eq!(item["foil"], true);
    assert_eq!(item["name"], "Foil Card");
    assert_eq!(item["collection_entry"]["quantity"], 2);
    assert_eq!(item["collection_entry"]["purchase_price"], 500);
}

#[tokio::test]
async fn get_collection_masks_collection_entry_for_search_style_entries() {
    let mut card = make_card("FDN", "42");
    card.collection_entry = CollectionEntry::Public {
        owner_count: 1,
        reserved: false,
    };

    let mut mock = MockGetCollectionUseCase::new();
    mock.expect_get_collection()
        .times(1)
        .returning(move |_, _| {
            let c = card.clone();
            Box::pin(async move { Ok(make_paginated(vec![c], 0, 20)) })
        });

    let response = authed(collection_app(mock), Method::GET, "/collection").await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_of(response).await;
    let item = &body["items"][0];
    assert_eq!(item["collection_entry"], json!(null));
    assert_eq!(item["owner_count"], 1);
}

#[tokio::test]
async fn get_collection_maps_price_guide_fields_when_card_has_prices() {
    let mut card = make_card("FDN", "1");
    card.price_guide = Some(PriceGuide {
        low: Price { value: Some(100) },
        avg: Price { value: Some(200) },
        trend: Price { value: Some(300) },
    });

    let mut mock = MockGetCollectionUseCase::new();
    mock.expect_get_collection()
        .times(1)
        .returning(move |_, _| {
            let c = card.clone();
            Box::pin(async move { Ok(make_paginated(vec![c], 0, 20)) })
        });

    let response = authed(collection_app(mock), Method::GET, "/collection").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        json_of(response).await["items"][0]["price_guide"],
        json!({ "low": 100, "avg": 200, "trend": 300 })
    );
}

#[tokio::test]
async fn get_collection_preserves_total_independent_of_page_items() {
    let mut mock = MockGetCollectionUseCase::new();
    mock.expect_get_collection().times(1).returning(|_, _| {
        Box::pin(async {
            Ok(Paginated {
                items: vec![make_card("FDN", "1")],
                total: 42,
                pagination: Pagination::try_new(2, 1, u32::MAX).unwrap(),
            })
        })
    });

    let response = authed(
        collection_app(mock),
        Method::GET,
        "/collection?page=2&page_size=1",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_of(response).await;
    assert_eq!(body["total"], 42);
    assert_eq!(body["items"].as_array().unwrap().len(), 1);
    assert_eq!(body["page"], 2);
    assert_eq!(body["page_size"], 1);
}

#[tokio::test]
async fn get_collection_passes_sort_by_to_use_case() {
    let variants = [
        ("avg", CollectionSortField::Avg),
        ("trend", CollectionSortField::Trend),
        ("set_code", CollectionSortField::SetCode),
        ("language_code", CollectionSortField::LanguageCode),
    ];

    for (input, expected) in variants {
        let mock = expecting_collection({
            let value = expected.clone();
            move |q| q.sort_by == value
        });

        let response = authed(
            collection_app(mock),
            Method::GET,
            &format!("/collection?sort_by={input}"),
        )
        .await;

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "sort_by={input} should pass {expected:?} to use case"
        );
    }
}

#[tokio::test]
async fn get_collection_passes_sort_dir_to_use_case() {
    let variants = [("asc", SortDirection::Asc), ("desc", SortDirection::Desc)];

    for (input, expected) in variants {
        let mock = expecting_collection({
            let value = expected.clone();
            move |q| q.sort_dir == value
        });

        let response = authed(
            collection_app(mock),
            Method::GET,
            &format!("/collection?sort_dir={input}"),
        )
        .await;

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "sort_dir={input} should pass {expected:?} to use case"
        );
    }
}

#[tokio::test]
async fn get_collection_passes_search_query_to_use_case() {
    let mock = expecting_collection(|q| q.search_query == Some("gob".to_string()));

    let response = authed(collection_app(mock), Method::GET, "/collection?q=gob").await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn get_collection_passes_none_search_query_when_q_is_absent() {
    let mock = expecting_collection(|q| q.search_query.is_none());

    let response = authed(collection_app(mock), Method::GET, "/collection").await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn get_collection_parses_repeated_rarity_query_params_into_rarity_codes() {
    let mock = expecting_collection(|q| q.rarity == vec![RarityCode::C, RarityCode::U]);

    let response = authed(
        collection_app(mock),
        Method::GET,
        "/collection?rarity=C&rarity=U",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn get_collection_defaults_rarity_to_empty_when_absent() {
    let mock = expecting_collection(|q| q.rarity.is_empty());

    let response = authed(collection_app(mock), Method::GET, "/collection").await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn get_collection_passes_all_query_params_to_use_case() {
    let mock = expecting_collection(|q| {
        q.pagination
            == PageRequest {
                page: 5,
                page_size: 50,
            }
            && q.sort_by == CollectionSortField::Avg
            && q.sort_dir == SortDirection::Asc
            && q.search_query == Some("goblin".to_string())
            && q.rarity == vec![RarityCode::C, RarityCode::R]
            && q.sets == vec!["FDN".to_string(), "GPT".to_string()]
            && q.price_min == Some(100)
            && q.price_max == Some(5000)
    });

    let response = authed(
        collection_app(mock),
        Method::GET,
        "/collection?page=5&page_size=50&sort_by=avg&sort_dir=asc&q=goblin&rarity=C&rarity=R&sets=FDN,GPT&price_min=100&price_max=5000",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn get_collection_applies_the_defaults_when_no_query_param_is_given() {
    let mock = expecting_collection(|q| {
        q.pagination
            == PageRequest {
                page: 0,
                page_size: 20,
            }
            && q.sort_by == CollectionSortField::Trend
            && q.sort_dir == SortDirection::Desc
            && q.search_query.is_none()
            && q.rarity.is_empty()
            && q.sets.is_empty()
            && q.price_min.is_none()
            && q.price_max.is_none()
    });

    let response = authed(collection_app(mock), Method::GET, "/collection").await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn get_collection_decodes_a_plus_as_a_space_and_keeps_the_other_defaults() {
    let mock = expecting_collection(|q| {
        q.pagination
            == PageRequest {
                page: 0,
                page_size: 20,
            }
            && q.sort_by == CollectionSortField::Trend
            && q.sort_dir == SortDirection::Desc
            && q.search_query == Some("ancestral reveal".to_string())
            && q.rarity.is_empty()
            && q.sets.is_empty()
            && q.price_min == Some(1000)
            && q.price_max.is_none()
    });

    let response = authed(
        collection_app(mock),
        Method::GET,
        "/collection?q=ancestral+reveal&price_min=1000",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

// ============================================================
// POST /collection/import
// ============================================================

fn import_request(body: impl Into<Body>) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri("/collection/import")
        .header(header::AUTHORIZATION, BEARER)
        .header(header::CONTENT_TYPE, "text/plain")
        .body(body.into())
        .unwrap()
}

/// An import app whose use case expects exactly one `start_import` of `csv` for the caller.
fn import_app_expecting(csv: &str) -> axum::Router {
    let expected = csv.to_string();
    let mut mock = MockImportCardUseCase::new();
    mock.expect_start_import()
        .withf(move |csv, user| csv == expected && *user == User::for_testing())
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(CardImportId::new()) }));
    import_card_app(mock)
}

async fn assert_import_accepted(csv: String) {
    let response = send(import_app_expecting(&csv), import_request(csv)).await;

    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert!(!json_of(response).await["id"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn import_cards_succeeds_with_valid_csv() {
    assert_import_accepted(format!(
        "{CSV_HEADER}\n\
        bulk,binder,Goblin Boarders,FDN,Foundations,87,normal,common,3,101506,4409a063-bf2a-4a49-803e-3ce6bd474353,0.08,false,false,false,near_mint,fr,false,EUR,2026-02-05T20:44:45.815Z"
    ))
    .await;
}

#[tokio::test]
async fn import_cards_succeeds_with_multiple_cards() {
    assert_import_accepted(format!(
        "{CSV_HEADER}\n\
        bulk,binder,Goblin Boarders,FDN,Foundations,87,normal,common,3,101506,4409a063-bf2a-4a49-803e-3ce6bd474353,0.08,false,false,false,near_mint,fr,false,EUR,2026-02-05T20:44:45.815Z\n\
        bulk,binder,Repeal,GPT,Guildpact,32,normal,common,2,27563,9e7dd929-4bba-46a6-86c9-b8ed853eb721,0.17,false,false,false,near_mint,fr,false,EUR,2026-02-05T20:44:45.815Z"
    ))
    .await;
}

#[tokio::test]
async fn import_cards_succeeds_with_foil_cards() {
    assert_import_accepted(format!(
        "{CSV_HEADER}\n\
        bulk,binder,Sol Ring,FDN,Foundations,42,foil,mythic,1,101500,11111111-1111-1111-1111-111111111111,5.00,false,false,false,near_mint,en,false,EUR,2026-02-05T20:44:45.815Z"
    ))
    .await;
}

#[tokio::test]
async fn import_cards_succeeds_with_special_characters_in_card_name() {
    assert_import_accepted(format!(
        "{CSV_HEADER}\n\
        bulk,binder,\"Dwynen, Gilt-Leaf Daen\",FDN,Foundations,217,normal,uncommon,2,100086,01c00d7b-7fac-4f8c-a1ea-de2cf4d06627,0.2,false,false,false,near_mint,fr,false,EUR,2026-02-05T20:44:45.815Z"
    ))
    .await;
}

#[tokio::test]
async fn import_cards_fails_with_invalid_utf8() {
    let mut mock = MockImportCardUseCase::new();
    mock.expect_start_import().never();

    let response = send(
        import_card_app(mock),
        import_request(vec![0xFF, 0xFE, 0xFD]),
    )
    .await;

    assert_error(response, StatusCode::BAD_REQUEST, "malformed_csv").await;
}

#[tokio::test]
async fn import_cards_returns_conflict_when_already_running() {
    let mut mock = MockImportCardUseCase::new();
    mock.expect_start_import().times(1).returning(|_, _| {
        Box::pin(async { Err(AppError::Functional(FunctionalError::ImportAlreadyRunning)) })
    });

    let response = send(
        import_card_app(mock),
        import_request(format!(
            "{CSV_HEADER}\n\
            bulk,binder,Goblin Boarders,FDN,Foundations,87,normal,common,3,101506,4409a063-bf2a-4a49-803e-3ce6bd474353,0.08,false,false,false,near_mint,fr,false,EUR,2026-02-05T20:44:45.815Z"
        )),
    )
    .await;

    assert_error(response, StatusCode::CONFLICT, "import_already_running").await;
}

// ============================================================
// GET /collection/import/{id}
// ============================================================

fn sample_card_import() -> CardImport {
    CardImport {
        id: CardImportId::new(),
        user_id: User::for_testing().id,
        status: CardImportStatus::Completed,
        source_lines: 1,
        total_lines: 1,
        processed_lines: 1,
        line_errors: vec![],
        line_error_count: 0,
        error_message: None,
        error_code: None,
        created_at: chrono::Utc::now(),
        finished_at: Some(chrono::Utc::now()),
    }
}

#[tokio::test]
async fn get_card_import_returns_the_import_when_owned_by_the_caller() {
    let import = sample_card_import();
    let import_id = import.id;

    let mut mock = MockGetCardImportUseCase::new();
    mock.expect_find()
        .withf(move |id, user| *id == import_id && *user == User::for_testing())
        .times(1)
        .returning(move |_, _| {
            let import = import.clone();
            Box::pin(async move { Ok(import) })
        });

    let response = authed(
        card_import_query_app(mock),
        Method::GET,
        &format!("/collection/import/{import_id}"),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_of(response).await;
    assert_eq!(body["id"], import_id.to_string());
    assert_eq!(body["status"], "completed");
}

#[tokio::test]
async fn get_card_import_exposes_the_error_code_of_a_failed_import() {
    let mut import = sample_card_import();
    import.status = CardImportStatus::Failed;
    import.error_message = Some("no valid line in file".to_string());
    import.error_code = Some("no_valid_line".to_string());
    let import_id = import.id;

    let mut mock = MockGetCardImportUseCase::new();
    mock.expect_find().times(1).returning(move |_, _| {
        let import = import.clone();
        Box::pin(async move { Ok(import) })
    });

    let response = authed(
        card_import_query_app(mock),
        Method::GET,
        &format!("/collection/import/{import_id}"),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_of(response).await;
    assert_eq!(body["status"], "failed");
    assert_eq!(body["error_code"], "no_valid_line");
}

#[tokio::test]
async fn get_card_import_returns_not_found_for_unknown_or_foreign_import() {
    let mut mock = MockGetCardImportUseCase::new();
    mock.expect_find().times(1).returning(|_, _| {
        Box::pin(async { Err(AppError::Functional(FunctionalError::ImportNotFound)) })
    });

    let response = authed(
        card_import_query_app(mock),
        Method::GET,
        &format!("/collection/import/{}", uuid::Uuid::new_v4()),
    )
    .await;

    assert_error(response, StatusCode::NOT_FOUND, "import_not_found").await;
}

// ============================================================
// GET /collection/import
// ============================================================

#[tokio::test]
async fn list_card_imports_returns_the_callers_imports() {
    let import = sample_card_import();
    let import_id = import.id;

    let mut mock = MockGetCardImportUseCase::new();
    mock.expect_list()
        .withf(|user| *user == User::for_testing())
        .times(1)
        .returning(move |_| {
            let import = import.clone();
            Box::pin(async move { Ok(vec![import]) })
        });

    let response = authed(
        card_import_query_app(mock),
        Method::GET,
        "/collection/import",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_of(response).await;
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["id"], import_id.to_string());
}

// ============================================================
// GET /collection/stats
// ============================================================

#[tokio::test]
async fn get_collection_stats_returns_stats_from_use_case() {
    let mut mock = MockGetCollectionStatsUseCase::new();
    mock.expect_get_collection_stats()
        .withf(|user_id| *user_id == User::for_testing().id)
        .times(1)
        .returning(|_| {
            Box::pin(async {
                Ok(CollectionStats {
                    total_cards: 42,
                    unique_cards: 10,
                    price_trend_min: Price::from_cents(100),
                    price_trend_max: Price::from_cents(5000),
                    sets: vec![SetName::new(SetCode::new("FDN"), "Foundations")],
                    binders: vec![BinderInfo {
                        name: "Trade Binder".to_string(),
                        card_count: 12,
                    }],
                })
            })
        });

    let response = authed(stats_app(mock), Method::GET, "/collection/stats").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        json_of(response).await,
        json!({
            "total_cards": 42,
            "unique_cards": 10,
            "price_trend_min": 100,
            "price_trend_max": 5000,
            "sets": [{ "code": "FDN", "name": "Foundations" }],
            "binders": [{ "name": "Trade Binder", "card_count": 12 }],
        })
    );
}

#[tokio::test]
async fn get_collection_stats_returns_empty_for_empty_collection() {
    let mut mock = MockGetCollectionStatsUseCase::new();
    mock.expect_get_collection_stats().times(1).returning(|_| {
        Box::pin(async {
            Ok(CollectionStats {
                total_cards: 0,
                unique_cards: 0,
                price_trend_min: Price::empty(),
                price_trend_max: Price::empty(),
                sets: vec![],
                binders: vec![],
            })
        })
    });

    let response = authed(stats_app(mock), Method::GET, "/collection/stats").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        json_of(response).await,
        json!({
            "total_cards": 0,
            "unique_cards": 0,
            "price_trend_min": null,
            "price_trend_max": null,
            "sets": [],
            "binders": [],
        })
    );
}

#[tokio::test]
async fn get_collection_stats_propagates_error_from_use_case() {
    let mut mock = MockGetCollectionStatsUseCase::new();
    mock.expect_get_collection_stats().times(1).returning(|_| {
        Box::pin(async {
            Err(AppError::Infra(InfraError::RepositoryError(
                "db failure".to_string(),
            )))
        })
    });

    let response = authed(stats_app(mock), Method::GET, "/collection/stats").await;

    assert_error(response, StatusCode::INTERNAL_SERVER_ERROR, "internal").await;
}

// ============================================================
// GET /collection/price-history
// ============================================================

#[tokio::test]
async fn get_collection_price_history_returns_entries() {
    let mut mock = MockGetCollectionPriceHistoryUseCase::new();
    mock.expect_get_collection_price_history()
        .withf(|user_id, start, end| {
            *user_id == User::for_testing().id
                && *start == NaiveDate::from_ymd_opt(2025, 1, 1)
                && *end == NaiveDate::from_ymd_opt(2025, 1, 31)
        })
        .times(1)
        .returning(|_, _, _| {
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

    let response = authed(
        price_history_app(mock),
        Method::GET,
        "/collection/price-history?start_date=2025-01-01&end_date=2025-01-31",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        json_of(response).await,
        json!([{ "date": "2025-01-15", "low": 100, "trend": 150, "avg": 130 }])
    );
}

#[tokio::test]
async fn get_collection_price_history_returns_empty_list() {
    let mut mock = MockGetCollectionPriceHistoryUseCase::new();
    mock.expect_get_collection_price_history()
        .times(1)
        .returning(|_, _, _| Box::pin(async { Ok(vec![]) }));

    let response = authed(
        price_history_app(mock),
        Method::GET,
        "/collection/price-history?start_date=2025-01-01&end_date=2025-01-31",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(json_of(response).await, json!([]));
}

#[tokio::test]
async fn get_collection_price_history_propagates_use_case_error() {
    let mut mock = MockGetCollectionPriceHistoryUseCase::new();
    mock.expect_get_collection_price_history()
        .times(1)
        .returning(|_, _, _| {
            Box::pin(async {
                Err(AppError::Functional(FunctionalError::WrongFormat(
                    "start_date must be before or equal to end_date".to_string(),
                )))
            })
        });

    let response = authed(
        price_history_app(mock),
        Method::GET,
        "/collection/price-history?start_date=2025-02-01&end_date=2025-01-01",
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = json_of(response).await;
    assert_eq!(body["code"], "wrong_format");
    assert_eq!(
        body["error"],
        "start_date must be before or equal to end_date"
    );
}

#[tokio::test]
async fn get_collection_price_history_passes_missing_dates_through_to_use_case() {
    let mut mock = MockGetCollectionPriceHistoryUseCase::new();
    mock.expect_get_collection_price_history()
        .withf(|_, s, e| s.is_none() && e.is_none())
        .times(1)
        .returning(|_, _, _| Box::pin(async { Ok(vec![]) }));

    let response = authed(
        price_history_app(mock),
        Method::GET,
        "/collection/price-history",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

// ============================================================
// Unit tests for dto.rs conversions & deserialisation
// ============================================================

#[test]
fn sort_by_param_from_mapping_is_complete() {
    assert_eq!(
        CollectionSortField::from(SortByParam::Avg),
        CollectionSortField::Avg
    );
    assert_eq!(
        CollectionSortField::from(SortByParam::Trend),
        CollectionSortField::Trend
    );
    assert_eq!(
        CollectionSortField::from(SortByParam::SetCode),
        CollectionSortField::SetCode
    );
    assert_eq!(
        CollectionSortField::from(SortByParam::LanguageCode),
        CollectionSortField::LanguageCode
    );
    assert_eq!(
        CollectionSortField::from(SortByParam::AddedAt),
        CollectionSortField::AddedAt
    );
}

#[test]
fn sort_dir_param_from_mapping_is_complete() {
    assert_eq!(SortDirection::from(SortDirParam::Asc), SortDirection::Asc);
    assert_eq!(SortDirection::from(SortDirParam::Desc), SortDirection::Desc);
}

#[test]
fn rarity_code_param_from_mapping_is_complete() {
    assert_eq!(RarityCode::from(RarityCodeParam::C), RarityCode::C);
    assert_eq!(RarityCode::from(RarityCodeParam::U), RarityCode::U);
    assert_eq!(RarityCode::from(RarityCodeParam::R), RarityCode::R);
    assert_eq!(RarityCode::from(RarityCodeParam::M), RarityCode::M);
    assert_eq!(RarityCode::from(RarityCodeParam::S), RarityCode::S);
}

#[test]
fn default_page_size_returns_20() {
    assert_eq!(default_page_size(), 20);
}

#[test]
fn collection_params_default_values() {
    let params = CollectionParams::default();
    assert_eq!(params.page, 0);
    assert_eq!(params.page_size, 20);
    assert_eq!(params.sort_by, SortByParam::Trend);
    assert_eq!(params.sort_dir, SortDirParam::Desc);
    assert!(params.q.is_none());
    assert!(params.rarity.is_empty());
    assert!(params.sets.is_none());
    assert!(params.price_min.is_none());
    assert!(params.price_max.is_none());
}
