use super::controller::*;
use super::dto::*;
use crate::application::error::{AppError, InfraError};
use crate::application::use_case::MockSearchCardsUseCase;
use crate::domain::card::{Card, CollectionEntry};
use crate::domain::collection::{CollectionSortField, PaginatedCollection, SortDirection};
use crate::domain::language_code::LanguageCode;
use crate::domain::rarity_code::RarityCode;
use crate::domain::user::User;
use crate::infrastructure::AppState;
use crate::infrastructure::adapter_in::auth_extractor::AuthenticatedUser;
use axum::extract::State;
use axum_extra::extract::Query;
use std::sync::Arc;

fn make_app_state_with_search(mock: MockSearchCardsUseCase) -> AppState {
    AppState {
        search_cards_use_case: Arc::new(mock),
        ..AppState::for_testing(Arc::new(
            crate::application::use_case::MockStatsUseCase::new(),
        ))
    }
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

fn make_paginated(items: Vec<Card>, page: u32, page_size: u32) -> PaginatedCollection {
    let total = items.len() as u64;
    PaginatedCollection {
        items,
        total,
        page,
        page_size,
    }
}

#[tokio::test]
async fn search_cards_returns_empty_response_when_search_is_empty() {
    let mut mock = MockSearchCardsUseCase::new();
    mock.expect_search_cards()
        .returning(|_| Box::pin(async { Ok(make_paginated(vec![], 0, 20)) }));

    let app_state = make_app_state_with_search(mock);

    let result = search_cards(
        AuthenticatedUser(User::for_testing()),
        State(app_state),
        Query(SearchParams::default()),
    )
    .await;

    assert!(result.is_ok());
    let axum::Json(response) = result.unwrap();
    assert!(response.items.is_empty());
    assert_eq!(response.total, 0);
    assert_eq!(response.page, 0);
    assert_eq!(response.page_size, 20);
}

#[tokio::test]
async fn search_cards_returns_cards_from_use_case() {
    let mut mock = MockSearchCardsUseCase::new();
    mock.expect_search_cards().returning(|_| {
        Box::pin(async {
            Ok(make_paginated(
                vec![make_card("FDN", "1"), make_card("GPT", "32")],
                0,
                20,
            ))
        })
    });

    let app_state = make_app_state_with_search(mock);

    let result = search_cards(
        AuthenticatedUser(User::for_testing()),
        State(app_state),
        Query(SearchParams::default()),
    )
    .await;

    assert!(result.is_ok());
    let axum::Json(response) = result.unwrap();
    assert_eq!(response.items.len(), 2);
    assert_eq!(response.total, 2);
}

#[tokio::test]
async fn search_cards_propagates_error_from_use_case() {
    let mut mock = MockSearchCardsUseCase::new();
    mock.expect_search_cards().returning(|_| {
        Box::pin(async {
            Err(AppError::Infra(InfraError::RepositoryError(
                "db failure".to_string(),
            )))
        })
    });

    let app_state = make_app_state_with_search(mock);

    let result = search_cards(
        AuthenticatedUser(User::for_testing()),
        State(app_state),
        Query(SearchParams::default()),
    )
    .await;

    assert!(result.is_err());
    match result.err().unwrap() {
        AppError::Infra(InfraError::RepositoryError(msg)) => assert_eq!(msg, "db failure"),
        _ => panic!("Expected RepositoryError"),
    }
}

#[tokio::test]
async fn search_cards_caps_page_size_at_100() {
    let mut mock = MockSearchCardsUseCase::new();
    mock.expect_search_cards()
        .withf(|q| q.collection_query.page_size == 100)
        .returning(|q| {
            let page_size = q.collection_query.page_size;
            Box::pin(async move { Ok(make_paginated(vec![], 0, page_size)) })
        });

    let app_state = make_app_state_with_search(mock);
    let params = SearchParams {
        page_size: 9999,
        ..Default::default()
    };

    let result = search_cards(
        AuthenticatedUser(User::for_testing()),
        State(app_state),
        Query(params),
    )
    .await;

    assert!(result.is_ok());
    let axum::Json(response) = result.unwrap();
    assert_eq!(response.page_size, 100);
}

#[tokio::test]
async fn search_cards_passes_pagination_params_to_use_case() {
    let mut mock = MockSearchCardsUseCase::new();
    mock.expect_search_cards()
        .withf(|q| q.collection_query.page == 3 && q.collection_query.page_size == 5)
        .returning(|q| {
            let (page, page_size) = (q.collection_query.page, q.collection_query.page_size);
            Box::pin(async move { Ok(make_paginated(vec![], page, page_size)) })
        });

    let app_state = make_app_state_with_search(mock);
    let params = SearchParams {
        page: 3,
        page_size: 5,
        ..Default::default()
    };

    let result = search_cards(
        AuthenticatedUser(User::for_testing()),
        State(app_state),
        Query(params),
    )
    .await;

    assert!(result.is_ok());
    let axum::Json(response) = result.unwrap();
    assert_eq!(response.page, 3);
    assert_eq!(response.page_size, 5);
}

#[tokio::test]
async fn search_cards_maps_card_fields_correctly() {
    let mut mock = MockSearchCardsUseCase::new();
    mock.expect_search_cards()
        .returning(|_| Box::pin(async { Ok(make_paginated(vec![make_card("FDN", "42")], 0, 20)) }));

    let app_state = make_app_state_with_search(mock);

    let result = search_cards(
        AuthenticatedUser(User::for_testing()),
        State(app_state),
        Query(SearchParams::default()),
    )
    .await;

    assert!(result.is_ok());
    let axum::Json(response) = result.unwrap();
    let item = &response.items[0];
    assert_eq!(item.set_code, "FDN");
    assert_eq!(item.collector_number, "42");
    assert_eq!(item.language_code, "EN");
    assert!(!item.foil);
    assert_eq!(item.name, "Test Card");
}

#[tokio::test]
async fn search_cards_never_exposes_collection_entry_always_owner_count() {
    let mut card = make_card("FDN", "42");
    card.collection_entry = CollectionEntry::Public {
        owner_count: 3,
        reserved: false,
    };

    let mut mock = MockSearchCardsUseCase::new();
    mock.expect_search_cards().returning(move |_| {
        Box::pin({
            let c = card.clone();
            async move { Ok(make_paginated(vec![c], 0, 20)) }
        })
    });

    let app_state = make_app_state_with_search(mock);

    let result = search_cards(
        AuthenticatedUser(User::for_testing()),
        State(app_state),
        Query(SearchParams::default()),
    )
    .await;

    assert!(result.is_ok());
    let axum::Json(response) = result.unwrap();
    let item = &response.items[0];
    assert!(item.collection_entry.is_none());
    assert_eq!(item.owner_count, Some(3));
}

#[tokio::test]
async fn search_cards_passes_sort_by_avg_to_use_case() {
    let mut mock = MockSearchCardsUseCase::new();
    mock.expect_search_cards()
        .withf(|q| q.collection_query.sort_by == CollectionSortField::Avg)
        .returning(|_| Box::pin(async { Ok(make_paginated(vec![], 0, 20)) }));

    let app_state = make_app_state_with_search(mock);
    let params = SearchParams {
        sort_by: SortByParam::Avg,
        ..Default::default()
    };

    let result = search_cards(
        AuthenticatedUser(User::for_testing()),
        State(app_state),
        Query(params),
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn search_cards_passes_sort_by_set_code_to_use_case() {
    let mut mock = MockSearchCardsUseCase::new();
    mock.expect_search_cards()
        .withf(|q| q.collection_query.sort_by == CollectionSortField::SetCode)
        .returning(|_| Box::pin(async { Ok(make_paginated(vec![], 0, 20)) }));

    let app_state = make_app_state_with_search(mock);
    let params = SearchParams {
        sort_by: SortByParam::SetCode,
        ..Default::default()
    };

    let result = search_cards(
        AuthenticatedUser(User::for_testing()),
        State(app_state),
        Query(params),
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn search_cards_passes_sort_by_language_code_to_use_case() {
    let mut mock = MockSearchCardsUseCase::new();
    mock.expect_search_cards()
        .withf(|q| q.collection_query.sort_by == CollectionSortField::LanguageCode)
        .returning(|_| Box::pin(async { Ok(make_paginated(vec![], 0, 20)) }));

    let app_state = make_app_state_with_search(mock);
    let params = SearchParams {
        sort_by: SortByParam::LanguageCode,
        ..Default::default()
    };

    let result = search_cards(
        AuthenticatedUser(User::for_testing()),
        State(app_state),
        Query(params),
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn search_cards_passes_sort_dir_asc_to_use_case() {
    let mut mock = MockSearchCardsUseCase::new();
    mock.expect_search_cards()
        .withf(|q| q.collection_query.sort_dir == SortDirection::Asc)
        .returning(|_| Box::pin(async { Ok(make_paginated(vec![], 0, 20)) }));

    let app_state = make_app_state_with_search(mock);
    let params = SearchParams {
        sort_dir: SortDirParam::Asc,
        ..Default::default()
    };

    let result = search_cards(
        AuthenticatedUser(User::for_testing()),
        State(app_state),
        Query(params),
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn search_cards_passes_sort_dir_desc_to_use_case() {
    let mut mock = MockSearchCardsUseCase::new();
    mock.expect_search_cards()
        .withf(|q| q.collection_query.sort_dir == SortDirection::Desc)
        .returning(|_| Box::pin(async { Ok(make_paginated(vec![], 0, 20)) }));

    let app_state = make_app_state_with_search(mock);

    let result = search_cards(
        AuthenticatedUser(User::for_testing()),
        State(app_state),
        Query(SearchParams::default()),
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn search_cards_passes_search_query_to_use_case() {
    let mut mock = MockSearchCardsUseCase::new();
    mock.expect_search_cards()
        .withf(|q| q.collection_query.search_query == Some("gob".to_string()))
        .returning(|_| Box::pin(async { Ok(make_paginated(vec![], 0, 20)) }));

    let app_state = make_app_state_with_search(mock);
    let params = SearchParams {
        q: Some("gob".to_string()),
        ..Default::default()
    };

    let result = search_cards(
        AuthenticatedUser(User::for_testing()),
        State(app_state),
        Query(params),
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn search_cards_passes_none_search_query_when_q_is_absent() {
    let mut mock = MockSearchCardsUseCase::new();
    mock.expect_search_cards()
        .withf(|q| q.collection_query.search_query.is_none())
        .returning(|_| Box::pin(async { Ok(make_paginated(vec![], 0, 20)) }));

    let app_state = make_app_state_with_search(mock);

    let result = search_cards(
        AuthenticatedUser(User::for_testing()),
        State(app_state),
        Query(SearchParams::default()),
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn search_cards_parses_repeated_rarity_query_params_into_rarity_codes() {
    let mut mock = MockSearchCardsUseCase::new();
    mock.expect_search_cards()
        .withf(|q| q.collection_query.rarity == vec![RarityCode::C, RarityCode::U])
        .returning(|_| Box::pin(async { Ok(make_paginated(vec![], 0, 20)) }));

    let app_state = make_app_state_with_search(mock);
    let uri: axum::http::Uri = "/search?rarity=C&rarity=U".parse().unwrap();
    let params = Query::<SearchParams>::try_from_uri(&uri).unwrap();

    let result = search_cards(
        AuthenticatedUser(User::for_testing()),
        State(app_state),
        params,
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn search_cards_defaults_rarity_to_empty_when_absent() {
    let mut mock = MockSearchCardsUseCase::new();
    mock.expect_search_cards()
        .withf(|q| q.collection_query.rarity.is_empty())
        .returning(|_| Box::pin(async { Ok(make_paginated(vec![], 0, 20)) }));

    let app_state = make_app_state_with_search(mock);
    let uri: axum::http::Uri = "/search".parse().unwrap();
    let params = Query::<SearchParams>::try_from_uri(&uri).unwrap();

    let result = search_cards(
        AuthenticatedUser(User::for_testing()),
        State(app_state),
        params,
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn search_cards_passes_player_username_to_use_case() {
    let mut mock = MockSearchCardsUseCase::new();
    mock.expect_search_cards()
        .withf(|q| q.player_username == Some("Alice".to_string()))
        .returning(|_| Box::pin(async { Ok(make_paginated(vec![], 0, 20)) }));

    let app_state = make_app_state_with_search(mock);
    let params = SearchParams {
        player_username: Some("Alice".to_string()),
        ..Default::default()
    };

    let result = search_cards(
        AuthenticatedUser(User::for_testing()),
        State(app_state),
        Query(params),
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn search_cards_treats_empty_player_username_as_none() {
    let mut mock = MockSearchCardsUseCase::new();
    mock.expect_search_cards()
        .withf(|q| q.player_username.is_none())
        .returning(|_| Box::pin(async { Ok(make_paginated(vec![], 0, 20)) }));

    let app_state = make_app_state_with_search(mock);
    let params = SearchParams {
        player_username: Some("".to_string()),
        ..Default::default()
    };

    let result = search_cards(
        AuthenticatedUser(User::for_testing()),
        State(app_state),
        Query(params),
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn search_cards_treats_whitespace_only_player_username_as_none() {
    let mut mock = MockSearchCardsUseCase::new();
    mock.expect_search_cards()
        .withf(|q| q.player_username.is_none())
        .returning(|_| Box::pin(async { Ok(make_paginated(vec![], 0, 20)) }));

    let app_state = make_app_state_with_search(mock);
    let params = SearchParams {
        player_username: Some("   ".to_string()),
        ..Default::default()
    };

    let result = search_cards(
        AuthenticatedUser(User::for_testing()),
        State(app_state),
        Query(params),
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn search_cards_defaults_player_username_to_none_when_absent() {
    let mut mock = MockSearchCardsUseCase::new();
    mock.expect_search_cards()
        .withf(|q| q.player_username.is_none())
        .returning(|_| Box::pin(async { Ok(make_paginated(vec![], 0, 20)) }));

    let app_state = make_app_state_with_search(mock);

    let result = search_cards(
        AuthenticatedUser(User::for_testing()),
        State(app_state),
        Query(SearchParams::default()),
    )
    .await;

    assert!(result.is_ok());
}
