use crate::application::caller::EdhRecCaller;
use crate::application::card_import_job::CardImportJob;
use crate::application::card_import_worker::CardImportWorker;
use crate::application::card_prices_refresher::CardPricesRefresher;
use crate::application::enrichment_queue::EnrichmentQueue;
use crate::application::error::{AppError, InfraError};
use crate::application::repository::{CardImportRepository, CardPricesRefreshRequester};
use crate::application::service::autocomplete::users::AutocompleteUserService;
use crate::application::service::binder::create::CreateBinderService;
use crate::application::service::binder::delete::DeleteBinderService;
use crate::application::service::binder::list::ListBindersService;
use crate::application::service::binder::rename::RenameBinderService;
use crate::application::service::card::get_offers::CardOfferService;
use crate::application::service::card::get_price_history::CardPriceHistoryService;
use crate::application::service::catalog::create_card::CreateCardService;
use crate::application::service::catalog::scan_index::ScanIndexService;
use crate::application::service::catalog::search::SearchCatalogService;
use crate::application::service::catalog::trend_prices::TrendPricesService;
use crate::application::service::collection::add_entry::AddCollectionEntryService;
use crate::application::service::collection::attach_card_to_sealed_product::AttachCardToSealedProductService;
use crate::application::service::collection::get::CollectionService;
use crate::application::service::collection::get_card_import::CardImportQueryService;
use crate::application::service::collection::get_entries::GetCollectionEntriesService;
use crate::application::service::collection::get_price_history::CollectionPriceHistoryService;
use crate::application::service::collection::get_stats::CollectionStatsService;
use crate::application::service::collection::import_card::ImportCardService;
use crate::application::service::collection::move_entry::MoveCollectionEntryService;
use crate::application::service::collection::remove_entry::RemoveCollectionEntryService;
use crate::application::service::collection::run_card_import::RunCardImportService;
use crate::application::service::collection::update_entry::UpdateCollectionEntryService;
use crate::application::service::maintenance::card_collection_price_calculation::CardCollectionService;
use crate::application::service::maintenance::enqueue_card_image::CardImageEnricher;
use crate::application::service::maintenance::enqueue_cardmarket_id::CardMarketIdEnricher;
use crate::application::service::maintenance::import_price::ImportPriceService;
use crate::application::service::maintenance::update_catalog::UpdateCatalogService;
use crate::application::service::purchase::create_sealed_product::CreateSealedProductService;
use crate::application::service::purchase::delete::DeletePurchaseService;
use crate::application::service::purchase::get::GetPurchaseService;
use crate::application::service::purchase::list::ListPurchasesService;
use crate::application::service::purchase::update::UpdatePurchaseService;
use crate::application::service::search::search_cards::SearchCardsService;
use crate::application::service::search::search_sets::SearchSetsService;
use crate::application::service::sets::get::GetSetService;
use crate::application::service::sets::list::ListSetsService;
use crate::application::service::showcase::get::ShowcaseService;
use crate::application::service::stats::get::StatsService;
use crate::application::service::trade::abandon::AbandonTradeService;
use crate::application::service::trade::accept::AcceptTradeService;
use crate::application::service::trade::add_card::AddTradeCardService;
use crate::application::service::trade::confirm::ConfirmTradeService;
use crate::application::service::trade::create::CreateTradeService;
use crate::application::service::trade::get::GetTradeService;
use crate::application::service::trade::list::ListTradesService;
use crate::application::service::trade::rate::RateTradeService;
use crate::application::service::trade::remove_card::RemoveTradeCardService;
use crate::application::service::trade_settings::add_binder::AddTradeBinderService;
use crate::application::service::trade_settings::get_binders::GetTradeBindersService;
use crate::application::service::trade_settings::get_rarity_filters::GetRarityTradeFiltersService;
use crate::application::service::trade_settings::get_receiving_binder::GetReceivingBinderService;
use crate::application::service::trade_settings::get_visibility::GetCollectionVisibilityService;
use crate::application::service::trade_settings::remove_binder::RemoveTradeBinderService;
use crate::application::service::trade_settings::set_rarity_filter::SetRarityTradeFilterService;
use crate::application::service::trade_settings::set_receiving_binder::SetReceivingBinderService;
use crate::application::service::trade_settings::set_visibility::SetCollectionVisibilityService;
use crate::application::service::user::get_profile::GetUserProfileService;
use crate::application::service::user::register::RegisterUserService;
use crate::application::token_validator::TokenValidator;
use crate::application::use_case::{
    AbandonTradeUseCase, AcceptTradeUseCase, AddCollectionEntryUseCase, AddTradeBinderUseCase,
    AddTradeCardUseCase, AutocompleteUsersUseCase, ConfirmTradeUseCase, CreateBinderUseCase,
    CreateCardUseCase, CreateTradeUseCase, DeleteBinderUseCase, EnqueueCardImageUpdateUseCase,
    EnqueueCardMarketIdUpdateUseCase, GetCardImportUseCase, GetCardOffersUseCase,
    GetCardPriceHistoryUseCase, GetCollectionEntriesUseCase, GetCollectionPriceHistoryUseCase,
    GetCollectionStatsUseCase, GetCollectionUseCase, GetCollectionVisibilityUseCase,
    GetRarityTradeFiltersUseCase, GetReceivingBinderUseCase, GetSetUseCase, GetShowcaseUseCase,
    GetTradeBindersUseCase, GetTradeUseCase, GetUserProfileUseCase, ImportCardUseCase,
    ImportPriceUseCase, ListBindersUseCase, ListSetsUseCase, ListTradesUseCase,
    MoveCollectionEntryUseCase, RateTradeUseCase, RegisterUserUseCase,
    RemoveCollectionEntryUseCase, RemoveTradeBinderUseCase, RemoveTradeCardUseCase,
    RenameBinderUseCase, RunCardImportUseCase, ScanIndexUseCase, SearchCardsUseCase,
    SearchCatalogUseCase, SearchSetsUseCase, SetCollectionVisibilityUseCase,
    SetRarityTradeFilterUseCase, SetReceivingBinderUseCase, StatsUseCase, TrendPricesUseCase,
    UpdateCatalogUseCase, UpdateCollectionEntryUseCase,
};
use crate::application::use_case::{
    AttachCardToSealedProductUseCase, CreateSealedProductUseCase, DeletePurchaseUseCase,
    GetPurchaseUseCase, ListPurchasesUseCase, UpdatePurchaseUseCase,
};
use crate::config::Config;
use crate::infrastructure::adapter_in::autocomplete::controller::create_autocomplete_router;
use crate::infrastructure::adapter_in::binder::controller::create_binder_router;
use crate::infrastructure::adapter_in::card::controller::create_card_router;
use crate::infrastructure::adapter_in::catalog::controller::create_catalog_router;
use crate::infrastructure::adapter_in::collection::controller::create_collection_router;
use crate::infrastructure::adapter_in::purchase::controller::create_purchase_router;
use crate::infrastructure::adapter_in::search::controller::create_search_router;
use crate::infrastructure::adapter_in::sets::controller::create_set_router;
use crate::infrastructure::adapter_in::trade::controller::create_trade_router;
use crate::infrastructure::adapter_in::trade_settings::controller::create_trade_settings_router;
use crate::infrastructure::adapter_in::user::controller::create_user_router;
use crate::infrastructure::adapter_in::with_error_body;
use crate::infrastructure::adapter_out::caller::cardmarket_caller_adapter::CardMarketCallerAdapter;
use crate::infrastructure::adapter_out::caller::edhrec_caller_adapter::EdhRecCallerAdapter;
use crate::infrastructure::adapter_out::clerk_token_validator_adapter::ClerkTokenValidatorAdapter;
use crate::infrastructure::adapter_out::repository::binder_repository_adapter::BinderRepositoryAdapter;
use crate::infrastructure::adapter_out::repository::card_prices_view_repository_adapter::CardPricesViewRepositoryAdapter;
use crate::infrastructure::adapter_out::repository::cardmarket_price_repository_adapter::CardMarketPriceRepositoryAdapter;
use crate::infrastructure::adapter_out::repository::collection_entry_repository_adapter::CollectionEntryRepositoryAdapter;
use crate::infrastructure::adapter_out::repository::collection_price_history_repository_adapter::CollectionPriceHistoryRepositoryAdapter;
use crate::infrastructure::adapter_out::repository::collection_rarity_filters_repository_adapter::CollectionRarityFiltersRepositoryAdapter;
use crate::infrastructure::adapter_out::repository::collection_stats_repository_adapter::CollectionStatsRepositoryAdapter;
use crate::infrastructure::adapter_out::repository::purchase_repository_adapter::PurchaseRepositoryAdapter;
use crate::infrastructure::adapter_out::repository::receiving_binder_repository_adapter::ReceivingBinderRepositoryAdapter;
use crate::infrastructure::adapter_out::repository::showcase_repository_adapter::ShowcaseRepositoryAdapter;
use crate::infrastructure::adapter_out::repository::stats_repository_adapter::StatsRepositoryAdapter;
use crate::infrastructure::adapter_out::repository::trade_repository_adapter::TradeRepositoryAdapter;
use crate::infrastructure::adapter_out::repository::trading_binders_repository_adapter::TradingBindersRepositoryAdapter;
use adapter_in::maintenance::controller::create_maintenance_router;
use adapter_in::showcase::controller::create_showcase_router;
use adapter_in::stats::controller::create_stats_router;
use adapter_out::caller::gatherer_caller_adapter::GathererCallerAdapter;
use adapter_out::caller::scryfall_caller_adapter::ScryfallCallerAdapter;
use adapter_out::repository::card_image_file_repository_adapter::CardImageFileRepositoryAdapter;
use adapter_out::repository::card_import_repository_adapter::CardImportRepositoryAdapter;
use adapter_out::repository::card_repository_adapter::CardRepositoryAdapter;
use adapter_out::repository::catalog_repository_adapter::CatalogRepositoryAdapter;
use adapter_out::repository::set_names_repository_adapter::SetNameRepositoryAdapter;
use adapter_out::repository::user_repository_adapter::UserRepositoryAdapter;
use axum::Router;
use axum::body::Body;
use axum::http::Request;
use chrono::Utc;
use cron_tab::AsyncCron;
use sentry::integrations::tower::{NewSentryLayer, SentryHttpLayer};
use sqlx::{Pool, Postgres};
use std::sync::Arc;

pub mod adapter_in;
pub mod adapter_out;
#[cfg(test)]
mod testing;

// ---- AppState ----
#[derive(Clone)]
pub struct AppState {
    pub import_card_use_case: Arc<dyn ImportCardUseCase>,
    pub edh_rec_caller_adapter: Arc<dyn EdhRecCaller>,
    pub stats_use_case: Arc<dyn StatsUseCase>,
    pub get_showcase_use_case: Arc<dyn GetShowcaseUseCase>,
    pub token_validator: Arc<dyn TokenValidator>,
    pub get_collection_use_case: Arc<dyn GetCollectionUseCase>,
    pub search_cards_use_case: Arc<dyn SearchCardsUseCase>,
    pub search_sets_use_case: Arc<dyn SearchSetsUseCase>,
    pub import_price_use_case: Arc<dyn ImportPriceUseCase>,
    pub enqueue_cardmarket_id_use_case: Arc<dyn EnqueueCardMarketIdUpdateUseCase>,
    pub enqueue_card_image_use_case: Arc<dyn EnqueueCardImageUpdateUseCase>,
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
    pub get_collection_entries_use_case: Arc<dyn GetCollectionEntriesUseCase>,
    pub add_collection_entry_use_case: Arc<dyn AddCollectionEntryUseCase>,
    pub update_collection_entry_use_case: Arc<dyn UpdateCollectionEntryUseCase>,
    pub move_collection_entry_use_case: Arc<dyn MoveCollectionEntryUseCase>,
    pub remove_collection_entry_use_case: Arc<dyn RemoveCollectionEntryUseCase>,
    pub list_binders_use_case: Arc<dyn ListBindersUseCase>,
    pub create_binder_use_case: Arc<dyn CreateBinderUseCase>,
    pub rename_binder_use_case: Arc<dyn RenameBinderUseCase>,
    pub delete_binder_use_case: Arc<dyn DeleteBinderUseCase>,
    pub list_purchases_use_case: Arc<dyn ListPurchasesUseCase>,
    pub get_purchase_use_case: Arc<dyn GetPurchaseUseCase>,
    pub create_sealed_product_use_case: Arc<dyn CreateSealedProductUseCase>,
    pub update_purchase_use_case: Arc<dyn UpdatePurchaseUseCase>,
    pub delete_purchase_use_case: Arc<dyn DeletePurchaseUseCase>,
    pub get_receiving_binder_use_case: Arc<dyn GetReceivingBinderUseCase>,
    pub set_receiving_binder_use_case: Arc<dyn SetReceivingBinderUseCase>,
    pub get_trade_binders_use_case: Arc<dyn GetTradeBindersUseCase>,
    pub add_trade_binder_use_case: Arc<dyn AddTradeBinderUseCase>,
    pub remove_trade_binder_use_case: Arc<dyn RemoveTradeBinderUseCase>,
    pub get_rarity_trade_filters_use_case: Arc<dyn GetRarityTradeFiltersUseCase>,
    pub set_rarity_trade_filter_use_case: Arc<dyn SetRarityTradeFilterUseCase>,
    pub list_sets_use_case: Arc<dyn ListSetsUseCase>,
    pub get_set_use_case: Arc<dyn GetSetUseCase>,
    pub card_import_query_use_case: Arc<dyn GetCardImportUseCase>,
    pub update_catalog_use_case: Arc<dyn UpdateCatalogUseCase>,
    pub scan_index_use_case: Arc<dyn ScanIndexUseCase>,
    pub search_catalog_use_case: Arc<dyn SearchCatalogUseCase>,
    pub trend_prices_use_case: Arc<dyn TrendPricesUseCase>,
}

// ---- Repositories ----
struct Repositories {
    card: Arc<CardRepositoryAdapter>,
    set_name: Arc<SetNameRepositoryAdapter>,
    card_market: Arc<CardMarketPriceRepositoryAdapter>,
    card_prices_view: Arc<CardPricesViewRepositoryAdapter>,
    stats: Arc<StatsRepositoryAdapter>,
    showcase: Arc<ShowcaseRepositoryAdapter>,
    user: Arc<UserRepositoryAdapter>,
    trade: Arc<TradeRepositoryAdapter>,
    collection_price_history: Arc<CollectionPriceHistoryRepositoryAdapter>,
    collection_stats: Arc<CollectionStatsRepositoryAdapter>,
    trading_binders: Arc<TradingBindersRepositoryAdapter>,
    receiving_binder: Arc<ReceivingBinderRepositoryAdapter>,
    binder: Arc<BinderRepositoryAdapter>,
    purchase: Arc<PurchaseRepositoryAdapter>,
    collection_entry: Arc<CollectionEntryRepositoryAdapter>,
    collection_rarity_filters: Arc<CollectionRarityFiltersRepositoryAdapter>,
    card_import: Arc<CardImportRepositoryAdapter>,
    catalog: Arc<CatalogRepositoryAdapter>,
}

fn create_repositories(pool: &Pool<Postgres>) -> Repositories {
    Repositories {
        card: Arc::new(CardRepositoryAdapter::new(pool.clone())),
        set_name: Arc::new(SetNameRepositoryAdapter::new(pool.clone())),
        card_market: Arc::new(CardMarketPriceRepositoryAdapter::new(pool.clone())),
        card_prices_view: Arc::new(CardPricesViewRepositoryAdapter::new(pool.clone())),
        stats: Arc::new(StatsRepositoryAdapter::new(pool.clone())),
        showcase: Arc::new(ShowcaseRepositoryAdapter::new(pool.clone())),
        user: Arc::new(UserRepositoryAdapter::new(pool.clone())),
        trade: Arc::new(TradeRepositoryAdapter::new(pool.clone())),
        collection_price_history: Arc::new(CollectionPriceHistoryRepositoryAdapter::new(
            pool.clone(),
        )),
        collection_stats: Arc::new(CollectionStatsRepositoryAdapter::new(pool.clone())),
        trading_binders: Arc::new(TradingBindersRepositoryAdapter::new(pool.clone())),
        receiving_binder: Arc::new(ReceivingBinderRepositoryAdapter::new(pool.clone())),
        binder: Arc::new(BinderRepositoryAdapter::new(pool.clone())),
        purchase: Arc::new(PurchaseRepositoryAdapter::new(pool.clone())),
        collection_entry: Arc::new(CollectionEntryRepositoryAdapter::new(pool.clone())),
        collection_rarity_filters: Arc::new(CollectionRarityFiltersRepositoryAdapter::new(
            pool.clone(),
        )),
        card_import: Arc::new(CardImportRepositoryAdapter::new(pool.clone())),
        catalog: Arc::new(CatalogRepositoryAdapter::new(pool.clone())),
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

async fn create_token_validator(config: &Config) -> Arc<dyn TokenValidator> {
    Arc::new(
        ClerkTokenValidatorAdapter::new(config.clerk_frontend_api_url.clone(), None)
            .await
            .expect("Failed to initialize Clerk token validator"),
    )
}

// ---- Background workers ----
// Canal non borné consommé en série par un unique worker : au plus un import actif par
// utilisateur (contrainte portée par la base), donc pas de besoin de parallélisme ici.
fn spawn_card_import_worker(
    repos: &Repositories,
    enqueue_cardmarket_id_use_case: Arc<dyn EnqueueCardMarketIdUpdateUseCase>,
    enqueue_card_image_use_case: Arc<dyn EnqueueCardImageUpdateUseCase>,
) -> tokio::sync::mpsc::UnboundedSender<CardImportJob> {
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<CardImportJob>();

    let run_use_case: Arc<dyn RunCardImportUseCase> = Arc::new(RunCardImportService::new(
        repos.card.clone(),
        repos.set_name.clone(),
        repos.card_import.clone(),
        enqueue_cardmarket_id_use_case,
        enqueue_card_image_use_case,
        repos.card_prices_view.clone(),
        repos.trading_binders.clone(),
    ));

    let worker = CardImportWorker::new(run_use_case);
    tokio::spawn(async move {
        worker.run(receiver).await;
    });

    sender
}

// ---- App state assembly ----
#[allow(clippy::too_many_arguments)]
fn create_app_state(
    repos: Repositories,
    callers: Callers,
    token_validator: Arc<dyn TokenValidator>,
    card_collection_service: Arc<CardCollectionService>,
    enqueue_cardmarket_id_use_case: Arc<dyn EnqueueCardMarketIdUpdateUseCase>,
    enqueue_card_image_use_case: Arc<dyn EnqueueCardImageUpdateUseCase>,
    card_import_sender: tokio::sync::mpsc::UnboundedSender<CardImportJob>,
    card_prices_refresh: Arc<dyn CardPricesRefreshRequester>,
) -> AppState {
    let import_card_service = Arc::new(ImportCardService::new(
        repos.card_import.clone(),
        card_import_sender,
    ));
    let card_import_query_service: Arc<dyn GetCardImportUseCase> =
        Arc::new(CardImportQueryService::new(repos.card_import.clone()));

    let import_price_use_case: Arc<dyn ImportPriceUseCase> = Arc::new(ImportPriceService::new(
        callers.card_market,
        repos.card_market.clone(),
        repos.card_prices_view.clone(),
        card_collection_service.clone(),
    ));

    let stats_service = Arc::new(StatsService::new(repos.stats));
    let showcase_service = Arc::new(ShowcaseService::new(repos.showcase));
    let collection_service = Arc::new(CollectionService::new(repos.card_prices_view.clone()));
    let search_cards_service = Arc::new(SearchCardsService::new(repos.card_prices_view.clone()));
    let search_sets_service = Arc::new(SearchSetsService::new(repos.card_prices_view.clone()));
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
    let get_collection_entries_service: Arc<dyn GetCollectionEntriesUseCase> = Arc::new(
        GetCollectionEntriesService::new(repos.collection_entry.clone()),
    );
    let create_card_service: Arc<dyn CreateCardUseCase> = Arc::new(CreateCardService::new(
        repos.card_prices_view.clone(),
        repos.catalog.clone(),
        repos.card.clone(),
        enqueue_cardmarket_id_use_case.clone(),
        enqueue_card_image_use_case.clone(),
    ));
    let attach_card_to_sealed_product_service: Arc<dyn AttachCardToSealedProductUseCase> =
        Arc::new(AttachCardToSealedProductService::new(
            create_card_service.clone(),
            repos.purchase.clone(),
            card_prices_refresh.clone(),
        ));
    let add_collection_entry_service: Arc<dyn AddCollectionEntryUseCase> =
        Arc::new(AddCollectionEntryService::new(
            create_card_service.clone(),
            repos.collection_entry.clone(),
            card_prices_refresh.clone(),
            attach_card_to_sealed_product_service,
        ));
    let update_collection_entry_service: Arc<dyn UpdateCollectionEntryUseCase> =
        Arc::new(UpdateCollectionEntryService::new(
            repos.collection_entry.clone(),
            card_prices_refresh.clone(),
        ));
    let move_collection_entry_service: Arc<dyn MoveCollectionEntryUseCase> =
        Arc::new(MoveCollectionEntryService::new(
            repos.collection_entry.clone(),
            card_prices_refresh.clone(),
        ));
    let remove_collection_entry_service: Arc<dyn RemoveCollectionEntryUseCase> =
        Arc::new(RemoveCollectionEntryService::new(
            repos.collection_entry.clone(),
            card_prices_refresh.clone(),
        ));
    let list_binders_service: Arc<dyn ListBindersUseCase> =
        Arc::new(ListBindersService::new(repos.binder.clone()));
    let create_binder_service: Arc<dyn CreateBinderUseCase> =
        Arc::new(CreateBinderService::new(repos.binder.clone()));
    let rename_binder_service: Arc<dyn RenameBinderUseCase> =
        Arc::new(RenameBinderService::new(repos.binder.clone()));
    let delete_binder_service: Arc<dyn DeleteBinderUseCase> =
        Arc::new(DeleteBinderService::new(repos.binder.clone()));
    let list_purchases_service: Arc<dyn ListPurchasesUseCase> =
        Arc::new(ListPurchasesService::new(repos.purchase.clone()));
    let get_purchase_service: Arc<dyn GetPurchaseUseCase> =
        Arc::new(GetPurchaseService::new(repos.purchase.clone()));
    let create_sealed_product_service: Arc<dyn CreateSealedProductUseCase> =
        Arc::new(CreateSealedProductService::new(repos.purchase.clone()));
    let update_purchase_service: Arc<dyn UpdatePurchaseUseCase> =
        Arc::new(UpdatePurchaseService::new(repos.purchase.clone()));
    let delete_purchase_service: Arc<dyn DeletePurchaseUseCase> =
        Arc::new(DeletePurchaseService::new(repos.purchase.clone()));
    let get_receiving_binder_service: Arc<dyn GetReceivingBinderUseCase> = Arc::new(
        GetReceivingBinderService::new(repos.receiving_binder.clone()),
    );
    let set_receiving_binder_service: Arc<dyn SetReceivingBinderUseCase> = Arc::new(
        SetReceivingBinderService::new(repos.receiving_binder, repos.binder),
    );
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
    let confirm_trade_service: Arc<dyn ConfirmTradeUseCase> = Arc::new(ConfirmTradeService::new(
        repos.trade.clone(),
        card_prices_refresh.clone(),
    ));
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
    let scan_index_service: Arc<dyn ScanIndexUseCase> =
        Arc::new(ScanIndexService::new(repos.catalog.clone()));
    let search_catalog_service: Arc<dyn SearchCatalogUseCase> =
        Arc::new(SearchCatalogService::new(repos.catalog.clone()));
    let trend_prices_service: Arc<dyn TrendPricesUseCase> =
        Arc::new(TrendPricesService::new(repos.catalog.clone()));
    let update_catalog_service: Arc<dyn UpdateCatalogUseCase> = Arc::new(
        UpdateCatalogService::new(callers.scryfall, repos.catalog, scan_index_service.clone()),
    );
    let list_sets_service = Arc::new(ListSetsService::new(repos.set_name.clone()));
    let get_set_service = Arc::new(GetSetService::new(repos.set_name));

    AppState {
        import_card_use_case: import_card_service,
        edh_rec_caller_adapter: callers.edh_rec,
        stats_use_case: stats_service,
        get_showcase_use_case: showcase_service,
        token_validator,
        get_collection_use_case: collection_service,
        search_cards_use_case: search_cards_service,
        search_sets_use_case: search_sets_service,
        import_price_use_case,
        enqueue_cardmarket_id_use_case,
        enqueue_card_image_use_case,
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
        get_collection_entries_use_case: get_collection_entries_service,
        add_collection_entry_use_case: add_collection_entry_service,
        update_collection_entry_use_case: update_collection_entry_service,
        move_collection_entry_use_case: move_collection_entry_service,
        remove_collection_entry_use_case: remove_collection_entry_service,
        list_binders_use_case: list_binders_service,
        create_binder_use_case: create_binder_service,
        rename_binder_use_case: rename_binder_service,
        delete_binder_use_case: delete_binder_service,
        list_purchases_use_case: list_purchases_service,
        get_purchase_use_case: get_purchase_service,
        create_sealed_product_use_case: create_sealed_product_service,
        update_purchase_use_case: update_purchase_service,
        delete_purchase_use_case: delete_purchase_service,
        get_receiving_binder_use_case: get_receiving_binder_service,
        set_receiving_binder_use_case: set_receiving_binder_service,
        get_trade_binders_use_case: get_trade_binders_service,
        add_trade_binder_use_case: add_trade_binder_service,
        remove_trade_binder_use_case: remove_trade_binder_service,
        get_rarity_trade_filters_use_case: get_rarity_trade_filters_service,
        set_rarity_trade_filter_use_case: set_rarity_trade_filter_service,
        list_sets_use_case: list_sets_service,
        get_set_use_case: get_set_service,
        card_import_query_use_case: card_import_query_service,
        update_catalog_use_case: update_catalog_service,
        scan_index_use_case: scan_index_service,
        search_catalog_use_case: search_catalog_service,
        trend_prices_use_case: trend_prices_service,
    }
}

/// Runs `job` on the cron `expression` (UTC) for as long as the process lives.
async fn schedule_job<F, Fut>(expression: &str, job: F)
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let mut cron = AsyncCron::new(Utc);
    cron.add_fn(expression, job).await.unwrap();
    cron.start().await;
}

async fn schedule_price_import_job(import_price_use_case: Arc<dyn ImportPriceUseCase>) {
    schedule_job("0 0 */12 * * *", move || {
        let service = import_price_use_case.clone();
        async move {
            if let Err(e) = service.import_prices_for_current_date().await {
                tracing::error!(error = %e, "failed to import prices");
            }
        }
    })
    .await;
}

async fn schedule_catalog_update_job(update_catalog_use_case: Arc<dyn UpdateCatalogUseCase>) {
    // Daily, off the hours of the price import: Scryfall refreshes its bulk files every day.
    schedule_job("0 0 4 * * *", move || {
        let service = update_catalog_use_case.clone();
        async move {
            if let Err(e) = service.update_catalog().await {
                tracing::error!(error = %e, "failed to update the catalog");
            }
        }
    })
    .await;
}

pub(crate) fn create_router(app_state: AppState) -> Router {
    Router::new()
        .nest("/autocomplete", create_autocomplete_router())
        .nest("/card", create_card_router())
        .nest("/catalog", create_catalog_router())
        .nest("/collection", create_collection_router())
        .nest("/collection/binders", create_binder_router())
        .nest("/purchases", create_purchase_router())
        .nest("/collection/trade-settings", create_trade_settings_router())
        .nest("/search", create_search_router())
        .nest("/sets", create_set_router())
        .nest("/showcase", create_showcase_router())
        .nest("/stats", create_stats_router())
        .nest("/maintenance", create_maintenance_router())
        .nest("/user", create_user_router())
        .nest("/trades", create_trade_router())
        .with_state(app_state)
        .layer(axum::middleware::map_response(with_error_body))
        .layer(NewSentryLayer::<Request<Body>>::new_from_top())
        .layer(SentryHttpLayer::new().enable_transaction())
}

pub async fn create_infra(pool: Pool<Postgres>, config: &Config) -> Router {
    let repos = create_repositories(&pool);
    let callers = create_callers(config);
    let token_validator = create_token_validator(config).await;

    let card_collection_service = Arc::new(CardCollectionService::new(
        repos.collection_price_history.clone(),
    ));

    // Any import left `pending`/`running` did not survive the previous process — fail it before
    // the worker (or the router) starts, so a fresh import for the same user is accepted.
    match repos
        .card_import
        .fail_all_active(&AppError::Infra(InfraError::QueueError(
            "server restarted during import".to_string(),
        )))
        .await
    {
        Ok(count) if count > 0 => {
            tracing::warn!(count, "failed stale card imports on startup");
        }
        Ok(_) => {}
        Err(e) => tracing::error!(error = %e, "failed to reset stale card imports on startup"),
    }

    let enqueue_cardmarket_id_use_case: Arc<dyn EnqueueCardMarketIdUpdateUseCase> =
        EnrichmentQueue::spawn(
            CardMarketIdEnricher::new(
                repos.card.clone(),
                callers.scryfall.clone(),
                card_collection_service.clone(),
            ),
            repos.card_prices_view.clone(),
        );
    let enqueue_card_image_use_case: Arc<dyn EnqueueCardImageUpdateUseCase> =
        EnrichmentQueue::spawn(
            CardImageEnricher::new(
                repos.card.clone(),
                Arc::new(CardImageFileRepositoryAdapter::new(
                    config.card_images_dir.clone(),
                )),
                callers.gatherer.clone(),
                callers.scryfall.clone(),
            ),
            repos.card_prices_view.clone(),
        );
    let card_import_sender = spawn_card_import_worker(
        &repos,
        enqueue_cardmarket_id_use_case.clone(),
        enqueue_card_image_use_case.clone(),
    );

    let card_prices_refresh: Arc<dyn CardPricesRefreshRequester> =
        CardPricesRefresher::spawn(repos.card_prices_view.clone());

    let app_state = create_app_state(
        repos,
        callers,
        token_validator,
        card_collection_service,
        enqueue_cardmarket_id_use_case,
        enqueue_card_image_use_case,
        card_import_sender,
        card_prices_refresh,
    );

    // The scan index lives in memory: rebuild it from the stored catalog without waiting for the cron.
    if let Err(e) = app_state.scan_index_use_case.refresh_scan_index().await {
        tracing::error!(error = %e, "failed to generate the scan index on startup");
    }

    schedule_price_import_job(app_state.import_price_use_case.clone()).await;
    schedule_catalog_update_job(app_state.update_catalog_use_case.clone()).await;

    create_router(app_state)
}
