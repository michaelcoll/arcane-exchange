use crate::application::caller::EdhRecCaller;
use crate::application::service::auth_service::AuthService;
use crate::application::service::card_collection_service::CardCollectionService;
use crate::application::service::card_offer_service::CardOfferService;
use crate::application::service::card_price_history_service::CardPriceHistoryService;
use crate::application::service::cardmarket_id_enqueue_service::CardMarketIdEnqueueService;
use crate::application::service::collection_price_history_service::CollectionPriceHistoryService;
use crate::application::service::collection_service::CollectionService;
use crate::application::service::collection_stats_service::CollectionStatsService;
use crate::application::service::gatherer_id_enqueue_service::GathererIdEnqueueService;
use crate::application::service::import_card_service::ImportCardService;
use crate::application::service::import_price_service::ImportPriceService;
use crate::application::service::register_user_service::RegisterUserService;
use crate::application::service::search_service::SearchService;
use crate::application::service::stats_service::StatsService;
use crate::application::service::trade_service::CreateTradeService;
use crate::application::service::update_card_market_service::CardMarketIdWorker;
use crate::application::service::update_gatherer_service::GathererIdWorker;
use crate::application::use_case::{
    CreateTradeUseCase, EnqueueCardMarketIdUpdateUseCase, EnqueueGathererIdUpdateUseCase,
    GetCardOffersUseCase, GetCardPriceHistoryUseCase, GetCollectionPriceHistoryUseCase,
    GetCollectionStatsUseCase, GetCollectionUseCase, ImportCardUseCase, ImportPriceUseCase,
    RegisterUserUseCase, SearchCardsUseCase, StatsUseCase,
};
use crate::config::Config;
use crate::domain::card::CardId;
use crate::infrastructure::adapter_in::card::controller::create_card_router;
use crate::infrastructure::adapter_in::collection::controller::create_collection_router;
use crate::infrastructure::adapter_in::search::controller::create_search_router;
use crate::infrastructure::adapter_in::trade::controller::create_trade_router;
use crate::infrastructure::adapter_in::user::controller::create_user_router;
use crate::infrastructure::adapter_out::caller::cardmarket_caller_adapter::CardMarketCallerAdapter;
use crate::infrastructure::adapter_out::caller::edhrec_caller_adapter::EdhRecCallerAdapter;
use crate::infrastructure::adapter_out::repository::card_prices_view_repository_adapter::CardPricesViewRepositoryAdapter;
use crate::infrastructure::adapter_out::repository::cardmarket_price_repository_adapter::CardMarketPriceRepositoryAdapter;
use crate::infrastructure::adapter_out::repository::collection_price_history_repository_adapter::CollectionPriceHistoryRepositoryAdapter;
use crate::infrastructure::adapter_out::repository::collection_stats_repository_adapter::CollectionStatsRepositoryAdapter;
use crate::infrastructure::adapter_out::repository::stats_repository_adapter::StatsRepositoryAdapter;
use crate::infrastructure::adapter_out::repository::trade_repository_adapter::TradeRepositoryAdapter;
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
    pub create_trade_use_case: Arc<dyn CreateTradeUseCase>,
    pub get_card_offers_use_case: Arc<dyn GetCardOffersUseCase>,
    pub max_page_size: u32,
    pub max_page_number: u32,
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
    config: &Config,
) -> AppState {
    let import_card_service = Arc::new(ImportCardService::new(
        repos.card.clone(),
        repos.set_name,
        enqueue_cardmarket_id_use_case.clone(),
        enqueue_gatherer_id_use_case.clone(),
        repos.card_prices_view.clone(),
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
        Arc::new(RegisterUserService::new(repos.user));
    let create_trade_service: Arc<dyn CreateTradeUseCase> =
        Arc::new(CreateTradeService::new(repos.trade));
    let card_offer_service: Arc<dyn GetCardOffersUseCase> =
        Arc::new(CardOfferService::new(repos.card_prices_view));

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
        create_trade_use_case: create_trade_service,
        get_card_offers_use_case: card_offer_service,
        max_page_size: config.max_page_size,
        max_page_number: config.max_page_number,
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
        .nest("/card", create_card_router())
        .nest("/collection", create_collection_router())
        .nest("/search", create_search_router())
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
        config,
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
            MockCreateTradeUseCase, MockEnqueueCardMarketIdUpdateUseCase,
            MockEnqueueGathererIdUpdateUseCase, MockGetCardOffersUseCase,
            MockGetCardPriceHistoryUseCase, MockGetCollectionPriceHistoryUseCase,
            MockGetCollectionStatsUseCase, MockGetCollectionUseCase, MockImportCardUseCase,
            MockRegisterUserUseCase, MockSearchCardsUseCase,
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
            .returning(|_| Ok(User::new("test-user-id".to_string(), None, None)));

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
            create_trade_use_case: Arc::new(MockCreateTradeUseCase::new()),
            get_card_offers_use_case: Arc::new(MockGetCardOffersUseCase::new()),
            max_page_size: 100,
            max_page_number: 10,
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
