use crate::application::caller::EdhRecCaller;
use crate::application::service::auth_service::AuthService;
use crate::application::service::autocomplete_user_service::AutocompleteUserService;
use crate::application::service::card_collection_service::CardCollectionService;
use crate::application::service::card_offer_service::CardOfferService;
use crate::application::service::card_price_history_service::CardPriceHistoryService;
use crate::application::service::cardmarket_id_enqueue_service::CardMarketIdEnqueueService;
use crate::application::service::collection_price_history_service::CollectionPriceHistoryService;
use crate::application::service::collection_service::CollectionService;
use crate::application::service::collection_stats_service::CollectionStatsService;
use crate::application::service::collection_visibility_service::{
    GetCollectionVisibilityService, SetCollectionVisibilityService,
};
use crate::application::service::gatherer_id_enqueue_service::GathererIdEnqueueService;
use crate::application::service::get_user_profile_service::GetUserProfileService;
use crate::application::service::import_card_service::ImportCardService;
use crate::application::service::import_price_service::ImportPriceService;
use crate::application::service::rarity_trade_filter_service::{
    GetRarityTradeFiltersService, SetRarityTradeFilterService,
};
use crate::application::service::register_user_service::RegisterUserService;
use crate::application::service::search_service::SearchService;
use crate::application::service::set_service::SetService;
use crate::application::service::stats_service::StatsService;
use crate::application::service::trade_binder_service::{
    AddTradeBinderService, GetTradeBindersService, RemoveTradeBinderService,
};
use crate::application::service::trade_service::{
    AbandonTradeService, AcceptTradeService, AddTradeCardService, ConfirmTradeService,
    CreateTradeService, GetTradeService, ListTradesService, RateTradeService,
    RemoveTradeCardService,
};
use crate::application::service::update_card_market_service::CardMarketIdWorker;
use crate::application::service::update_gatherer_service::GathererIdWorker;
use crate::application::use_case::{
    AbandonTradeUseCase, AcceptTradeUseCase, AddTradeBinderUseCase, AddTradeCardUseCase,
    AutocompleteUsersUseCase, ConfirmTradeUseCase, CreateTradeUseCase,
    EnqueueCardMarketIdUpdateUseCase, EnqueueGathererIdUpdateUseCase, GetCardOffersUseCase,
    GetCardPriceHistoryUseCase, GetCollectionPriceHistoryUseCase, GetCollectionStatsUseCase,
    GetCollectionUseCase, GetCollectionVisibilityUseCase, GetRarityTradeFiltersUseCase,
    GetSetUseCase, GetTradeBindersUseCase, GetTradeUseCase, GetUserProfileUseCase,
    ImportCardUseCase, ImportPriceUseCase, ListSetsUseCase, ListTradesUseCase, RateTradeUseCase,
    RegisterUserUseCase, RemoveTradeBinderUseCase, RemoveTradeCardUseCase, SearchCardsUseCase,
    SetCollectionVisibilityUseCase, SetRarityTradeFilterUseCase, StatsUseCase,
};
use crate::config::Config;
use crate::domain::card::CardId;
use crate::infrastructure::adapter_in::autocomplete::controller::create_autocomplete_router;
use crate::infrastructure::adapter_in::card::controller::create_card_router;
use crate::infrastructure::adapter_in::collection::controller::create_collection_router;
use crate::infrastructure::adapter_in::search::controller::create_search_router;
use crate::infrastructure::adapter_in::sets::controller::create_set_router;
use crate::infrastructure::adapter_in::trade::controller::create_trade_router;
use crate::infrastructure::adapter_in::user::controller::create_user_router;
use crate::infrastructure::adapter_out::caller::cardmarket_caller_adapter::CardMarketCallerAdapter;
use crate::infrastructure::adapter_out::caller::edhrec_caller_adapter::EdhRecCallerAdapter;
use crate::infrastructure::adapter_out::repository::card_prices_view_repository_adapter::CardPricesViewRepositoryAdapter;
use crate::infrastructure::adapter_out::repository::cardmarket_price_repository_adapter::CardMarketPriceRepositoryAdapter;
use crate::infrastructure::adapter_out::repository::collection_price_history_repository_adapter::CollectionPriceHistoryRepositoryAdapter;
use crate::infrastructure::adapter_out::repository::collection_rarity_filters_repository_adapter::CollectionRarityFiltersRepositoryAdapter;
use crate::infrastructure::adapter_out::repository::collection_stats_repository_adapter::CollectionStatsRepositoryAdapter;
use crate::infrastructure::adapter_out::repository::stats_repository_adapter::StatsRepositoryAdapter;
use crate::infrastructure::adapter_out::repository::trade_repository_adapter::TradeRepositoryAdapter;
use crate::infrastructure::adapter_out::repository::trading_binders_repository_adapter::TradingBindersRepositoryAdapter;
use adapter_in::maintenance::controller::create_maintenance_router;
use adapter_out::caller::gatherer_caller_adapter::GathererCallerAdapter;
use adapter_out::caller::scryfall_caller_adapter::ScryfallCallerAdapter;
use adapter_out::repository::card_repository_adapter::CardRepositoryAdapter;
use adapter_out::repository::set_names_repository_adapter::SetNameRepositoryAdapter;
use adapter_out::repository::user_repository_adapter::UserRepositoryAdapter;
use axum::Router;
use axum::body::Body;
use axum::http::Request;
use chrono::Utc;
use cron_tab::AsyncCron;
use sentry::integrations::tower::{NewSentryLayer, SentryHttpLayer};
use sqlx::{Pool, Postgres};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub mod adapter_in;
pub mod adapter_out;

// ---- AppState ----
#[derive(Clone)]
pub struct AppState {
    pub import_card_use_case: Arc<dyn ImportCardUseCase>,
    pub edh_rec_caller_adapter: Arc<dyn EdhRecCaller>,
    pub stats_use_case: Arc<dyn StatsUseCase>,
    pub auth_service: Arc<dyn AuthService>,
    pub get_collection_use_case: Arc<dyn GetCollectionUseCase>,
    pub search_cards_use_case: Arc<dyn SearchCardsUseCase>,
    pub import_price_use_case: Arc<dyn ImportPriceUseCase>,
    pub enqueue_cardmarket_id_use_case: Arc<dyn EnqueueCardMarketIdUpdateUseCase>,
    pub enqueue_gatherer_id_use_case: Arc<dyn EnqueueGathererIdUpdateUseCase>,
    pub get_collection_price_history_use_case: Arc<dyn GetCollectionPriceHistoryUseCase>,
    pub get_card_price_history_use_case: Arc<dyn GetCardPriceHistoryUseCase>,
    pub get_collection_stats_use_case: Arc<dyn GetCollectionStatsUseCase>,
    pub register_user_use_case: Arc<dyn RegisterUserUseCase>,
    pub get_user_profile_use_case: Arc<dyn GetUserProfileUseCase>,
    pub create_trade_use_case: Arc<dyn CreateTradeUseCase>,
    pub accept_trade_use_case: Arc<dyn AcceptTradeUseCase>,
    pub abandon_trade_use_case: Arc<dyn AbandonTradeUseCase>,
    pub confirm_trade_use_case: Arc<dyn ConfirmTradeUseCase>,
    pub rate_trade_use_case: Arc<dyn RateTradeUseCase>,
    pub get_card_offers_use_case: Arc<dyn GetCardOffersUseCase>,
    pub autocomplete_users_use_case: Arc<dyn AutocompleteUsersUseCase>,
    pub get_trade_use_case: Arc<dyn GetTradeUseCase>,
    pub list_trades_use_case: Arc<dyn ListTradesUseCase>,
    pub add_trade_card_use_case: Arc<dyn AddTradeCardUseCase>,
    pub remove_trade_card_use_case: Arc<dyn RemoveTradeCardUseCase>,
    pub get_collection_visibility_use_case: Arc<dyn GetCollectionVisibilityUseCase>,
    pub set_collection_visibility_use_case: Arc<dyn SetCollectionVisibilityUseCase>,
    pub get_trade_binders_use_case: Arc<dyn GetTradeBindersUseCase>,
    pub add_trade_binder_use_case: Arc<dyn AddTradeBinderUseCase>,
    pub remove_trade_binder_use_case: Arc<dyn RemoveTradeBinderUseCase>,
    pub get_rarity_trade_filters_use_case: Arc<dyn GetRarityTradeFiltersUseCase>,
    pub set_rarity_trade_filter_use_case: Arc<dyn SetRarityTradeFilterUseCase>,
    pub list_sets_use_case: Arc<dyn ListSetsUseCase>,
    pub get_set_use_case: Arc<dyn GetSetUseCase>,
}

// ---- Repositories ----
struct Repositories {
    card: Arc<CardRepositoryAdapter>,
    set_name: Arc<SetNameRepositoryAdapter>,
    card_market: Arc<CardMarketPriceRepositoryAdapter>,
    card_prices_view: Arc<CardPricesViewRepositoryAdapter>,
    stats: Arc<StatsRepositoryAdapter>,
    user: Arc<UserRepositoryAdapter>,
    trade: Arc<TradeRepositoryAdapter>,
    collection_price_history: Arc<CollectionPriceHistoryRepositoryAdapter>,
    collection_stats: Arc<CollectionStatsRepositoryAdapter>,
    trading_binders: Arc<TradingBindersRepositoryAdapter>,
    collection_rarity_filters: Arc<CollectionRarityFiltersRepositoryAdapter>,
}

fn create_repositories(pool: &Pool<Postgres>) -> Repositories {
    Repositories {
        card: Arc::new(CardRepositoryAdapter::new(pool.clone())),
        set_name: Arc::new(SetNameRepositoryAdapter::new(pool.clone())),
        card_market: Arc::new(CardMarketPriceRepositoryAdapter::new(pool.clone())),
        card_prices_view: Arc::new(CardPricesViewRepositoryAdapter::new(pool.clone())),
        stats: Arc::new(StatsRepositoryAdapter::new(pool.clone())),
        user: Arc::new(UserRepositoryAdapter::new(pool.clone())),
        trade: Arc::new(TradeRepositoryAdapter::new(pool.clone())),
        collection_price_history: Arc::new(CollectionPriceHistoryRepositoryAdapter::new(
            pool.clone(),
        )),
        collection_stats: Arc::new(CollectionStatsRepositoryAdapter::new(pool.clone())),
        trading_binders: Arc::new(TradingBindersRepositoryAdapter::new(pool.clone())),
        collection_rarity_filters: Arc::new(CollectionRarityFiltersRepositoryAdapter::new(
            pool.clone(),
        )),
    }
}

// ---- Callers ----
struct Callers {
    card_market: Arc<CardMarketCallerAdapter>,
    edh_rec: Arc<EdhRecCallerAdapter>,
    scryfall: Arc<ScryfallCallerAdapter>,
    gatherer: Arc<GathererCallerAdapter>,
}

fn create_callers(config: &Config) -> Callers {
    Callers {
        card_market: Arc::new(CardMarketCallerAdapter::new(
            config.cardmarket_price_guides_url.clone(),
        )),
        edh_rec: Arc::new(EdhRecCallerAdapter::new(config.edh_rec_base_url.clone())),
        scryfall: Arc::new(ScryfallCallerAdapter::new(
            config.scryfall_base_url.clone(),
            config.scryfall_rate_limit_tokens,
        )),
        gatherer: Arc::new(GathererCallerAdapter::new(config.gatherer_base_url.clone())),
    }
}

async fn create_auth_service(config: &Config) -> Arc<dyn AuthService> {
    Arc::new(
        crate::application::service::auth_service::ClerkAuthService::new(
            config.clerk_frontend_api_url.clone(),
            None,
        )
        .await
        .expect("Failed to initialize Clerk Auth Service"),
    )
}

// ---- Background workers ----
// Canal non borné + HashSet de déduplication partagé entre enqueue service et worker
fn spawn_cardmarket_id_worker(
    repos: &Repositories,
    scryfall_caller_adapter: Arc<ScryfallCallerAdapter>,
    card_collection_service: Arc<CardCollectionService>,
) -> Arc<CardMarketIdEnqueueService> {
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<(CardId, Uuid)>();
    let dedup_set = Arc::new(Mutex::new(HashSet::<CardId>::new()));

    let enqueue_service = Arc::new(CardMarketIdEnqueueService::new(
        repos.card.clone(),
        sender,
        dedup_set.clone(),
    ));

    let worker = CardMarketIdWorker::new(
        repos.card.clone(),
        scryfall_caller_adapter,
        card_collection_service,
        repos.card_prices_view.clone(),
        dedup_set,
    );
    tokio::spawn(async move {
        if let Err(e) = worker.run(receiver).await {
            tracing::error!("CardMarket worker terminated with error: {:?}", e);
        }
    });

    enqueue_service
}

// Canal + HashSet de déduplication dédiés à l'enrichissement the_gatherer_id
fn spawn_gatherer_id_worker(
    repos: &Repositories,
    gatherer_caller_adapter: Arc<GathererCallerAdapter>,
) -> Arc<GathererIdEnqueueService> {
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<(CardId, String)>();
    let dedup_set = Arc::new(Mutex::new(HashSet::<CardId>::new()));

    let enqueue_service = Arc::new(GathererIdEnqueueService::new(
        repos.card.clone(),
        sender,
        dedup_set.clone(),
    ));

    let worker = GathererIdWorker::new(
        repos.card.clone(),
        gatherer_caller_adapter,
        repos.card_prices_view.clone(),
        dedup_set,
    );
    tokio::spawn(async move {
        if let Err(e) = worker.run(receiver).await {
            tracing::error!("Gatherer worker terminated with error: {:?}", e);
        }
    });

    enqueue_service
}

// ---- App state assembly ----
#[allow(clippy::too_many_arguments)]
fn create_app_state(
    repos: Repositories,
    callers: Callers,
    auth_service: Arc<dyn AuthService>,
    card_collection_service: Arc<CardCollectionService>,
    enqueue_cardmarket_id_use_case: Arc<CardMarketIdEnqueueService>,
    enqueue_gatherer_id_use_case: Arc<GathererIdEnqueueService>,
) -> AppState {
    let import_card_service = Arc::new(ImportCardService::new(
        repos.card.clone(),
        repos.set_name.clone(),
        enqueue_cardmarket_id_use_case.clone(),
        enqueue_gatherer_id_use_case.clone(),
        repos.card_prices_view.clone(),
        repos.trading_binders.clone(),
    ));

    let import_price_use_case: Arc<dyn ImportPriceUseCase> = Arc::new(ImportPriceService::new(
        callers.card_market,
        repos.card_market.clone(),
        repos.card_prices_view.clone(),
        card_collection_service.clone(),
    ));

    let stats_service = Arc::new(StatsService::new(repos.stats));
    let collection_service = Arc::new(CollectionService::new(repos.card_prices_view.clone()));
    let search_service: Arc<dyn SearchCardsUseCase> =
        Arc::new(SearchService::new(repos.card_prices_view.clone()));
    let collection_price_history_service: Arc<dyn GetCollectionPriceHistoryUseCase> = Arc::new(
        CollectionPriceHistoryService::new(repos.collection_price_history.clone()),
    );
    let card_price_history_service: Arc<dyn GetCardPriceHistoryUseCase> = Arc::new(
        CardPriceHistoryService::new(repos.card.clone(), repos.card_market),
    );
    let collection_stats_service: Arc<dyn GetCollectionStatsUseCase> =
        Arc::new(CollectionStatsService::new(repos.collection_stats));
    let register_user_service: Arc<dyn RegisterUserUseCase> =
        Arc::new(RegisterUserService::new(repos.user.clone()));
    let get_user_profile_service: Arc<dyn GetUserProfileUseCase> =
        Arc::new(GetUserProfileService::new(repos.user.clone()));
    let get_collection_visibility_service: Arc<dyn GetCollectionVisibilityUseCase> =
        Arc::new(GetCollectionVisibilityService::new(repos.user.clone()));
    let set_collection_visibility_service: Arc<dyn SetCollectionVisibilityUseCase> =
        Arc::new(SetCollectionVisibilityService::new(repos.user.clone()));
    let get_trade_binders_service: Arc<dyn GetTradeBindersUseCase> =
        Arc::new(GetTradeBindersService::new(repos.trading_binders.clone()));
    let add_trade_binder_service: Arc<dyn AddTradeBinderUseCase> =
        Arc::new(AddTradeBinderService::new(repos.trading_binders.clone()));
    let remove_trade_binder_service: Arc<dyn RemoveTradeBinderUseCase> =
        Arc::new(RemoveTradeBinderService::new(repos.trading_binders));
    let get_rarity_trade_filters_service: Arc<dyn GetRarityTradeFiltersUseCase> = Arc::new(
        GetRarityTradeFiltersService::new(repos.collection_rarity_filters.clone()),
    );
    let set_rarity_trade_filter_service: Arc<dyn SetRarityTradeFilterUseCase> = Arc::new(
        SetRarityTradeFilterService::new(repos.collection_rarity_filters),
    );
    let create_trade_service: Arc<dyn CreateTradeUseCase> = Arc::new(CreateTradeService::new(
        repos.trade.clone(),
        repos.user.clone(),
    ));
    let accept_trade_service: Arc<dyn AcceptTradeUseCase> =
        Arc::new(AcceptTradeService::new(repos.trade.clone()));
    let abandon_trade_service: Arc<dyn AbandonTradeUseCase> =
        Arc::new(AbandonTradeService::new(repos.trade.clone()));
    let confirm_trade_service: Arc<dyn ConfirmTradeUseCase> =
        Arc::new(ConfirmTradeService::new(repos.trade.clone()));
    let rate_trade_service: Arc<dyn RateTradeUseCase> =
        Arc::new(RateTradeService::new(repos.trade.clone()));
    let card_offer_service: Arc<dyn GetCardOffersUseCase> =
        Arc::new(CardOfferService::new(repos.card_prices_view));
    let autocomplete_users_service: Arc<dyn AutocompleteUsersUseCase> =
        Arc::new(AutocompleteUserService::new(repos.user.clone()));
    let get_trade_service: Arc<dyn GetTradeUseCase> = Arc::new(GetTradeService::new(
        repos.trade.clone(),
        repos.user.clone(),
    ));
    let list_trades_service: Arc<dyn ListTradesUseCase> =
        Arc::new(ListTradesService::new(repos.trade.clone()));
    let add_trade_card_service: Arc<dyn AddTradeCardUseCase> = Arc::new(AddTradeCardService::new(
        repos.trade.clone(),
        repos.user.clone(),
    ));
    let remove_trade_card_service: Arc<dyn RemoveTradeCardUseCase> =
        Arc::new(RemoveTradeCardService::new(repos.trade, repos.user));
    let set_service = Arc::new(SetService::new(repos.set_name));

    AppState {
        import_card_use_case: import_card_service,
        edh_rec_caller_adapter: callers.edh_rec,
        stats_use_case: stats_service,
        auth_service,
        get_collection_use_case: collection_service,
        search_cards_use_case: search_service,
        import_price_use_case,
        enqueue_cardmarket_id_use_case,
        enqueue_gatherer_id_use_case,
        get_collection_price_history_use_case: collection_price_history_service,
        get_card_price_history_use_case: card_price_history_service,
        get_collection_stats_use_case: collection_stats_service,
        register_user_use_case: register_user_service,
        get_user_profile_use_case: get_user_profile_service,
        create_trade_use_case: create_trade_service,
        accept_trade_use_case: accept_trade_service,
        abandon_trade_use_case: abandon_trade_service,
        confirm_trade_use_case: confirm_trade_service,
        rate_trade_use_case: rate_trade_service,
        get_card_offers_use_case: card_offer_service,
        autocomplete_users_use_case: autocomplete_users_service,
        get_trade_use_case: get_trade_service,
        list_trades_use_case: list_trades_service,
        add_trade_card_use_case: add_trade_card_service,
        remove_trade_card_use_case: remove_trade_card_service,
        get_collection_visibility_use_case: get_collection_visibility_service,
        set_collection_visibility_use_case: set_collection_visibility_service,
        get_trade_binders_use_case: get_trade_binders_service,
        add_trade_binder_use_case: add_trade_binder_service,
        remove_trade_binder_use_case: remove_trade_binder_service,
        get_rarity_trade_filters_use_case: get_rarity_trade_filters_service,
        set_rarity_trade_filter_use_case: set_rarity_trade_filter_service,
        list_sets_use_case: set_service.clone(),
        get_set_use_case: set_service,
    }
}

async fn schedule_price_import_job(import_price_use_case: Arc<dyn ImportPriceUseCase>) {
    let mut cron = AsyncCron::new(Utc);

    cron.add_fn("0 0 */12 * * *", move || {
        let service = import_price_use_case.clone();
        async move {
            service
                .import_prices_for_current_date()
                .await
                .expect("Failed to import prices");
        }
    })
    .await
    .unwrap();

    cron.start().await;
}

fn create_router(app_state: AppState) -> Router {
    Router::new()
        .nest("/autocomplete", create_autocomplete_router())
        .nest("/card", create_card_router())
        .nest("/collection", create_collection_router())
        .nest("/search", create_search_router())
        .nest("/sets", create_set_router())
        .nest("/maintenance", create_maintenance_router())
        .nest("/user", create_user_router())
        .nest("/trades", create_trade_router())
        .with_state(app_state)
        .layer(NewSentryLayer::<Request<Body>>::new_from_top())
        .layer(SentryHttpLayer::new().enable_transaction())
}

pub async fn create_infra(pool: Pool<Postgres>, config: &Config) -> Router {
    let repos = create_repositories(&pool);
    let callers = create_callers(config);
    let auth_service = create_auth_service(config).await;

    let card_collection_service = Arc::new(CardCollectionService::new(
        repos.collection_price_history.clone(),
    ));

    let enqueue_cardmarket_id_use_case = spawn_cardmarket_id_worker(
        &repos,
        callers.scryfall.clone(),
        card_collection_service.clone(),
    );
    let enqueue_gatherer_id_use_case = spawn_gatherer_id_worker(&repos, callers.gatherer.clone());

    let app_state = create_app_state(
        repos,
        callers,
        auth_service,
        card_collection_service,
        enqueue_cardmarket_id_use_case,
        enqueue_gatherer_id_use_case,
    );

    schedule_price_import_job(app_state.import_price_use_case.clone()).await;

    create_router(app_state)
}

#[cfg(test)]
impl AppState {
    pub fn for_testing(stats_use_case: Arc<dyn StatsUseCase>) -> Self {
        use crate::application::use_case::MockImportPriceUseCase;
        let mut mock_import_price = MockImportPriceUseCase::new();
        mock_import_price
            .expect_import_prices_for_current_date()
            .returning(|| Box::pin(async { Ok(()) }));
        Self::for_testing_with_import_price(stats_use_case, Arc::new(mock_import_price))
    }

    pub fn for_testing_with_import_price(
        stats_use_case: Arc<dyn StatsUseCase>,
        import_price_use_case: Arc<dyn ImportPriceUseCase>,
    ) -> Self {
        use crate::application::caller::MockEdhRecCaller;
        use crate::application::service::auth_service::MockAuthService;
        use crate::application::use_case::{
            MockAbandonTradeUseCase, MockAcceptTradeUseCase, MockAddTradeBinderUseCase,
            MockAddTradeCardUseCase, MockAutocompleteUsersUseCase, MockConfirmTradeUseCase,
            MockCreateTradeUseCase, MockEnqueueCardMarketIdUpdateUseCase,
            MockEnqueueGathererIdUpdateUseCase, MockGetCardOffersUseCase,
            MockGetCardPriceHistoryUseCase, MockGetCollectionPriceHistoryUseCase,
            MockGetCollectionStatsUseCase, MockGetCollectionUseCase,
            MockGetCollectionVisibilityUseCase, MockGetRarityTradeFiltersUseCase,
            MockGetSetUseCase, MockGetTradeBindersUseCase, MockGetTradeUseCase,
            MockGetUserProfileUseCase, MockImportCardUseCase, MockListSetsUseCase,
            MockListTradesUseCase, MockRateTradeUseCase, MockRegisterUserUseCase,
            MockRemoveTradeBinderUseCase, MockRemoveTradeCardUseCase, MockSearchCardsUseCase,
            MockSetCollectionVisibilityUseCase, MockSetRarityTradeFilterUseCase,
        };
        use crate::domain::card::CardInfo;
        use crate::domain::user::User;

        let mut mock_import_card = MockImportCardUseCase::new();
        mock_import_card
            .expect_import_cards()
            .returning(|_, _| Box::pin(async { Ok(()) }));

        let mut mock_edh_rec = MockEdhRecCaller::new();
        mock_edh_rec.expect_get_card_info().returning(|_| {
            Box::pin(async {
                Ok(CardInfo {
                    inclusion: 0,
                    total_decks: 0,
                })
            })
        });

        let mut mock_auth = MockAuthService::new();
        mock_auth
            .expect_validate_token()
            .returning(|_| Ok(User::new("test-user-id".to_string(), None, None, None)));

        Self {
            import_card_use_case: Arc::new(mock_import_card),
            edh_rec_caller_adapter: Arc::new(mock_edh_rec),
            stats_use_case,
            auth_service: Arc::new(mock_auth),
            get_collection_use_case: Arc::new(MockGetCollectionUseCase::new()),
            search_cards_use_case: Arc::new(MockSearchCardsUseCase::new()),
            import_price_use_case,
            enqueue_cardmarket_id_use_case: Arc::new(MockEnqueueCardMarketIdUpdateUseCase::new()),
            enqueue_gatherer_id_use_case: Arc::new(MockEnqueueGathererIdUpdateUseCase::new()),
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
            get_trade_binders_use_case: Arc::new(MockGetTradeBindersUseCase::new()),
            add_trade_binder_use_case: Arc::new(MockAddTradeBinderUseCase::new()),
            remove_trade_binder_use_case: Arc::new(MockRemoveTradeBinderUseCase::new()),
            get_rarity_trade_filters_use_case: Arc::new(MockGetRarityTradeFiltersUseCase::new()),
            set_rarity_trade_filter_use_case: Arc::new(MockSetRarityTradeFilterUseCase::new()),
            list_sets_use_case: Arc::new(MockListSetsUseCase::new()),
            get_set_use_case: Arc::new(MockGetSetUseCase::new()),
        }
    }

    pub fn for_testing_with_enqueue_cardmarket_id(
        stats_use_case: Arc<dyn StatsUseCase>,
        enqueue_cardmarket_id_use_case: Arc<dyn EnqueueCardMarketIdUpdateUseCase>,
    ) -> Self {
        use crate::application::use_case::MockImportPriceUseCase;
        let mut mock_import_price = MockImportPriceUseCase::new();
        mock_import_price
            .expect_import_prices_for_current_date()
            .returning(|| Box::pin(async { Ok(()) }));
        let mut base =
            Self::for_testing_with_import_price(stats_use_case, Arc::new(mock_import_price));
        base.enqueue_cardmarket_id_use_case = enqueue_cardmarket_id_use_case;
        base
    }

    pub fn for_testing_with_create_trade(
        stats_use_case: Arc<dyn StatsUseCase>,
        create_trade_use_case: Arc<dyn CreateTradeUseCase>,
    ) -> Self {
        use crate::application::use_case::MockImportPriceUseCase;
        let mut mock_import_price = MockImportPriceUseCase::new();
        mock_import_price
            .expect_import_prices_for_current_date()
            .returning(|| Box::pin(async { Ok(()) }));
        let mut base =
            Self::for_testing_with_import_price(stats_use_case, Arc::new(mock_import_price));
        base.create_trade_use_case = create_trade_use_case;
        base
    }

    pub fn for_testing_with_accept_trade(
        stats_use_case: Arc<dyn StatsUseCase>,
        accept_trade_use_case: Arc<dyn AcceptTradeUseCase>,
    ) -> Self {
        use crate::application::use_case::MockImportPriceUseCase;
        let mut mock_import_price = MockImportPriceUseCase::new();
        mock_import_price
            .expect_import_prices_for_current_date()
            .returning(|| Box::pin(async { Ok(()) }));
        let mut base =
            Self::for_testing_with_import_price(stats_use_case, Arc::new(mock_import_price));
        base.accept_trade_use_case = accept_trade_use_case;
        base
    }

    pub fn for_testing_with_abandon_trade(
        stats_use_case: Arc<dyn StatsUseCase>,
        abandon_trade_use_case: Arc<dyn AbandonTradeUseCase>,
    ) -> Self {
        use crate::application::use_case::MockImportPriceUseCase;
        let mut mock_import_price = MockImportPriceUseCase::new();
        mock_import_price
            .expect_import_prices_for_current_date()
            .returning(|| Box::pin(async { Ok(()) }));
        let mut base =
            Self::for_testing_with_import_price(stats_use_case, Arc::new(mock_import_price));
        base.abandon_trade_use_case = abandon_trade_use_case;
        base
    }

    pub fn for_testing_with_confirm_trade(
        stats_use_case: Arc<dyn StatsUseCase>,
        confirm_trade_use_case: Arc<dyn ConfirmTradeUseCase>,
    ) -> Self {
        use crate::application::use_case::MockImportPriceUseCase;
        let mut mock_import_price = MockImportPriceUseCase::new();
        mock_import_price
            .expect_import_prices_for_current_date()
            .returning(|| Box::pin(async { Ok(()) }));
        let mut base =
            Self::for_testing_with_import_price(stats_use_case, Arc::new(mock_import_price));
        base.confirm_trade_use_case = confirm_trade_use_case;
        base
    }

    pub fn for_testing_with_rate_trade(
        stats_use_case: Arc<dyn StatsUseCase>,
        rate_trade_use_case: Arc<dyn RateTradeUseCase>,
    ) -> Self {
        use crate::application::use_case::MockImportPriceUseCase;
        let mut mock_import_price = MockImportPriceUseCase::new();
        mock_import_price
            .expect_import_prices_for_current_date()
            .returning(|| Box::pin(async { Ok(()) }));
        let mut base =
            Self::for_testing_with_import_price(stats_use_case, Arc::new(mock_import_price));
        base.rate_trade_use_case = rate_trade_use_case;
        base
    }

    pub fn for_testing_with_get_trade(
        stats_use_case: Arc<dyn StatsUseCase>,
        get_trade_use_case: Arc<dyn GetTradeUseCase>,
    ) -> Self {
        use crate::application::use_case::MockImportPriceUseCase;
        let mut mock_import_price = MockImportPriceUseCase::new();
        mock_import_price
            .expect_import_prices_for_current_date()
            .returning(|| Box::pin(async { Ok(()) }));
        let mut base =
            Self::for_testing_with_import_price(stats_use_case, Arc::new(mock_import_price));
        base.get_trade_use_case = get_trade_use_case;
        base
    }

    pub fn for_testing_with_list_trades(
        stats_use_case: Arc<dyn StatsUseCase>,
        list_trades_use_case: Arc<dyn ListTradesUseCase>,
    ) -> Self {
        use crate::application::use_case::MockImportPriceUseCase;
        let mut mock_import_price = MockImportPriceUseCase::new();
        mock_import_price
            .expect_import_prices_for_current_date()
            .returning(|| Box::pin(async { Ok(()) }));
        let mut base =
            Self::for_testing_with_import_price(stats_use_case, Arc::new(mock_import_price));
        base.list_trades_use_case = list_trades_use_case;
        base
    }

    pub fn for_testing_with_add_trade_card(
        stats_use_case: Arc<dyn StatsUseCase>,
        add_trade_card_use_case: Arc<dyn AddTradeCardUseCase>,
    ) -> Self {
        use crate::application::use_case::MockImportPriceUseCase;
        let mut mock_import_price = MockImportPriceUseCase::new();
        mock_import_price
            .expect_import_prices_for_current_date()
            .returning(|| Box::pin(async { Ok(()) }));
        let mut base =
            Self::for_testing_with_import_price(stats_use_case, Arc::new(mock_import_price));
        base.add_trade_card_use_case = add_trade_card_use_case;
        base
    }

    pub fn for_testing_with_remove_trade_card(
        stats_use_case: Arc<dyn StatsUseCase>,
        remove_trade_card_use_case: Arc<dyn RemoveTradeCardUseCase>,
    ) -> Self {
        use crate::application::use_case::MockImportPriceUseCase;
        let mut mock_import_price = MockImportPriceUseCase::new();
        mock_import_price
            .expect_import_prices_for_current_date()
            .returning(|| Box::pin(async { Ok(()) }));
        let mut base =
            Self::for_testing_with_import_price(stats_use_case, Arc::new(mock_import_price));
        base.remove_trade_card_use_case = remove_trade_card_use_case;
        base
    }

    pub fn for_testing_with_card_offers(
        stats_use_case: Arc<dyn StatsUseCase>,
        get_card_offers_use_case: Arc<dyn GetCardOffersUseCase>,
    ) -> Self {
        use crate::application::use_case::MockImportPriceUseCase;
        let mut mock_import_price = MockImportPriceUseCase::new();
        mock_import_price
            .expect_import_prices_for_current_date()
            .returning(|| Box::pin(async { Ok(()) }));
        let mut base =
            Self::for_testing_with_import_price(stats_use_case, Arc::new(mock_import_price));
        base.get_card_offers_use_case = get_card_offers_use_case;
        base
    }

    pub fn for_testing_with_enqueue_gatherer_id(
        stats_use_case: Arc<dyn StatsUseCase>,
        enqueue_gatherer_id_use_case: Arc<dyn EnqueueGathererIdUpdateUseCase>,
    ) -> Self {
        use crate::application::use_case::MockImportPriceUseCase;
        let mut mock_import_price = MockImportPriceUseCase::new();
        mock_import_price
            .expect_import_prices_for_current_date()
            .returning(|| Box::pin(async { Ok(()) }));
        let mut base =
            Self::for_testing_with_import_price(stats_use_case, Arc::new(mock_import_price));
        base.enqueue_gatherer_id_use_case = enqueue_gatherer_id_use_case;
        base
    }
}
