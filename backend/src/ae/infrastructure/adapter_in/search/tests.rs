use crate::application::error::{AppError, InfraError};
use crate::application::use_case::{MockSearchCardsUseCase, MockSearchSetsUseCase};
use crate::domain::card::CardId;
use crate::domain::card_image::{CardImage, CardImageSource};
use crate::domain::collection::SortDirection;
use crate::domain::error::FunctionalError;
use crate::domain::language_code::LanguageCode;
use crate::domain::pagination::{PageRequest, Paginated, Pagination};
use crate::domain::price::PriceGuide;
use crate::domain::rarity_code::RarityCode;
use crate::domain::search::{Offer, Possession, SearchCard, SearchImage, SearchQuery, SearchScope};
use crate::domain::set_name::{SetCode, SetName};
use crate::domain::user::User;
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

fn make_card(set_code: &str, collector_number: &str) -> SearchCard {
    let set_code = SetCode::new(set_code);
    SearchCard {
        set_name: SetName::new(set_code.clone(), format!("Set {}", set_code)),
        collector_number: collector_number.to_string(),
        name: "Test Card".to_string(),
        french_name: None,
        rarity_code: RarityCode::C,
        scryfall_id: uuid::Uuid::default(),
        image: None,
        price_guide: None,
        foil_price_guide: None,
        owner_count: 1,
        owned_quantity: 0,
        offered_languages: vec![],
        reserved: None,
    }
}

/// A search mock that answers `card` as the only result.
fn answering(card: SearchCard) -> MockSearchCardsUseCase {
    let mut mock = MockSearchCardsUseCase::new();
    mock.expect_search_cards().times(1).returning(move |_, _| {
        let c = card.clone();
        Box::pin(async move { Ok(make_paginated(vec![c], 0, 20)) })
    });
    mock
}

fn make_paginated(items: Vec<SearchCard>, page: u32, page_size: u32) -> Paginated<SearchCard> {
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
        .withf(move |_, q| matcher(q))
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(make_paginated(vec![], 0, 20)) }));
    mock
}

/// A sets mock that accepts exactly one call matching `matcher` and answers no set.
fn expecting_sets(
    matcher: impl Fn(&SearchScope) -> bool + Send + 'static,
) -> MockSearchSetsUseCase {
    let mut mock = MockSearchSetsUseCase::new();
    mock.expect_search_sets()
        .withf(move |_, scope| matcher(scope))
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(vec![]) }));
    mock
}

#[tokio::test]
async fn search_cards_returns_empty_response_when_search_is_empty() {
    let mut mock = MockSearchCardsUseCase::new();
    mock.expect_search_cards()
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(make_paginated(vec![], 0, 20)) }));

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
    mock.expect_search_cards().times(1).returning(|_, _| {
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
    mock.expect_search_cards().times(1).returning(|_, _| {
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
    mock.expect_search_cards().times(1).returning(|_, _| {
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
    mock.expect_search_cards().times(1).returning(|_, _| {
        Box::pin(async { Ok(make_paginated(vec![make_card("FDN", "42")], 0, 20)) })
    });

    let response = authed(search_app(mock), Method::GET, "/search/card").await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_of(response).await;
    let item = &body["items"][0];
    assert_eq!(item["set_code"], "FDN");
    assert_eq!(item["set_name"], "Set FDN");
    assert_eq!(item["collector_number"], "42");
    assert_eq!(item["name"], "Test Card");
    assert_eq!(item["rarity_code"], "C");
    assert!(item.get("french_name").is_none());
    // An impression has no language and no finish.
    assert!(item.get("language_code").is_none());
    assert!(item.get("foil").is_none());
}

#[tokio::test]
async fn search_cards_exposes_the_owner_count_and_no_private_data() {
    let mut card = make_card("FDN", "42");
    card.owner_count = 3;

    let response = authed(search_app(answering(card)), Method::GET, "/search/card").await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_of(response).await;
    let item = &body["items"][0];
    assert!(item.get("collection_entry").is_none());
    assert!(item.get("purchase_price").is_none());
    assert!(item.get("added_at").is_none());
    assert_eq!(item["owner_count"], 3);
}

#[tokio::test]
async fn search_cards_exposes_the_owned_quantity() {
    let mut card = make_card("FDN", "42");
    card.owned_quantity = 5;

    let response = authed(search_app(answering(card)), Method::GET, "/search/card").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(json_of(response).await["items"][0]["owned_quantity"], 5);
}

#[tokio::test]
async fn search_cards_exposes_the_offered_languages() {
    let mut card = make_card("FDN", "42");
    card.offered_languages = vec![LanguageCode::EN, LanguageCode::FR];

    let response = authed(search_app(answering(card)), Method::GET, "/search/card").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        json_of(response).await["items"][0]["offered_languages"],
        json!(["EN", "FR"])
    );
}

#[tokio::test]
async fn search_cards_searches_as_the_authenticated_user() {
    let mut mock = MockSearchCardsUseCase::new();
    mock.expect_search_cards()
        .withf(|user_id, _| *user_id == User::for_testing().id)
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(make_paginated(vec![], 0, 20)) }));

    let response = authed(search_app(mock), Method::GET, "/search/card").await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_cards_covers_every_impression_by_default() {
    let mock = expecting_search(|q| q.possession == Possession::Any);

    let response = authed(search_app(mock), Method::GET, "/search/card").await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_cards_passes_owned_true_to_use_case() {
    let mock = expecting_search(|q| q.possession == Possession::Owned);

    let response = authed(search_app(mock), Method::GET, "/search/card?owned=true").await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_cards_passes_owned_false_to_use_case() {
    let mock = expecting_search(|q| q.possession == Possession::NotOwned);

    let response = authed(search_app(mock), Method::GET, "/search/card?owned=false").await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_cards_combines_the_possession_and_the_offer_axes() {
    let mock =
        expecting_search(|q| q.possession == Possession::NotOwned && q.offer == Offer::Offered);

    let response = authed(
        search_app(mock),
        Method::GET,
        "/search/card?owned=false&offered=true",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_cards_exposes_both_names_and_both_price_guides() {
    let mut card = make_card("FDN", "42");
    card.french_name = Some("Carte d'essai".to_string());
    card.price_guide = Some(PriceGuide::new(100u32, 300u32, 200u32));
    card.foil_price_guide = Some(PriceGuide::new(400u32, 900u32, 600u32));

    let response = authed(search_app(answering(card)), Method::GET, "/search/card").await;

    let body = json_of(response).await;
    let item = &body["items"][0];
    assert_eq!(item["name"], "Test Card");
    assert_eq!(item["french_name"], "Carte d'essai");
    assert_eq!(
        item["price_guide"],
        json!({ "low": 100, "avg": 200, "trend": 300 })
    );
    assert_eq!(
        item["foil_price_guide"],
        json!({ "low": 400, "avg": 600, "trend": 900 })
    );
}

#[tokio::test]
async fn search_cards_exposes_reserved_only_when_the_search_has_it() {
    let mut reserved = make_card("FDN", "42");
    reserved.reserved = Some(true);

    let with =
        json_of(authed(search_app(answering(reserved)), Method::GET, "/search/card").await).await;
    let without = json_of(
        authed(
            search_app(answering(make_card("FDN", "42"))),
            Method::GET,
            "/search/card",
        )
        .await,
    )
    .await;

    assert_eq!(with["items"][0]["reserved"], true);
    assert!(without["items"][0].get("reserved").is_none());
}

#[tokio::test]
async fn search_cards_serves_a_downloaded_image_from_the_card_images_and_the_scryfall_one_as_is() {
    let mut local = make_card("FDN", "42");
    local.image = Some(SearchImage::Local {
        card_id: CardId {
            set_code: SetCode::new("FDN"),
            collector_number: "42".to_string(),
            language_code: LanguageCode::FR,
        },
        image: CardImage {
            source: CardImageSource::GathererLocalized,
            has_back: true,
        },
    });
    let mut scryfall = make_card("FDN", "43");
    scryfall.image = Some(SearchImage::Scryfall(
        "https://cards.scryfall.io/43.jpg".to_string(),
    ));

    let local =
        json_of(authed(search_app(answering(local)), Method::GET, "/search/card").await).await;
    let scryfall =
        json_of(authed(search_app(answering(scryfall)), Method::GET, "/search/card").await).await;

    assert_eq!(
        local["items"][0]["image_url"],
        "/card-images/FDN_42_FR.webp?v=gatherer"
    );
    assert_eq!(
        local["items"][0]["image_back_url"],
        "/card-images/FDN_42_FR_back.webp?v=gatherer"
    );
    assert_eq!(
        scryfall["items"][0]["image_url"],
        "https://cards.scryfall.io/43.jpg"
    );
    assert!(scryfall["items"][0]["image_back_url"].is_null());
}

#[tokio::test]
async fn search_cards_passes_sort_dir_asc_to_use_case() {
    let mock = expecting_search(|q| q.sort_dir == SortDirection::Asc);

    let response = authed(search_app(mock), Method::GET, "/search/card?sort_dir=asc").await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_cards_defaults_sort_dir_to_desc() {
    let mock = expecting_search(|q| q.sort_dir == SortDirection::Desc);

    let response = authed(search_app(mock), Method::GET, "/search/card").await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_cards_passes_search_query_to_use_case() {
    let mock = expecting_search(|q| q.search_query == Some("gob".to_string()));

    let response = authed(search_app(mock), Method::GET, "/search/card?q=gob").await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_cards_passes_none_search_query_when_q_is_absent() {
    let mock = expecting_search(|q| q.search_query.is_none());

    let response = authed(search_app(mock), Method::GET, "/search/card").await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_cards_parses_repeated_rarity_query_params_into_rarity_codes() {
    let mock = expecting_search(|q| q.rarity == vec![RarityCode::C, RarityCode::U]);

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
    let mock = expecting_search(|q| q.rarity.is_empty());

    let response = authed(search_app(mock), Method::GET, "/search/card").await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_cards_treats_a_blank_q_as_no_text() {
    let mock = expecting_search(|q| q.search_query.is_none());

    let response = authed(search_app(mock), Method::GET, "/search/card?q=%20%20").await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_cards_offers_every_impression_by_default() {
    let mock = expecting_search(|q| q.offer == Offer::Any);

    let response = authed(search_app(mock), Method::GET, "/search/card").await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_cards_passes_offered_to_use_case() {
    let mock = expecting_search(|q| q.offer == Offer::Offered);

    let response = authed(search_app(mock), Method::GET, "/search/card?offered=true").await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_cards_treats_offered_false_as_every_impression() {
    let mock = expecting_search(|q| q.offer == Offer::Any);

    let response = authed(search_app(mock), Method::GET, "/search/card?offered=false").await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_cards_passes_offered_by_to_use_case_trimmed() {
    let mock = expecting_search(|q| q.offer == Offer::OfferedBy("Alice".to_string()));

    let response = authed(
        search_app(mock),
        Method::GET,
        "/search/card?offered_by=%20Alice%20",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_cards_offered_by_wins_over_offered() {
    let mock = expecting_search(|q| q.offer == Offer::OfferedBy("Alice".to_string()));

    let response = authed(
        search_app(mock),
        Method::GET,
        "/search/card?offered=true&offered_by=Alice",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_cards_treats_a_blank_offered_by_as_no_player() {
    let mock = expecting_search(|q| q.offer == Offer::Offered);

    let response = authed(
        search_app(mock),
        Method::GET,
        "/search/card?offered=true&offered_by=%20%20",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_cards_no_longer_reads_player_username() {
    let mock = expecting_search(|q| q.offer == Offer::Any);

    let response = authed(
        search_app(mock),
        Method::GET,
        "/search/card?player_username=Alice",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_cards_passes_the_set_and_price_filters_to_use_case() {
    let mock = expecting_search(|q| {
        q.sets == vec!["FDN".to_string(), "MH3".to_string()]
            && q.price_min == Some(100)
            && q.price_max == Some(900)
    });

    let response = authed(
        search_app(mock),
        Method::GET,
        "/search/card?sets=fdn,%20mh3&price_min=100&price_max=900",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_card_sets_passes_the_scope_with_a_trimmed_player() {
    let mock = expecting_sets(|scope| {
        *scope
            == SearchScope {
                search_query: Some("gob".to_string()),
                possession: Possession::Any,
                offer: Offer::OfferedBy("Bob".to_string()),
            }
    });

    let response = authed(
        sets_app(mock),
        Method::GET,
        "/search/card/sets?q=gob&offered_by=%20%20Bob%20",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_card_sets_passes_both_axes_and_the_authenticated_player() {
    let mut mock = MockSearchSetsUseCase::new();
    mock.expect_search_sets()
        .withf(|user_id, scope| {
            *user_id == User::for_testing().id
                && scope.possession == Possession::NotOwned
                && scope.offer == Offer::Offered
        })
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(vec![]) }));

    let response = authed(
        sets_app(mock),
        Method::GET,
        "/search/card/sets?owned=false&offered=true",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_card_sets_ignores_the_rarity_set_and_price_filters_of_the_list() {
    // The clients send the list's request to the facet: the same scope must come out of it.
    let scope = SearchScope {
        search_query: Some("Sol Ring".to_string()),
        possession: Possession::Owned,
        offer: Offer::OfferedBy("urza".to_string()),
    };
    let mock = expecting_sets({
        let scope = scope.clone();
        move |received| *received == scope
    });

    let response = authed(
        sets_app(mock),
        Method::GET,
        "/search/card/sets?q=Sol%20Ring&owned=true&offered_by=urza&sets=FDN&rarity=R&price_min=100&price_max=900",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_card_sets_passes_offered() {
    let mock = expecting_sets(|scope| scope.offer == Offer::Offered);

    let response = authed(
        sets_app(mock),
        Method::GET,
        "/search/card/sets?offered=true",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_card_sets_treats_a_blank_player_as_absent() {
    let mock = expecting_sets(|scope| scope.offer == Offer::Any);

    let response = authed(
        sets_app(mock),
        Method::GET,
        "/search/card/sets?q=gob&offered_by=%20%20%20",
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
        "/search/card/sets?q=%20%20&offered_by=Bob",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_card_sets_returns_the_sets_in_the_use_case_order() {
    let mut mock = MockSearchSetsUseCase::new();
    mock.expect_search_sets().times(1).returning(|_, _| {
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
    mock.expect_search_sets().times(1).returning(|_, _| {
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
