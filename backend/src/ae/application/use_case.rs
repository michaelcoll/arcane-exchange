use crate::application::error::AppError;
use async_trait::async_trait;

use crate::application::card_import_job::CardImportJob;
use crate::domain::binder::{Binder, BinderId, ReceivingBinder};
use crate::domain::card::{Card, CardId, CollectionEntry, CopyId};
use crate::domain::card_import::{CardImport, CardImportId};
use crate::domain::card_offer::CardOfferSortField;
use crate::domain::catalog::{CatalogSearchItem, ScanIndex};
use crate::domain::collection::{CollectionQuery, SearchQuery, SearchScope};
use crate::domain::collection_entry::{CardEntries, CollectionEntryId};
use crate::domain::collection_stats::CollectionStats;
use crate::domain::pagination::{PageRequest, Paginated};
use crate::domain::price::PriceHistoryEntry;
use crate::domain::purchase::{Purchase, PurchaseEditRequest, PurchaseId, SealedProductRequest};
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
pub trait GetReceivingBinderUseCase: Send + Sync {
    /// The binder the player receives their Trades' cards into, `None` for the default (`trades`).
    async fn get_receiving_binder(
        &self,
        user_id: UserId,
    ) -> Result<Option<ReceivingBinder>, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait SetReceivingBinderUseCase: Send + Sync {
    /// Chooses the receiving binder, back to the default for `None`, and returns the setting as
    /// stored. `BinderNotFound` (`404`), also for a binder of another player.
    async fn set_receiving_binder(
        &self,
        user_id: UserId,
        binder_id: Option<BinderId>,
    ) -> Result<Option<ReceivingBinder>, AppError>;
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
pub trait ListBindersUseCase: Send + Sync {
    async fn list_binders(&self, user_id: UserId) -> Result<Vec<Binder>, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait CreateBinderUseCase: Send + Sync {
    /// Creates a binder closed to trade. `InvalidBinderName` (`400`) for a blank or too long name,
    /// `BinderNameTaken` (`409`) for a name the player already uses.
    async fn create_binder(&self, user_id: UserId, name: &str) -> Result<Binder, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait RenameBinderUseCase: Send + Sync {
    /// Renames a binder, which keeps its trade opening. Same refusals as creation, plus
    /// `BinderNotFound` (`404`), also for a binder of another player.
    async fn rename_binder(
        &self,
        user_id: UserId,
        id: BinderId,
        name: &str,
    ) -> Result<Binder, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait DeleteBinderUseCase: Send + Sync {
    /// Deletes an empty binder with its trade opening. `BinderNotFound` (`404`), also for a binder
    /// of another player, `BinderNotEmpty` (`409`) if it still holds copies.
    async fn delete_binder(&self, user_id: UserId, id: BinderId) -> Result<(), AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait ListPurchasesUseCase: Send + Sync {
    /// The player's Purchases, by purchase date then creation date, both most recent first.
    async fn list_purchases(
        &self,
        user_id: UserId,
        page: PageRequest,
    ) -> Result<Paginated<Purchase>, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait GetPurchaseUseCase: Send + Sync {
    /// `PurchaseNotFound` (`404`), also for a Purchase of another player.
    async fn get_purchase(&self, user_id: UserId, id: PurchaseId) -> Result<Purchase, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait CreateSealedProductUseCase: Send + Sync {
    /// Records a sealed product. A blank name is stored as absent. `InvalidPurchasePrice` (`400`)
    /// for an amount that is not strictly positive, `SetNotFound` (`404`) for an unknown set.
    async fn create_sealed_product(
        &self,
        user_id: UserId,
        request: SealedProductRequest,
    ) -> Result<Purchase, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait UpdatePurchaseUseCase: Send + Sync {
    /// Edits a Purchase, never the collection. An empty name clears the stored one. Refused like
    /// [`CreateSealedProductUseCase::create_sealed_product`], plus `PurchaseNotFound` (`404`), also
    /// for a Purchase of another player.
    async fn update_purchase(
        &self,
        user_id: UserId,
        id: PurchaseId,
        edit: PurchaseEditRequest,
    ) -> Result<Purchase, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait DeletePurchaseUseCase: Send + Sync {
    /// Deletes a Purchase and its content, never a collection entry. `PurchaseNotFound` (`404`),
    /// also for a Purchase of another player.
    async fn delete_purchase(&self, user_id: UserId, id: PurchaseId) -> Result<(), AppError>;
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
pub trait SearchCatalogUseCase: Send + Sync {
    /// Searches the catalog by English or French name (`query`, trimmed; a blank one matches
    /// every card) and, when given, by set. Paginated up to an offset of 1000. Tokens are not in
    /// the catalog, so they are never listed.
    ///
    /// Refused with `InvalidPageSize` / `PaginationTooDeep` (`400`) for a page out of bounds.
    async fn search_catalog(
        &self,
        query: Option<String>,
        set_code: Option<SetCode>,
        page: PageRequest,
    ) -> Result<Paginated<CatalogSearchItem>, AppError>;
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

#[async_trait]
#[cfg_attr(test, automock)]
pub trait GetCollectionEntriesUseCase: Send + Sync {
    /// What the player owns of a card, in both finishes, and what the accepted Trades committed
    /// of each. Empty entries for a card the player does not own.
    async fn get_collection_entries(
        &self,
        user_id: &UserId,
        card_id: CardId,
    ) -> Result<CardEntries, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait AddCollectionEntryUseCase: Send + Sync {
    /// Adds `quantity` copies of a card to the collection, in a finish and a binder (or none),
    /// without purchase price, and returns the entries of the card, then asks for a refresh of
    /// the card prices views without waiting for it. The Card is created when it does not exist
    /// ([`CreateCardUseCase`]), in the language asked even when Scryfall has none. The copies are
    /// merged into the entry of the same finish and binder when there is one.
    ///
    /// Refused with `InvalidQuantity` (`400`) outside 1 to 255 (also when a merge goes over 255),
    /// `CardNotInCatalog` (`404`) for a set and number the catalog does not have, and
    /// `BinderNotFound` (`404`) for an absent binder or another player's.
    async fn add_collection_entry(
        &self,
        user_id: &UserId,
        card_id: CardId,
        foil: bool,
        quantity: i32,
        binder_id: Option<BinderId>,
    ) -> Result<CardEntries, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait UpdateCollectionEntryUseCase: Send + Sync {
    /// Changes the quantity, the finish and/or the purchase price (cents per copy) of an entry and
    /// returns the entries of its card, then asks for a refresh of the card prices views without
    /// waiting for it. A finish change onto an existing entry merges the two. A new purchase
    /// price replaces the entry's and creates or changes no Purchase.
    ///
    /// Refused with `InvalidQuantity` (`400`) outside 1 to 255, `CollectionEntryNotFound`
    /// (`404`) for an absent entry or another player's, and `ReservedQuantityExceeded` (`409`)
    /// when it takes the owned copies under what the accepted Trades committed.
    async fn update_collection_entry(
        &self,
        user_id: &UserId,
        id: CollectionEntryId,
        quantity: Option<i32>,
        foil: Option<bool>,
        purchase_price: Option<crate::domain::purchase::PurchaseAmount>,
    ) -> Result<CardEntries, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait MoveCollectionEntryUseCase: Send + Sync {
    /// Moves `quantity` copies of an entry (all of them when `None`) into `binder_id`, or out of
    /// any binder when `None`, and returns the entries of its card. It merges into the entry
    /// already there and removes the source entry when it empties. It does not change what the
    /// player owns, so the reservation guard does not apply. Then asks for a refresh of the card
    /// prices views without waiting for it.
    ///
    /// Refused with `CollectionEntryNotFound` (`404`) for an absent entry or another player's,
    /// `BinderNotFound` (`404`), also for a binder of another player, and `InvalidQuantity`
    /// (`400`) for a quantity of 0 or over the entry's.
    async fn move_collection_entry(
        &self,
        user_id: &UserId,
        id: CollectionEntryId,
        binder_id: Option<BinderId>,
        quantity: Option<i32>,
    ) -> Result<CardEntries, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait RemoveCollectionEntryUseCase: Send + Sync {
    /// Removes an entry, then asks for a refresh of the card prices views without waiting for it.
    /// Refused like [`UpdateCollectionEntryUseCase::update_collection_entry`], minus the quantity.
    async fn remove_collection_entry(
        &self,
        user_id: &UserId,
        id: CollectionEntryId,
    ) -> Result<(), AppError>;
}
