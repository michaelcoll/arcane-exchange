use crate::application::error::{AppError, InfraError};
use crate::application::use_case::{MockSearchCardsUseCase, MockSearchSetsUseCase};
use crate::domain::card::{Card, CollectionEntry};
use crate::domain::collection::{CollectionSortField, SearchQuery, SearchScope, SortDirection};
use crate::domain::error::FunctionalError;
use crate::domain::language_code::LanguageCode;
use crate::domain::pagination::{PageRequest, Paginated, Pagination};
use crate::domain::rarity_code::RarityCode;
use crate::domain::set_name::SetName;
use crate::infrastructure::testing::{
    assert_error, assert_routes_refuse_anonymous, authed, json_of,
};
use crate::infrastructure::{AppState, create_router};
use axum::http::{Method, StatusCode};
use serde_json::json;
use std::sync::Arc;

fn search_app(search_cards_use_case: MockSearchCardsUseCase) -> axum::Router {
    create_router(AppState {
        search_cards_use_case: Arc::new(search_cards_use_case),
        ..AppState::for_testing()
    })
}

fn sets_app(search_sets_use_case: MockSearchSetsUseCase) -> axum::Router {
    create_router(AppState {
        search_sets_use_case: Arc::new(search_sets_use_case),
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

/// A search mock that accepts exactly one call matching `matcher` and answers an empty page.
fn expecting_search(
    matcher: impl Fn(&SearchQuery<PageRequest>) -> bool + Send + 'static,
) -> MockSearchCardsUseCase {
    let mut mock = MockSearchCardsUseCase::new();
    mock.expect_search_cards()
        .withf(move |q| matcher(q))
        .times(1)
        .returning(|_| Box::pin(async { Ok(make_paginated(vec![], 0, 20)) }));
    mock
}

/// A sets mock that accepts exactly one call matching `matcher` and answers no set.
fn expecting_sets(
    matcher: impl Fn(&SearchScope) -> bool + Send + 'static,
) -> MockSearchSetsUseCase {
    let mut mock = MockSearchSetsUseCase::new();
    mock.expect_search_sets()
        .withf(move |scope| matcher(scope))
        .times(1)
        .returning(|_| Box::pin(async { Ok(vec![]) }));
    mock
}

#[tokio::test]
async fn search_cards_returns_empty_response_when_search_is_empty() {
    let mut mock = MockSearchCardsUseCase::new();
    mock.expect_search_cards()
        .times(1)
        .returning(|_| Box::pin(async { Ok(make_paginated(vec![], 0, 20)) }));

    let response = authed(search_app(mock), Method::GET, "/search/card").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        json_of(response).await,
        json!({ "items": [], "total": 0, "page": 0, "page_size": 20 })
    );
}

#[tokio::test]
async fn search_cards_returns_cards_from_use_case() {
    let mut mock = MockSearchCardsUseCase::new();
    mock.expect_search_cards().times(1).returning(|_| {
        Box::pin(async {
            Ok(make_paginated(
                vec![make_card("FDN", "1"), make_card("GPT", "32")],
                0,
                20,
            ))
        })
    });

    let response = authed(search_app(mock), Method::GET, "/search/card").await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_of(response).await;
    assert_eq!(body["items"].as_array().unwrap().len(), 2);
    assert_eq!(body["total"], 2);
}

#[tokio::test]
async fn search_cards_returns_empty_items_with_nonzero_total_for_page_beyond_last_result() {
    let mut mock = MockSearchCardsUseCase::new();
    mock.expect_search_cards().times(1).returning(|_| {
        Box::pin(async {
            Ok(Paginated {
                items: vec![],
                total: 7,
                pagination: Pagination::try_new(5, 20, u32::MAX).unwrap(),
            })
        })
    });

    let response = authed(search_app(mock), Method::GET, "/search/card?page=5").await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_of(response).await;
    assert_eq!(body["items"], json!([]));
    assert_eq!(body["total"], 7);
}

#[tokio::test]
async fn search_cards_propagates_error_from_use_case() {
    let mut mock = MockSearchCardsUseCase::new();
    mock.expect_search_cards().times(1).returning(|_| {
        Box::pin(async {
            Err(AppError::Infra(InfraError::RepositoryError(
                "db failure".to_string(),
            )))
        })
    });

    let response = authed(search_app(mock), Method::GET, "/search/card").await;

    assert_error(response, StatusCode::INTERNAL_SERVER_ERROR, "internal").await;
}

#[tokio::test]
async fn search_cards_passes_the_requested_page_to_the_use_case_unvalidated() {
    // Validating the page against the search's depth limit is the use case's job: the
    // controller forwards whatever the client asked for, even a page the use case will reject.
    let mut mock = MockSearchCardsUseCase::new();
    mock.expect_search_cards()
        .withf(|q| {
            q.collection_query.pagination
                == PageRequest {
                    page: 1000,
                    page_size: 9999,
                }
        })
        .times(1)
        .returning(|_| {
            Box::pin(async {
                Err(AppError::Functional(FunctionalError::InvalidPageSize {
                    requested: 9999,
                    max: 100,
                }))
            })
        });

    let response = authed(
        search_app(mock),
        Method::GET,
        "/search/card?page=1000&page_size=9999",
    )
    .await;

    assert_error(response, StatusCode::BAD_REQUEST, "invalid_page_size").await;
}

#[tokio::test]
async fn search_cards_passes_pagination_params_to_use_case() {
    let mut mock = MockSearchCardsUseCase::new();
    mock.expect_search_cards()
        .withf(|q| {
            q.collection_query.pagination
                == PageRequest {
                    page: 3,
                    page_size: 5,
                }
        })
        .times(1)
        .returning(|q| {
            let PageRequest { page, page_size } = q.collection_query.pagination;
            Box::pin(async move { Ok(make_paginated(vec![], page, page_size)) })
        });

    let response = authed(
        search_app(mock),
        Method::GET,
        "/search/card?page=3&page_size=5",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_of(response).await;
    assert_eq!(body["page"], 3);
    assert_eq!(body["page_size"], 5);
}

#[tokio::test]
async fn search_cards_maps_card_fields_correctly() {
    let mut mock = MockSearchCardsUseCase::new();
    mock.expect_search_cards()
        .times(1)
        .returning(|_| Box::pin(async { Ok(make_paginated(vec![make_card("FDN", "42")], 0, 20)) }));

    let response = authed(search_app(mock), Method::GET, "/search/card").await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_of(response).await;
    let item = &body["items"][0];
    assert_eq!(item["set_code"], "FDN");
    assert_eq!(item["collector_number"], "42");
    assert_eq!(item["language_code"], "EN");
    assert_eq!(item["foil"], false);
    assert_eq!(item["name"], "Test Card");
}

#[tokio::test]
async fn search_cards_never_exposes_collection_entry_always_owner_count() {
    let mut card = make_card("FDN", "42");
    card.collection_entry = CollectionEntry::Public {
        owner_count: 3,
        reserved: false,
    };

    let mut mock = MockSearchCardsUseCase::new();
    mock.expect_search_cards().times(1).returning(move |_| {
        let c = card.clone();
        Box::pin(async move { Ok(make_paginated(vec![c], 0, 20)) })
    });

    let response = authed(search_app(mock), Method::GET, "/search/card").await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_of(response).await;
    let item = &body["items"][0];
    assert_eq!(item["collection_entry"], json!(null));
    assert_eq!(item["owner_count"], 3);
}

#[tokio::test]
async fn search_cards_passes_sort_by_avg_to_use_case() {
    let mock = expecting_search(|q| q.collection_query.sort_by == CollectionSortField::Avg);

    let response = authed(search_app(mock), Method::GET, "/search/card?sort_by=avg").await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_cards_passes_sort_by_set_code_to_use_case() {
    let mock = expecting_search(|q| q.collection_query.sort_by == CollectionSortField::SetCode);

    let response = authed(
        search_app(mock),
        Method::GET,
        "/search/card?sort_by=set_code",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_cards_passes_sort_by_language_code_to_use_case() {
    let mock =
        expecting_search(|q| q.collection_query.sort_by == CollectionSortField::LanguageCode);

    let response = authed(
        search_app(mock),
        Method::GET,
        "/search/card?sort_by=language_code",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_cards_passes_sort_dir_asc_to_use_case() {
    let mock = expecting_search(|q| q.collection_query.sort_dir == SortDirection::Asc);

    let response = authed(search_app(mock), Method::GET, "/search/card?sort_dir=asc").await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_cards_defaults_sort_dir_to_desc() {
    let mock = expecting_search(|q| q.collection_query.sort_dir == SortDirection::Desc);

    let response = authed(search_app(mock), Method::GET, "/search/card").await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_cards_passes_search_query_to_use_case() {
    let mock = expecting_search(|q| q.collection_query.search_query == Some("gob".to_string()));

    let response = authed(search_app(mock), Method::GET, "/search/card?q=gob").await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_cards_passes_none_search_query_when_q_is_absent() {
    let mock = expecting_search(|q| q.collection_query.search_query.is_none());

    let response = authed(search_app(mock), Method::GET, "/search/card").await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_cards_parses_repeated_rarity_query_params_into_rarity_codes() {
    let mock =
        expecting_search(|q| q.collection_query.rarity == vec![RarityCode::C, RarityCode::U]);

    let response = authed(
        search_app(mock),
        Method::GET,
        "/search/card?rarity=C&rarity=U",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_cards_defaults_rarity_to_empty_when_absent() {
    let mock = expecting_search(|q| q.collection_query.rarity.is_empty());

    let response = authed(search_app(mock), Method::GET, "/search/card").await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_cards_passes_player_username_to_use_case() {
    let mock = expecting_search(|q| q.player_username == Some("Alice".to_string()));

    let response = authed(
        search_app(mock),
        Method::GET,
        "/search/card?player_username=Alice",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_cards_treats_empty_player_username_as_none() {
    let mock = expecting_search(|q| q.player_username.is_none());

    let response = authed(
        search_app(mock),
        Method::GET,
        "/search/card?player_username=",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_cards_treats_whitespace_only_player_username_as_none() {
    let mock = expecting_search(|q| q.player_username.is_none());

    let response = authed(
        search_app(mock),
        Method::GET,
        "/search/card?player_username=%20%20%20",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_cards_defaults_player_username_to_none_when_absent() {
    let mock = expecting_search(|q| q.player_username.is_none());

    let response = authed(search_app(mock), Method::GET, "/search/card").await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_cards_rejects_added_at_sort_without_player_username() {
    // The use case must never be reached: the combination is rejected before that.
    let response = authed(
        search_app(MockSearchCardsUseCase::new()),
        Method::GET,
        "/search/card?sort_by=added_at",
    )
    .await;

    assert_error(
        response,
        StatusCode::BAD_REQUEST,
        "added_at_sort_requires_player_username",
    )
    .await;
}

#[tokio::test]
async fn search_cards_rejects_added_at_sort_with_empty_player_username() {
    let response = authed(
        search_app(MockSearchCardsUseCase::new()),
        Method::GET,
        "/search/card?sort_by=added_at&player_username=",
    )
    .await;

    assert_error(
        response,
        StatusCode::BAD_REQUEST,
        "added_at_sort_requires_player_username",
    )
    .await;
}

#[tokio::test]
async fn search_cards_passes_sort_by_added_at_to_use_case() {
    let mock = expecting_search(|q| q.collection_query.sort_by == CollectionSortField::AddedAt);

    let response = authed(
        search_app(mock),
        Method::GET,
        "/search/card?sort_by=added_at&player_username=alice",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_cards_with_added_at_sort_and_unknown_player_username_still_reaches_use_case() {
    // Whether the username matches a real user is a repository concern; the
    // controller-level validation only cares that `player_username` is present.
    let mock = expecting_search(|q| {
        q.collection_query.sort_by == CollectionSortField::AddedAt
            && q.player_username.as_deref() == Some("no-such-user")
    });

    let response = authed(
        search_app(mock),
        Method::GET,
        "/search/card?sort_by=added_at&player_username=no-such-user",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_card_sets_passes_the_scope_with_a_trimmed_player_username() {
    let mock = expecting_sets(|scope| {
        *scope
            == SearchScope {
                search_query: Some("gob".to_string()),
                player_username: Some("Bob".to_string()),
            }
    });

    let response = authed(
        sets_app(mock),
        Method::GET,
        "/search/card/sets?q=gob&player_username=%20%20Bob%20",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_card_sets_treats_a_blank_player_username_as_absent() {
    let mock = expecting_sets(|scope| scope.player_username.is_none());

    let response = authed(
        sets_app(mock),
        Method::GET,
        "/search/card/sets?q=gob&player_username=%20%20%20",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_card_sets_treats_a_blank_q_as_absent() {
    let mock = expecting_sets(|scope| scope.search_query.is_none());

    let response = authed(
        sets_app(mock),
        Method::GET,
        "/search/card/sets?q=%20%20&player_username=Bob",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_card_sets_returns_the_sets_in_the_use_case_order() {
    let mut mock = MockSearchSetsUseCase::new();
    mock.expect_search_sets().times(1).returning(|_| {
        Box::pin(async {
            Ok(vec![
                SetName::new("ZZZ", "Alpha"),
                SetName::new("AAA", "Zendikar"),
            ])
        })
    });

    let response = authed(sets_app(mock), Method::GET, "/search/card/sets").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        json_of(response).await,
        json!([
            { "code": "ZZZ", "name": "Alpha" },
            { "code": "AAA", "name": "Zendikar" },
        ])
    );
}

#[tokio::test]
async fn search_card_sets_propagates_use_case_errors() {
    let mut mock = MockSearchSetsUseCase::new();
    mock.expect_search_sets().times(1).returning(|_| {
        Box::pin(async {
            Err(AppError::Infra(InfraError::RepositoryError(
                "db down".to_string(),
            )))
        })
    });

    let response = authed(sets_app(mock), Method::GET, "/search/card/sets").await;

    assert_error(response, StatusCode::INTERNAL_SERVER_ERROR, "internal").await;
}

#[tokio::test]
async fn every_route_refuses_an_anonymous_request() {
    let routes = [
        (Method::GET, "/search/card"),
        (Method::GET, "/search/card/sets"),
    ];

    assert_routes_refuse_anonymous(|| create_router(AppState::for_testing()), &routes).await;
}
