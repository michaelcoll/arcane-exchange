use crate::application::error::AppError;
use async_trait::async_trait;

use crate::application::card_import_job::CardImportJob;
use crate::domain::card::{Card, CardId, CollectionEntry, CopyId};
use crate::domain::card_import::{CardImport, CardImportId};
use crate::domain::card_offer::CardOfferSortField;
use crate::domain::catalog::ScanIndex;
use crate::domain::collection::{CollectionQuery, SearchQuery, SearchScope};
use crate::domain::collection_stats::CollectionStats;
use crate::domain::pagination::{PageRequest, Paginated};
use crate::domain::price::PriceHistoryEntry;
use crate::domain::rarity_trade_filter::{RarityTradeFilter, RarityTradeFilterRule};
use crate::domain::set_name::{SetCode, SetName};
use crate::domain::showcase::ShowcaseCard;
use crate::domain::stats::Stats;
use crate::domain::trade::{TradeDetail, TradeId, TradeListQuery, TradeSummary};
use crate::domain::user::{CollectionVisibility, User, UserId, UserSuggestion};
#[cfg(test)]
use mockall::automock;
use std::sync::Arc;

#[async_trait]
#[cfg_attr(test, automock)]
pub trait ImportCardUseCase: Send + Sync {
    /// Parses `csv`, creates the `CardImport` at `Pending` and hands the parsed cards off to the
    /// background worker. Returns the import's id. A parsing error (`400`) or an already-running
    /// import for this user (`ImportAlreadyRunning`, `409`) is returned before any effect.
    async fn start_import(&self, csv: &str, user: User) -> Result<CardImportId, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait RunCardImportUseCase: Send + Sync {
    /// Executes one import job: writes the cards in batches, updating progress as it goes, then
    /// marks the import `Completed` or `Failed`. Called by the import worker, never directly by
    /// an HTTP handler.
    async fn run(&self, job: CardImportJob) -> Result<(), AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait GetCardImportUseCase: Send + Sync {
    /// Returns `FunctionalError::ImportNotFound` both when the import does not exist and when it
    /// belongs to another user, so as not to leak whether it exists.
    async fn find(&self, id: &CardImportId, user: &User) -> Result<CardImport, AppError>;

    /// The caller's imports, most recent first.
    async fn list(&self, user: &User) -> Result<Vec<CardImport>, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait RegisterUserUseCase: Send + Sync {
    async fn register_user(&self, user: &User) -> Result<(), AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait GetUserProfileUseCase: Send + Sync {
    async fn get_user_profile(&self, username: &str) -> Result<User, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait GetCollectionVisibilityUseCase: Send + Sync {
    async fn get_visibility(&self, user_id: UserId) -> Result<CollectionVisibility, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait SetCollectionVisibilityUseCase: Send + Sync {
    async fn set_visibility(
        &self,
        user_id: UserId,
        visibility: CollectionVisibility,
    ) -> Result<(), AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait GetTradeBindersUseCase: Send + Sync {
    async fn get_trade_binders(&self, user_id: UserId) -> Result<Vec<String>, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait AddTradeBinderUseCase: Send + Sync {
    async fn add_trade_binder(&self, user_id: UserId, binder_name: String) -> Result<(), AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait RemoveTradeBinderUseCase: Send + Sync {
    async fn remove_trade_binder(
        &self,
        user_id: UserId,
        binder_name: String,
    ) -> Result<(), AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait GetRarityTradeFiltersUseCase: Send + Sync {
    async fn get_rarity_trade_filters(
        &self,
        user_id: UserId,
    ) -> Result<Vec<RarityTradeFilter>, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait SetRarityTradeFilterUseCase: Send + Sync {
    async fn set_rarity_trade_filter(
        &self,
        user_id: UserId,
        rule: RarityTradeFilterRule,
    ) -> Result<(), AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait EnqueueCardMarketIdUpdateUseCase: Send + Sync {
    async fn enqueue_pending_updates(&self) -> Result<usize, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait EnqueueCardImageUpdateUseCase: Send + Sync {
    /// Queues the cards whose images are still pending.
    async fn enqueue_pending_updates(&self) -> Result<usize, AppError>;
    /// Queues again the cards in fallback, and the pending ones (maintenance).
    async fn enqueue_fallback_and_pending_updates(&self) -> Result<usize, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait CardCollectionPriceCalculationUseCase: Send + Sync {
    async fn calculate_total_price(&self) -> Result<(), AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait ImportPriceUseCase: Send + Sync {
    async fn import_prices_for_current_date(&self) -> Result<(), AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait UpdateCatalogUseCase: Send + Sync {
    /// Reloads the catalog from the Scryfall bulk. A failure leaves the previous catalog in place.
    /// A call made while a load is running is ignored (and logged): there is one load at a time.
    async fn update_catalog(&self) -> Result<(), AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait CreateCardUseCase: Send + Sync {
    /// Makes sure the card exists, without any import, and returns its id (the collector number
    /// without leading zeros). Idempotent: an existing card is returned untouched, even when the
    /// catalog lacks it. The language is taken as read, not checked against Scryfall. A newly
    /// created card enters the enrichment queues (Cardmarket id when the catalog lacks it,
    /// images). `FunctionalError::CardNotInCatalog` when the card does not exist and no catalog
    /// card has this set and number.
    async fn create_card(&self, card_id: CardId) -> Result<CardId, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait ScanIndexUseCase: Send + Sync {
    /// Regenerates the scan index from the catalog and keeps it in memory. With no catalog
    /// loaded, there is no index. A failure keeps the previous index.
    async fn refresh_scan_index(&self) -> Result<(), AppError>;

    /// The scan index, or `ScanIndexUnavailable` while no catalog is loaded.
    async fn get_scan_index(&self) -> Result<Arc<ScanIndex>, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait StatsUseCase: Send + Sync {
    async fn get_stats(&self) -> Result<Stats, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait GetShowcaseUseCase: Send + Sync {
    /// The showcase (« Vitrine »): the most expensive cards held in the platform's collections,
    /// most expensive first, whatever the visibility of the collection they belong to.
    async fn get_showcase(&self) -> Result<Vec<ShowcaseCard>, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait GetCollectionUseCase: Send + Sync {
    /// Rejects the requested page with `InvalidPageSize` or `PaginationTooDeep` (`400`) when it
    /// is out of the collection's bounds.
    async fn get_collection(
        &self,
        user_id: &UserId,
        query: CollectionQuery<PageRequest>,
    ) -> Result<Paginated<Card>, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait SearchCardsUseCase: Send + Sync {
    /// Rejects the requested page with `InvalidPageSize` or `PaginationTooDeep` (`400`) when it
    /// is out of the search's bounds.
    async fn search_cards(
        &self,
        query: SearchQuery<PageRequest>,
    ) -> Result<Paginated<Card>, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait SearchSetsUseCase: Send + Sync {
    /// The sets a search over `scope` can return cards from, sorted by name, whatever its
    /// rarity, set or price filters.
    async fn search_sets(&self, scope: SearchScope) -> Result<Vec<SetName>, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait GetCollectionPriceHistoryUseCase: Send + Sync {
    async fn get_collection_price_history(
        &self,
        user_id: &UserId,
        start_date: Option<chrono::NaiveDate>,
        end_date: Option<chrono::NaiveDate>,
    ) -> Result<Vec<PriceHistoryEntry>, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait GetCardPriceHistoryUseCase: Send + Sync {
    async fn get_card_price_history(
        &self,
        scryfall_id: uuid::Uuid,
        foil: bool,
        start_date: Option<chrono::NaiveDate>,
        end_date: Option<chrono::NaiveDate>,
    ) -> Result<Vec<PriceHistoryEntry>, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait GetCollectionStatsUseCase: Send + Sync {
    async fn get_collection_stats(&self, user_id: &UserId) -> Result<CollectionStats, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait GetCardOffersUseCase: Send + Sync {
    /// Rejects the requested page with `InvalidPageSize` or `PaginationTooDeep` (`400`) when it
    /// is out of the offers' bounds, before looking the card up.
    async fn get_card_offers(
        &self,
        user_id: &UserId,
        copy_id: CopyId,
        sort_by: CardOfferSortField,
        page_request: PageRequest,
    ) -> Result<Paginated<CollectionEntry>, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait CreateTradeUseCase: Send + Sync {
    async fn create_trade(
        &self,
        initiator_user_id: UserId,
        respondent_username: String,
    ) -> Result<TradeId, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait AddTradeCardUseCase: Send + Sync {
    async fn add_card(
        &self,
        trade_id: TradeId,
        caller_id: UserId,
        owner_username: String,
        copy_id: CopyId,
    ) -> Result<(), AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait RemoveTradeCardUseCase: Send + Sync {
    async fn remove_card(
        &self,
        trade_id: TradeId,
        caller_id: UserId,
        owner_username: String,
        copy_id: CopyId,
    ) -> Result<(), AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait AutocompleteUsersUseCase: Send + Sync {
    async fn autocomplete(&self, query: Option<String>) -> Result<Vec<UserSuggestion>, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait AcceptTradeUseCase: Send + Sync {
    async fn accept(&self, trade_id: TradeId, caller_id: UserId) -> Result<(), AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait AbandonTradeUseCase: Send + Sync {
    async fn abandon(&self, trade_id: TradeId, caller_id: UserId) -> Result<(), AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait ConfirmTradeUseCase: Send + Sync {
    async fn confirm(&self, trade_id: TradeId, caller_id: UserId) -> Result<(), AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait RateTradeUseCase: Send + Sync {
    async fn rate(&self, trade_id: TradeId, caller_id: UserId, rating: u8) -> Result<(), AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait GetTradeUseCase: Send + Sync {
    async fn get_trade(
        &self,
        trade_id: TradeId,
        caller_id: UserId,
    ) -> Result<TradeDetail, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait ListTradesUseCase: Send + Sync {
    /// Rejects the requested page with `InvalidPageSize` or `PaginationTooDeep` (`400`) when it
    /// is out of the trade list's bounds.
    async fn list_trades(
        &self,
        caller_id: UserId,
        query: TradeListQuery<PageRequest>,
    ) -> Result<Paginated<TradeSummary>, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait ListSetsUseCase: Send + Sync {
    async fn list_sets(&self) -> Result<Vec<SetName>, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait GetSetUseCase: Send + Sync {
    async fn get_set(&self, code: SetCode) -> Result<SetName, AppError>;
}
