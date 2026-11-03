use super::dto::ListTradesParams;
use crate::application::error::AppError;
use crate::application::use_case::{
    MockAbandonTradeUseCase, MockAcceptTradeUseCase, MockAddTradeCardUseCase,
    MockConfirmTradeUseCase, MockCreateTradeUseCase, MockGetTradeUseCase, MockListTradesUseCase,
    MockRateTradeUseCase, MockRemoveTradeCardUseCase,
};
use crate::domain::card::CopyId;
use crate::domain::card_image::{CardImage, CardImageSource};
use crate::domain::error::FunctionalError;
use crate::domain::language_code::LanguageCode;
use crate::domain::pagination::{PageRequest, Paginated, Pagination};
use crate::domain::price::PriceGuide;
use crate::domain::trade::{
    TradeCardDetail, TradeDetail, TradeId, TradePartyState, TradeStatus, TradeSummary,
};
use crate::domain::user::{User, UserId};
use crate::infrastructure::testing::{
    assert_error, assert_routes_refuse_anonymous, authed, authed_json, json_of,
};
use crate::infrastructure::{AppState, create_router};
use axum::http::{Method, StatusCode};
use axum::response::Response;
use chrono::{TimeZone, Utc};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

const TRADE_ID: Uuid = Uuid::from_u128(0x1234_5678_9abc_def0_1234_5678_9abc_def0);
const TRADE_ID_STR: &str = "12345678-9abc-def0-1234-56789abcdef0";

fn copy_id() -> CopyId {
    CopyId::new("FDN", "87", LanguageCode::FR, false)
}

fn card_body() -> serde_json::Value {
    json!({
        "set_code": "FDN",
        "collector_number": "87",
        "language_code": "FR",
        "foil": false,
        "owner_username": "bob",
    })
}

fn empty_page() -> Paginated<TradeSummary> {
    Paginated {
        items: vec![],
        total: 0,
        pagination: Pagination::default(),
    }
}

// --- create_trade ---

fn create_trade_app(create_trade_use_case: MockCreateTradeUseCase) -> axum::Router {
    create_router(AppState {
        create_trade_use_case: Arc::new(create_trade_use_case),
        ..AppState::for_testing()
    })
}

async fn post_create_failing_with(error: fn() -> FunctionalError) -> Response {
    let mut mock_use_case = MockCreateTradeUseCase::new();
    mock_use_case
        .expect_create_trade()
        .times(1)
        .returning(move |_, _| Box::pin(async move { Err(AppError::Functional(error())) }));

    authed_json(
        create_trade_app(mock_use_case),
        Method::POST,
        "/trades",
        json!({ "respondent_username": "bob" }),
    )
    .await
}

#[tokio::test]
async fn create_trade_returns_created_on_nominal_payload() {
    let created = TradeId(TRADE_ID);
    let mut mock_use_case = MockCreateTradeUseCase::new();
    mock_use_case
        .expect_create_trade()
        .times(1)
        .withf(|caller, respondent| *caller == User::for_testing().id && respondent == "bob")
        .returning(move |_, _| Box::pin(async move { Ok(created) }));

    let response = authed_json(
        create_trade_app(mock_use_case),
        Method::POST,
        "/trades",
        json!({ "respondent_username": "bob" }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(json_of(response).await, json!({ "id": TRADE_ID_STR }));
}

#[tokio::test]
async fn create_trade_rejects_a_payload_without_respondent() {
    let response = authed_json(
        create_router(AppState::for_testing()),
        Method::POST,
        "/trades",
        json!({}),
    )
    .await;

    assert_error(
        response,
        StatusCode::UNPROCESSABLE_ENTITY,
        "invalid_request",
    )
    .await;
}

#[tokio::test]
async fn create_trade_propagates_user_not_found_from_use_case() {
    let response = post_create_failing_with(|| FunctionalError::UserNotFound).await;

    assert_error(response, StatusCode::NOT_FOUND, "user_not_found").await;
}

#[tokio::test]
async fn create_trade_propagates_self_trade_from_use_case() {
    let response = post_create_failing_with(|| FunctionalError::SelfTrade).await;

    assert_error(response, StatusCode::BAD_REQUEST, "self_trade").await;
}

// --- add_trade_card ---

fn add_trade_card_app(add_trade_card_use_case: MockAddTradeCardUseCase) -> axum::Router {
    create_router(AppState {
        add_trade_card_use_case: Arc::new(add_trade_card_use_case),
        ..AppState::for_testing()
    })
}

async fn post_add_card_failing_with(error: fn() -> FunctionalError) -> Response {
    let mut mock_use_case = MockAddTradeCardUseCase::new();
    mock_use_case
        .expect_add_card()
        .times(1)
        .returning(move |_, _, _, _| Box::pin(async move { Err(AppError::Functional(error())) }));

    authed_json(
        add_trade_card_app(mock_use_case),
        Method::POST,
        &format!("/trades/{TRADE_ID}/cards"),
        card_body(),
    )
    .await
}

#[tokio::test]
async fn add_trade_card_ignores_the_quantity_older_apps_still_send() {
    let mut mock_use_case = MockAddTradeCardUseCase::new();
    mock_use_case
        .expect_add_card()
        .times(1)
        .withf(|_, _, owner_username, _| owner_username == "bob")
        .returning(|_, _, _, _| Box::pin(async { Ok(()) }));

    let mut body = card_body();
    body["quantity"] = json!(3);

    let response = authed_json(
        add_trade_card_app(mock_use_case),
        Method::POST,
        &format!("/trades/{TRADE_ID}/cards"),
        body,
    )
    .await;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn add_trade_card_returns_no_content_on_success() {
    let mut mock_use_case = MockAddTradeCardUseCase::new();
    mock_use_case
        .expect_add_card()
        .times(1)
        .withf(|trade_id, caller, owner_username, card| {
            *trade_id == TradeId(TRADE_ID)
                && *caller == User::for_testing().id
                && owner_username == "bob"
                && *card == copy_id()
        })
        .returning(|_, _, _, _| Box::pin(async { Ok(()) }));

    let response = authed_json(
        add_trade_card_app(mock_use_case),
        Method::POST,
        &format!("/trades/{TRADE_ID}/cards"),
        card_body(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn add_trade_card_returns_bad_request_on_invalid_language_code() {
    let mut body = card_body();
    body["language_code"] = json!("XX");

    let response = authed_json(
        create_router(AppState::for_testing()),
        Method::POST,
        &format!("/trades/{TRADE_ID}/cards"),
        body,
    )
    .await;

    assert_error(response, StatusCode::BAD_REQUEST, "invalid_language_code").await;
}

#[tokio::test]
async fn add_trade_card_rejects_a_malformed_trade_id() {
    let response = authed_json(
        create_router(AppState::for_testing()),
        Method::POST,
        "/trades/not-a-uuid/cards",
        card_body(),
    )
    .await;

    assert_error(response, StatusCode::BAD_REQUEST, "invalid_request").await;
}

#[tokio::test]
async fn add_trade_card_propagates_trade_not_found_from_use_case() {
    let response = post_add_card_failing_with(|| FunctionalError::TradeNotFound).await;

    assert_error(response, StatusCode::NOT_FOUND, "trade_not_found").await;
}

#[tokio::test]
async fn add_trade_card_propagates_card_not_found_from_use_case() {
    // Covers both a nonexistent card and a card the owner doesn't actually offer to trade
    // (visibility/binders/rarity filters) — the use case surfaces both as `CardNotFound`.
    let response = post_add_card_failing_with(|| FunctionalError::CardNotFound).await;

    assert_error(response, StatusCode::NOT_FOUND, "card_not_found").await;
}

#[tokio::test]
async fn add_trade_card_propagates_trade_access_denied_from_use_case() {
    let response = post_add_card_failing_with(|| FunctionalError::TradeAccessDenied).await;

    assert_error(response, StatusCode::FORBIDDEN, "trade_access_denied").await;
}

#[tokio::test]
async fn add_trade_card_propagates_trade_not_modifiable_from_use_case() {
    let response = post_add_card_failing_with(|| FunctionalError::TradeNotModifiable).await;

    assert_error(response, StatusCode::CONFLICT, "trade_not_modifiable").await;
}

// --- remove_trade_card ---

fn remove_trade_card_app(remove_trade_card_use_case: MockRemoveTradeCardUseCase) -> axum::Router {
    create_router(AppState {
        remove_trade_card_use_case: Arc::new(remove_trade_card_use_case),
        ..AppState::for_testing()
    })
}

async fn post_remove_card_failing_with(error: fn() -> FunctionalError) -> Response {
    let mut mock_use_case = MockRemoveTradeCardUseCase::new();
    mock_use_case
        .expect_remove_card()
        .times(1)
        .returning(move |_, _, _, _| Box::pin(async move { Err(AppError::Functional(error())) }));

    authed_json(
        remove_trade_card_app(mock_use_case),
        Method::POST,
        &format!("/trades/{TRADE_ID}/cards/remove"),
        card_body(),
    )
    .await
}

#[tokio::test]
async fn remove_trade_card_returns_no_content_on_success() {
    let mut mock_use_case = MockRemoveTradeCardUseCase::new();
    mock_use_case
        .expect_remove_card()
        .times(1)
        .withf(|trade_id, caller, owner_username, card| {
            *trade_id == TradeId(TRADE_ID)
                && *caller == User::for_testing().id
                && owner_username == "bob"
                && *card == copy_id()
        })
        .returning(|_, _, _, _| Box::pin(async { Ok(()) }));

    let response = authed_json(
        remove_trade_card_app(mock_use_case),
        Method::POST,
        &format!("/trades/{TRADE_ID}/cards/remove"),
        card_body(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn remove_trade_card_returns_bad_request_on_invalid_language_code() {
    let mut body = card_body();
    body["language_code"] = json!("XX");

    let response = authed_json(
        create_router(AppState::for_testing()),
        Method::POST,
        &format!("/trades/{TRADE_ID}/cards/remove"),
        body,
    )
    .await;

    assert_error(response, StatusCode::BAD_REQUEST, "invalid_language_code").await;
}

#[tokio::test]
async fn remove_trade_card_propagates_trade_card_not_found_from_use_case() {
    let response = post_remove_card_failing_with(|| FunctionalError::TradeCardNotFound).await;

    assert_error(response, StatusCode::NOT_FOUND, "trade_card_not_found").await;
}

#[tokio::test]
async fn remove_trade_card_propagates_trade_not_modifiable_from_use_case() {
    let response = post_remove_card_failing_with(|| FunctionalError::TradeNotModifiable).await;

    assert_error(response, StatusCode::CONFLICT, "trade_not_modifiable").await;
}

// --- accept_trade ---

fn accept_trade_app(accept_trade_use_case: MockAcceptTradeUseCase) -> axum::Router {
    create_router(AppState {
        accept_trade_use_case: Arc::new(accept_trade_use_case),
        ..AppState::for_testing()
    })
}

async fn post_accept_failing_with(error: fn() -> FunctionalError) -> Response {
    let mut mock_use_case = MockAcceptTradeUseCase::new();
    mock_use_case
        .expect_accept()
        .times(1)
        .returning(move |_, _| Box::pin(async move { Err(AppError::Functional(error())) }));

    authed(
        accept_trade_app(mock_use_case),
        Method::POST,
        &format!("/trades/{TRADE_ID}/accept"),
    )
    .await
}

#[tokio::test]
async fn accept_trade_returns_no_content_on_success() {
    let mut mock_use_case = MockAcceptTradeUseCase::new();
    mock_use_case
        .expect_accept()
        .times(1)
        .withf(|trade_id, caller| {
            *trade_id == TradeId(TRADE_ID) && *caller == User::for_testing().id
        })
        .returning(|_, _| Box::pin(async { Ok(()) }));

    let response = authed(
        accept_trade_app(mock_use_case),
        Method::POST,
        &format!("/trades/{TRADE_ID}/accept"),
    )
    .await;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn accept_trade_propagates_trade_not_found_from_use_case() {
    let response = post_accept_failing_with(|| FunctionalError::TradeNotFound).await;

    assert_error(response, StatusCode::NOT_FOUND, "trade_not_found").await;
}

#[tokio::test]
async fn accept_trade_propagates_trade_access_denied_from_use_case() {
    let response = post_accept_failing_with(|| FunctionalError::TradeAccessDenied).await;

    assert_error(response, StatusCode::FORBIDDEN, "trade_access_denied").await;
}

#[tokio::test]
async fn accept_trade_propagates_trade_already_accepted_from_use_case() {
    let response = post_accept_failing_with(|| FunctionalError::TradeAlreadyAccepted).await;

    assert_error(response, StatusCode::CONFLICT, "trade_already_accepted").await;
}

#[tokio::test]
async fn accept_trade_propagates_trade_not_acceptable_from_use_case() {
    let response = post_accept_failing_with(|| FunctionalError::TradeNotAcceptable).await;

    assert_error(response, StatusCode::CONFLICT, "trade_not_acceptable").await;
}

// --- abandon_trade ---

fn abandon_trade_app(abandon_trade_use_case: MockAbandonTradeUseCase) -> axum::Router {
    create_router(AppState {
        abandon_trade_use_case: Arc::new(abandon_trade_use_case),
        ..AppState::for_testing()
    })
}

async fn post_abandon_failing_with(error: fn() -> FunctionalError) -> Response {
    let mut mock_use_case = MockAbandonTradeUseCase::new();
    mock_use_case
        .expect_abandon()
        .times(1)
        .returning(move |_, _| Box::pin(async move { Err(AppError::Functional(error())) }));

    authed(
        abandon_trade_app(mock_use_case),
        Method::POST,
        &format!("/trades/{TRADE_ID}/abandon"),
    )
    .await
}

#[tokio::test]
async fn abandon_trade_returns_no_content_on_success() {
    let mut mock_use_case = MockAbandonTradeUseCase::new();
    mock_use_case
        .expect_abandon()
        .times(1)
        .withf(|trade_id, caller| {
            *trade_id == TradeId(TRADE_ID) && *caller == User::for_testing().id
        })
        .returning(|_, _| Box::pin(async { Ok(()) }));

    let response = authed(
        abandon_trade_app(mock_use_case),
        Method::POST,
        &format!("/trades/{TRADE_ID}/abandon"),
    )
    .await;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn abandon_trade_propagates_trade_not_found_from_use_case() {
    let response = post_abandon_failing_with(|| FunctionalError::TradeNotFound).await;

    assert_error(response, StatusCode::NOT_FOUND, "trade_not_found").await;
}

#[tokio::test]
async fn abandon_trade_propagates_trade_access_denied_from_use_case() {
    let response = post_abandon_failing_with(|| FunctionalError::TradeAccessDenied).await;

    assert_error(response, StatusCode::FORBIDDEN, "trade_access_denied").await;
}

#[tokio::test]
async fn abandon_trade_propagates_trade_already_finalized_from_use_case() {
    let response = post_abandon_failing_with(|| FunctionalError::TradeAlreadyFinalized).await;

    assert_error(response, StatusCode::CONFLICT, "trade_already_finalized").await;
}

// --- confirm_trade ---

fn confirm_trade_app(confirm_trade_use_case: MockConfirmTradeUseCase) -> axum::Router {
    create_router(AppState {
        confirm_trade_use_case: Arc::new(confirm_trade_use_case),
        ..AppState::for_testing()
    })
}

async fn post_confirm_failing_with(error: fn() -> FunctionalError) -> Response {
    let mut mock_use_case = MockConfirmTradeUseCase::new();
    mock_use_case
        .expect_confirm()
        .times(1)
        .returning(move |_, _| Box::pin(async move { Err(AppError::Functional(error())) }));

    authed(
        confirm_trade_app(mock_use_case),
        Method::POST,
        &format!("/trades/{TRADE_ID}/confirm"),
    )
    .await
}

#[tokio::test]
async fn confirm_trade_returns_no_content_on_success() {
    let mut mock_use_case = MockConfirmTradeUseCase::new();
    mock_use_case
        .expect_confirm()
        .times(1)
        .withf(|trade_id, caller| {
            *trade_id == TradeId(TRADE_ID) && *caller == User::for_testing().id
        })
        .returning(|_, _| Box::pin(async { Ok(()) }));

    let response = authed(
        confirm_trade_app(mock_use_case),
        Method::POST,
        &format!("/trades/{TRADE_ID}/confirm"),
    )
    .await;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn confirm_trade_propagates_trade_not_found_from_use_case() {
    let response = post_confirm_failing_with(|| FunctionalError::TradeNotFound).await;

    assert_error(response, StatusCode::NOT_FOUND, "trade_not_found").await;
}

#[tokio::test]
async fn confirm_trade_propagates_trade_access_denied_from_use_case() {
    let response = post_confirm_failing_with(|| FunctionalError::TradeAccessDenied).await;

    assert_error(response, StatusCode::FORBIDDEN, "trade_access_denied").await;
}

#[tokio::test]
async fn confirm_trade_propagates_trade_already_confirmed_from_use_case() {
    let response = post_confirm_failing_with(|| FunctionalError::TradeAlreadyConfirmed).await;

    assert_error(response, StatusCode::CONFLICT, "trade_already_confirmed").await;
}

#[tokio::test]
async fn confirm_trade_propagates_trade_not_fully_accepted_from_use_case() {
    let response = post_confirm_failing_with(|| FunctionalError::TradeNotFullyAccepted).await;

    assert_error(response, StatusCode::CONFLICT, "trade_not_fully_accepted").await;
}

// --- rate_trade ---

fn rate_trade_app(rate_trade_use_case: MockRateTradeUseCase) -> axum::Router {
    create_router(AppState {
        rate_trade_use_case: Arc::new(rate_trade_use_case),
        ..AppState::for_testing()
    })
}

async fn post_rate_failing_with(error: fn() -> FunctionalError) -> Response {
    let mut mock_use_case = MockRateTradeUseCase::new();
    mock_use_case
        .expect_rate()
        .times(1)
        .returning(move |_, _, _| Box::pin(async move { Err(AppError::Functional(error())) }));

    authed_json(
        rate_trade_app(mock_use_case),
        Method::POST,
        &format!("/trades/{TRADE_ID}/rate"),
        json!({ "rating": 5 }),
    )
    .await
}

#[tokio::test]
async fn rate_trade_returns_no_content_on_success() {
    let mut mock_use_case = MockRateTradeUseCase::new();
    mock_use_case
        .expect_rate()
        .times(1)
        .withf(|trade_id, caller, rating| {
            *trade_id == TradeId(TRADE_ID) && *caller == User::for_testing().id && *rating == 5
        })
        .returning(|_, _, _| Box::pin(async { Ok(()) }));

    let response = authed_json(
        rate_trade_app(mock_use_case),
        Method::POST,
        &format!("/trades/{TRADE_ID}/rate"),
        json!({ "rating": 5 }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn rate_trade_returns_bad_request_when_rating_out_of_range() {
    let response = authed_json(
        create_router(AppState::for_testing()),
        Method::POST,
        &format!("/trades/{TRADE_ID}/rate"),
        json!({ "rating": 6 }),
    )
    .await;

    assert_error(response, StatusCode::BAD_REQUEST, "wrong_format").await;
}

#[tokio::test]
async fn rate_trade_propagates_trade_not_found_from_use_case() {
    let response = post_rate_failing_with(|| FunctionalError::TradeNotFound).await;

    assert_error(response, StatusCode::NOT_FOUND, "trade_not_found").await;
}

#[tokio::test]
async fn rate_trade_propagates_trade_access_denied_from_use_case() {
    let response = post_rate_failing_with(|| FunctionalError::TradeAccessDenied).await;

    assert_error(response, StatusCode::FORBIDDEN, "trade_access_denied").await;
}

#[tokio::test]
async fn rate_trade_propagates_trade_already_rated_from_use_case() {
    let response = post_rate_failing_with(|| FunctionalError::TradeAlreadyRated).await;

    assert_error(response, StatusCode::CONFLICT, "trade_already_rated").await;
}

#[tokio::test]
async fn rate_trade_propagates_trade_abandoned_by_caller_from_use_case() {
    let response = post_rate_failing_with(|| FunctionalError::TradeAbandonedByCaller).await;

    assert_error(response, StatusCode::CONFLICT, "trade_abandoned_by_caller").await;
}

#[tokio::test]
async fn rate_trade_propagates_trade_abandoned_automatically_from_use_case() {
    let response = post_rate_failing_with(|| FunctionalError::TradeAbandonedAutomatically).await;

    assert_error(
        response,
        StatusCode::CONFLICT,
        "trade_abandoned_automatically",
    )
    .await;
}

#[tokio::test]
async fn rate_trade_propagates_trade_not_completed_from_use_case() {
    let response = post_rate_failing_with(|| FunctionalError::TradeNotCompleted).await;

    assert_error(response, StatusCode::CONFLICT, "trade_not_completed").await;
}

// --- get_trade ---

fn get_trade_app(get_trade_use_case: MockGetTradeUseCase) -> axum::Router {
    create_router(AppState {
        get_trade_use_case: Arc::new(get_trade_use_case),
        ..AppState::for_testing()
    })
}

fn make_trade_detail() -> TradeDetail {
    TradeDetail {
        id: TradeId(TRADE_ID),
        status: TradeStatus::Pending,
        partner_username: "bob".to_string(),
        my_cards: vec![],
        partner_cards: vec![],
        me: TradePartyState {
            accepted: false,
            confirmed: false,
            rating: None,
            abandoned: false,
        },
        partner: TradePartyState {
            accepted: false,
            confirmed: false,
            rating: None,
            abandoned: false,
        },
    }
}

async fn get_trade_failing_with(error: fn() -> FunctionalError) -> Response {
    let mut mock_use_case = MockGetTradeUseCase::new();
    mock_use_case
        .expect_get_trade()
        .times(1)
        .returning(move |_, _| Box::pin(async move { Err(AppError::Functional(error())) }));

    authed(
        get_trade_app(mock_use_case),
        Method::GET,
        &format!("/trades/{TRADE_ID}"),
    )
    .await
}

#[tokio::test]
async fn get_trade_returns_detail_on_success() {
    let mut mock_use_case = MockGetTradeUseCase::new();
    mock_use_case
        .expect_get_trade()
        .times(1)
        .withf(|trade_id, caller| {
            *trade_id == TradeId(TRADE_ID) && *caller == User::for_testing().id
        })
        .returning(|_, _| Box::pin(async { Ok(make_trade_detail()) }));

    let response = authed(
        get_trade_app(mock_use_case),
        Method::GET,
        &format!("/trades/{TRADE_ID}"),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        json_of(response).await,
        json!({
            "id": TRADE_ID_STR,
            "status": "PENDING",
            "partner_username": "bob",
            "my_cards": [],
            "partner_cards": [],
            "me": { "accepted": false, "confirmed": false, "rating": null, "abandoned": false },
            "partner": { "accepted": false, "confirmed": false, "rating": null, "abandoned": false },
        })
    );
}

#[tokio::test]
async fn get_trade_propagates_trade_not_found() {
    let response = get_trade_failing_with(|| FunctionalError::TradeNotFound).await;

    assert_error(response, StatusCode::NOT_FOUND, "trade_not_found").await;
}

#[tokio::test]
async fn get_trade_propagates_trade_access_denied() {
    let response = get_trade_failing_with(|| FunctionalError::TradeAccessDenied).await;

    assert_error(response, StatusCode::FORBIDDEN, "trade_access_denied").await;
}

#[tokio::test]
async fn get_trade_response_maps_card_details() {
    let scryfall_id = Uuid::from_u128(0xabcd);
    let card = TradeCardDetail {
        card_id: copy_id(),
        owner_user_id: UserId::new("bob"),
        name: "Goblin Boarders".to_string(),
        quantity: 3,
        price_guide: Some(PriceGuide::new(150u32, 220u32, 200u32)),
        scryfall_id,
        image: Some(CardImage {
            source: CardImageSource::GathererEn,
            has_back: true,
        }),
    };
    let mut mock_use_case = MockGetTradeUseCase::new();
    mock_use_case
        .expect_get_trade()
        .times(1)
        .returning(move |_, _| {
            let card = card.clone();
            Box::pin(async move {
                Ok(TradeDetail {
                    partner_cards: vec![card],
                    ..make_trade_detail()
                })
            })
        });

    let response = authed(
        get_trade_app(mock_use_case),
        Method::GET,
        &format!("/trades/{TRADE_ID}"),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_of(response).await;
    assert_eq!(body["my_cards"], json!([]));
    assert_eq!(body["partner_cards"].as_array().unwrap().len(), 1);
    let card = &body["partner_cards"][0];
    assert_eq!(card["set_code"], "FDN");
    assert_eq!(card["collector_number"], "87");
    assert_eq!(card["language_code"], "FR");
    assert_eq!(card["foil"], false);
    assert_eq!(card["name"], "Goblin Boarders");
    assert_eq!(card["quantity"], 3);
    assert_eq!(card["scryfall_id"], "00000000-0000-0000-0000-00000000abcd");
    assert_eq!(card["image_url"], "/card-images/FDN_87_EN.webp?v=gatherer");
    assert_eq!(
        card["image_back_url"],
        "/card-images/FDN_87_EN_back.webp?v=gatherer"
    );
    assert_eq!(card["price_guide"]["low"], 150);
    assert_eq!(card["price_guide"]["avg"], 200);
    assert_eq!(card["price_guide"]["trend"], 220);
}

// --- list_trades ---

fn list_trades_app(list_trades_use_case: MockListTradesUseCase) -> axum::Router {
    create_router(AppState {
        list_trades_use_case: Arc::new(list_trades_use_case),
        ..AppState::for_testing()
    })
}

#[test]
fn list_trades_params_deserializes_with_default_page_and_page_size() {
    let params: ListTradesParams = serde_json::from_value(serde_json::json!({})).unwrap();
    assert_eq!(params.page, 0);
    assert_eq!(params.page_size, 20);
}

#[tokio::test]
async fn list_trades_returns_paginated_response_on_success() {
    let mut mock_use_case = MockListTradesUseCase::new();
    mock_use_case
        .expect_list_trades()
        .times(1)
        .withf(|caller, query| {
            *caller == User::for_testing().id
                && query.statuses.is_empty()
                && query.pagination
                    == PageRequest {
                        page: 0,
                        page_size: 20,
                    }
        })
        .returning(|_, _| Box::pin(async { Ok(empty_page()) }));

    let response = authed(list_trades_app(mock_use_case), Method::GET, "/trades").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        json_of(response).await,
        json!({ "items": [], "total": 0, "page": 0, "page_size": 20 })
    );
}

#[tokio::test]
async fn list_trades_returns_empty_items_with_nonzero_total_for_page_beyond_last_result() {
    let mut mock_use_case = MockListTradesUseCase::new();
    mock_use_case
        .expect_list_trades()
        .times(1)
        .returning(|_, _| {
            Box::pin(async {
                Ok(Paginated {
                    items: vec![],
                    total: 9,
                    pagination: Pagination::try_new(5, 20, u32::MAX).unwrap(),
                })
            })
        });

    let response = authed(
        list_trades_app(mock_use_case),
        Method::GET,
        "/trades?page=5&page_size=20",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        json_of(response).await,
        json!({ "items": [], "total": 9, "page": 5, "page_size": 20 })
    );
}

#[tokio::test]
async fn list_trades_passes_the_requested_page_to_the_use_case_unvalidated() {
    // Validating the page against the trade list's depth limit is the use case's job: the
    // controller forwards whatever the client asked for, even a page the use case will reject.
    let mut mock_use_case = MockListTradesUseCase::new();
    mock_use_case
        .expect_list_trades()
        .times(1)
        .withf(|_, query| {
            query.pagination
                == PageRequest {
                    page: 1000,
                    page_size: 500,
                }
        })
        .returning(|_, _| {
            Box::pin(async {
                Err(AppError::Functional(FunctionalError::InvalidPageSize {
                    requested: 500,
                    max: 100,
                }))
            })
        });

    let response = authed(
        list_trades_app(mock_use_case),
        Method::GET,
        "/trades?page=1000&page_size=500",
    )
    .await;

    assert_error(response, StatusCode::BAD_REQUEST, "invalid_page_size").await;
}

#[tokio::test]
async fn list_trades_response_maps_trade_summaries() {
    let updated_at = Utc.with_ymd_and_hms(2026, 3, 14, 9, 30, 0).unwrap();
    let mut mock_use_case = MockListTradesUseCase::new();
    mock_use_case
        .expect_list_trades()
        .times(1)
        .returning(move |_, _| {
            Box::pin(async move {
                Ok(Paginated {
                    items: vec![TradeSummary {
                        id: TradeId(TRADE_ID),
                        status: TradeStatus::OneAccepted,
                        partner_username: "bob".to_string(),
                        my_card_count: 2,
                        partner_card_count: 5,
                        updated_at,
                    }],
                    total: 1,
                    pagination: Pagination::default(),
                })
            })
        });

    let response = authed(list_trades_app(mock_use_case), Method::GET, "/trades").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        json_of(response).await,
        json!({
            "items": [{
                "id": TRADE_ID_STR,
                "status": "ONE_ACCEPTED",
                "partner_username": "bob",
                "my_card_count": 2,
                "partner_card_count": 5,
                "updated_at": "2026-03-14T09:30:00+00:00",
            }],
            "total": 1,
            "page": 0,
            "page_size": 20,
        })
    );
}

#[tokio::test]
async fn list_trades_maps_every_status_param_variant_to_domain_status() {
    let mut mock_use_case = MockListTradesUseCase::new();
    mock_use_case
        .expect_list_trades()
        .times(1)
        .withf(|_, query| {
            query.statuses
                == vec![
                    TradeStatus::Pending,
                    TradeStatus::OneAccepted,
                    TradeStatus::FullyAccepted,
                    TradeStatus::Completed,
                    TradeStatus::Closed,
                    TradeStatus::Abandoned,
                ]
        })
        .returning(|_, _| Box::pin(async { Ok(empty_page()) }));

    let response = authed(
        list_trades_app(mock_use_case),
        Method::GET,
        "/trades?status=PENDING&status=ONE_ACCEPTED&status=FULLY_ACCEPTED&status=COMPLETED\
         &status=CLOSED&status=ABANDONED",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn list_trades_rejects_an_unknown_status() {
    let response = authed(
        create_router(AppState::for_testing()),
        Method::GET,
        "/trades?status=NOPE",
    )
    .await;

    assert_error(response, StatusCode::BAD_REQUEST, "invalid_request").await;
}

// --- authentication ---

#[tokio::test]
async fn every_route_refuses_an_anonymous_request() {
    let routes = [
        (Method::POST, "/trades".to_string()),
        (Method::GET, "/trades".to_string()),
        (Method::GET, format!("/trades/{TRADE_ID}")),
        (Method::POST, format!("/trades/{TRADE_ID}/cards")),
        (Method::POST, format!("/trades/{TRADE_ID}/cards/remove")),
        (Method::POST, format!("/trades/{TRADE_ID}/accept")),
        (Method::POST, format!("/trades/{TRADE_ID}/abandon")),
        (Method::POST, format!("/trades/{TRADE_ID}/confirm")),
        (Method::POST, format!("/trades/{TRADE_ID}/rate")),
    ];

    let routes: Vec<(Method, &str)> = routes
        .iter()
        .map(|(method, uri)| (method.clone(), uri.as_str()))
        .collect();

    assert_routes_refuse_anonymous(|| create_router(AppState::for_testing()), &routes).await;
}
