//! `AppState` test harness: every use case defaults to a mock.
//!
//! A test overrides only the field it cares about with struct update syntax:
//! `AppState { get_collection_use_case: Arc::new(mock), ..AppState::for_testing() }`.
//!
//! Most mocks have no expectation and panic if called. Four are pre-armed to succeed on any
//! call: `import_card_use_case`, `edh_rec_caller_adapter`, `token_validator` and
//! `import_price_use_case`. Override them to assert on calls.
//!
//! The pre-armed `token_validator` accepts any token and resolves it to `User::for_testing()`.
//! A test that needs another caller overrides `token_validator` with a mock returning that user.
//!
//! Requests go through `get`, `send`, `authed`, `authed_json` and `anonymous`; responses are read
//! with `json_of`. `assert_error` checks an error response, `assert_routes_refuse_anonymous` the
//! `401` of a table of routes.

use super::AppState;
use crate::application::caller::MockEdhRecCaller;
use crate::application::service::showcase::get::ShowcaseService;
use crate::application::service::stats::get::StatsService;
use crate::application::token_validator::MockTokenValidator;
use crate::application::use_case::{
    MockAbandonTradeUseCase, MockAcceptTradeUseCase, MockAddTradeBinderUseCase,
    MockAddTradeCardUseCase, MockAutocompleteUsersUseCase, MockConfirmTradeUseCase,
    MockCreateBinderUseCase, MockCreateTradeUseCase, MockDeleteBinderUseCase,
    MockEditCollectionEntryUseCase, MockEnqueueCardImageUpdateUseCase,
    MockEnqueueCardMarketIdUpdateUseCase, MockGetCardImportUseCase, MockGetCardOffersUseCase,
    MockGetCardPriceHistoryUseCase, MockGetCollectionEntriesUseCase,
    MockGetCollectionPriceHistoryUseCase, MockGetCollectionStatsUseCase, MockGetCollectionUseCase,
    MockGetCollectionVisibilityUseCase, MockGetRarityTradeFiltersUseCase, MockGetSetUseCase,
    MockGetShowcaseUseCase, MockGetTradeBindersUseCase, MockGetTradeUseCase,
    MockGetUserProfileUseCase, MockImportCardUseCase, MockImportPriceUseCase,
    MockListBindersUseCase, MockListSetsUseCase, MockListTradesUseCase, MockRateTradeUseCase,
    MockRegisterUserUseCase, MockRemoveCollectionEntryUseCase, MockRemoveTradeBinderUseCase,
    MockRemoveTradeCardUseCase, MockRenameBinderUseCase, MockScanIndexUseCase,
    MockSearchCardsUseCase, MockSearchSetsUseCase, MockSetCollectionVisibilityUseCase,
    MockSetRarityTradeFilterUseCase, MockStatsUseCase, MockUpdateCatalogUseCase,
};
use crate::domain::card::CardInfo;
use crate::domain::card_import::CardImportId;
use crate::domain::user::User;
use crate::infrastructure::adapter_out::repository::showcase_repository_adapter::ShowcaseRepositoryAdapter;
use crate::infrastructure::adapter_out::repository::stats_repository_adapter::StatsRepositoryAdapter;
use crate::infrastructure::create_router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use axum::response::Response;
use serde_json::Value;
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

/// The whole API router, with the public endpoints (`/stats`, `/showcase`) wired to the real
/// database.
pub(crate) fn public_app_on(pool: PgPool) -> axum::Router {
    create_router(AppState {
        stats_use_case: Arc::new(StatsService::new(Arc::new(StatsRepositoryAdapter::new(
            pool.clone(),
        )))),
        get_showcase_use_case: Arc::new(ShowcaseService::new(Arc::new(
            ShowcaseRepositoryAdapter::new(pool),
        ))),
        ..AppState::for_testing()
    })
}

/// An anonymous GET: no `Authorization` header.
pub(crate) async fn get(app: axum::Router, uri: &str) -> Response {
    app.oneshot(Request::get(uri).body(Body::empty()).unwrap())
        .await
        .unwrap()
}

/// Sends a hand-built `Request` (a multipart import, say) through the router.
pub(crate) async fn send(app: axum::Router, request: Request<Body>) -> Response {
    app.oneshot(request).await.unwrap()
}

/// A request without body, authenticated as the caller: any `Bearer` token is accepted, and
/// resolves to `User::for_testing()` unless the test overrides `token_validator`.
pub(crate) async fn authed(app: axum::Router, method: Method, uri: &str) -> Response {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, BEARER)
        .body(Body::empty())
        .unwrap();
    send(app, request).await
}

/// Like [`authed`], with `body` as the JSON request body.
pub(crate) async fn authed_json(
    app: axum::Router,
    method: Method,
    uri: &str,
    body: Value,
) -> Response {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, BEARER)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    send(app, request).await
}

/// A request without body and without `Authorization` header, whatever the verb: a `401` on an
/// authenticated route, the real answer on a public one.
pub(crate) async fn anonymous(app: axum::Router, method: Method, uri: &str) -> Response {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    send(app, request).await
}

pub(crate) const BEARER: &str = "Bearer test-token";

pub(crate) async fn json_of(response: Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// Asserts the response has this `status` and an `{error, code}` body carrying this `code`.
pub(crate) async fn assert_error(response: Response, status: StatusCode, code: &str) {
    assert_eq!(response.status(), status);
    let body = json_of(response).await;
    assert_eq!(body["code"], code);
    assert!(body["error"].is_string(), "missing `error` in {body}");
}

/// Asserts that each `(verb, path)` of `routes`, sent without `Authorization` header to a router
/// built by `app`, is refused with a `401` and the `{error, code: "unauthorized"}` body. The
/// failure names the route.
pub(crate) async fn assert_routes_refuse_anonymous(
    app: impl Fn() -> axum::Router,
    routes: &[(Method, &str)],
) {
    for (method, uri) in routes {
        let response = anonymous(app(), method.clone(), uri).await;

        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "{method} {uri} must refuse an anonymous request"
        );
        let body = json_of(response).await;
        assert_eq!(
            body["code"], "unauthorized",
            "{method} {uri} must answer with the error body"
        );
        assert!(
            body["error"].is_string(),
            "{method} {uri} must answer with an `error` message"
        );
    }
}

impl AppState {
    pub(crate) fn for_testing() -> Self {
        let mut mock_import_card = MockImportCardUseCase::new();
        mock_import_card
            .expect_start_import()
            .returning(|_, _| Box::pin(async { Ok(CardImportId::new()) }));

        let mut mock_edh_rec = MockEdhRecCaller::new();
        mock_edh_rec.expect_get_card_info().returning(|_| {
            Box::pin(async {
                Ok(CardInfo {
                    inclusion: 0,
                    total_decks: 0,
                })
            })
        });

        let mut mock_auth = MockTokenValidator::new();
        mock_auth
            .expect_validate_token()
            .returning(|_| Ok(User::for_testing()));

        let mut mock_import_price = MockImportPriceUseCase::new();
        mock_import_price
            .expect_import_prices_for_current_date()
            .returning(|| Box::pin(async { Ok(()) }));

        Self {
            import_card_use_case: Arc::new(mock_import_card),
            edh_rec_caller_adapter: Arc::new(mock_edh_rec),
            stats_use_case: Arc::new(MockStatsUseCase::new()),
            get_showcase_use_case: Arc::new(MockGetShowcaseUseCase::new()),
            token_validator: Arc::new(mock_auth),
            get_collection_use_case: Arc::new(MockGetCollectionUseCase::new()),
            search_cards_use_case: Arc::new(MockSearchCardsUseCase::new()),
            search_sets_use_case: Arc::new(MockSearchSetsUseCase::new()),
            import_price_use_case: Arc::new(mock_import_price),
            enqueue_cardmarket_id_use_case: Arc::new(MockEnqueueCardMarketIdUpdateUseCase::new()),
            enqueue_card_image_use_case: Arc::new(MockEnqueueCardImageUpdateUseCase::new()),
            get_collection_price_history_use_case: Arc::new(
                MockGetCollectionPriceHistoryUseCase::new(),
            ),
            get_card_price_history_use_case: Arc::new(MockGetCardPriceHistoryUseCase::new()),
            get_collection_stats_use_case: Arc::new(MockGetCollectionStatsUseCase::new()),
            register_user_use_case: Arc::new(MockRegisterUserUseCase::new()),
            get_user_profile_use_case: Arc::new(MockGetUserProfileUseCase::new()),
            create_trade_use_case: Arc::new(MockCreateTradeUseCase::new()),
            accept_trade_use_case: Arc::new(MockAcceptTradeUseCase::new()),
            abandon_trade_use_case: Arc::new(MockAbandonTradeUseCase::new()),
            confirm_trade_use_case: Arc::new(MockConfirmTradeUseCase::new()),
            rate_trade_use_case: Arc::new(MockRateTradeUseCase::new()),
            get_card_offers_use_case: Arc::new(MockGetCardOffersUseCase::new()),
            autocomplete_users_use_case: Arc::new(MockAutocompleteUsersUseCase::new()),
            get_trade_use_case: Arc::new(MockGetTradeUseCase::new()),
            list_trades_use_case: Arc::new(MockListTradesUseCase::new()),
            add_trade_card_use_case: Arc::new(MockAddTradeCardUseCase::new()),
            remove_trade_card_use_case: Arc::new(MockRemoveTradeCardUseCase::new()),
            get_collection_visibility_use_case: Arc::new(MockGetCollectionVisibilityUseCase::new()),
            set_collection_visibility_use_case: Arc::new(MockSetCollectionVisibilityUseCase::new()),
            get_collection_entries_use_case: Arc::new(MockGetCollectionEntriesUseCase::new()),
            edit_collection_entry_use_case: Arc::new(MockEditCollectionEntryUseCase::new()),
            remove_collection_entry_use_case: Arc::new(MockRemoveCollectionEntryUseCase::new()),
            list_binders_use_case: Arc::new(MockListBindersUseCase::new()),
            create_binder_use_case: Arc::new(MockCreateBinderUseCase::new()),
            rename_binder_use_case: Arc::new(MockRenameBinderUseCase::new()),
            delete_binder_use_case: Arc::new(MockDeleteBinderUseCase::new()),
            get_trade_binders_use_case: Arc::new(MockGetTradeBindersUseCase::new()),
            add_trade_binder_use_case: Arc::new(MockAddTradeBinderUseCase::new()),
            remove_trade_binder_use_case: Arc::new(MockRemoveTradeBinderUseCase::new()),
            get_rarity_trade_filters_use_case: Arc::new(MockGetRarityTradeFiltersUseCase::new()),
            set_rarity_trade_filter_use_case: Arc::new(MockSetRarityTradeFilterUseCase::new()),
            list_sets_use_case: Arc::new(MockListSetsUseCase::new()),
            get_set_use_case: Arc::new(MockGetSetUseCase::new()),
            card_import_query_use_case: Arc::new(MockGetCardImportUseCase::new()),
            update_catalog_use_case: Arc::new(MockUpdateCatalogUseCase::new()),
            scan_index_use_case: Arc::new(MockScanIndexUseCase::new()),
        }
    }
}
