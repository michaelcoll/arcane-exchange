use crate::application::error::AppError;
use crate::application::imported_card::ImportedCard;
use crate::domain::card::{Card, CardId, CollectionEntry, CopyId};
use crate::domain::card_import::{CardImport, CardImportId, CardImportStatus};
use crate::domain::card_offer::CardOfferSortField;
use crate::domain::collection::{CollectionQuery, SearchQuery};
use crate::domain::collection_stats::CollectionStats;
use crate::domain::pagination::{Paginated, Pagination};
use crate::domain::price::{FullPriceGuide, PriceHistoryEntry};
use crate::domain::rarity_trade_filter::{RarityTradeFilter, RarityTradeFilterRule};
use crate::domain::set_name::{SetCode, SetName};
use crate::domain::trade::{
    Trade, TradeCard, TradeCardDetail, TradeId, TradeListQuery, TradeSummary, TradeTransition,
};
use crate::domain::user::{CollectionVisibility, User, UserId, UserSuggestion};
use async_trait::async_trait;
use chrono::NaiveDate;
#[cfg(test)]
use mockall::automock;

#[async_trait]
#[cfg_attr(test, automock)]
pub trait CardRepository: Send + Sync {
    async fn get_all_without_cardmarket_id(&self) -> Result<Vec<(CardId, uuid::Uuid)>, AppError>;
    async fn get_all_without_gatherer_id(&self) -> Result<Vec<(CardId, String)>, AppError>;
    /// Returns `cardmarket_id` for the card matching `scryfall_id`, if any.
    async fn find_by_scryfall_id(
        &self,
        scryfall_id: uuid::Uuid,
    ) -> Result<Option<Option<u32>>, AppError>;
    /// Writes `cards` in bulk (grouped `INSERT ... ON CONFLICT DO UPDATE`), in one transaction.
    /// `cards` is expected to already fit within a single statement's bound-parameter budget —
    /// callers importing a large collection chunk it themselves.
    async fn save_all(&self, user: &User, cards: &[ImportedCard]) -> Result<(), AppError>;
    async fn update_cardmarket_id(
        &self,
        id: CardId,
        cardmarket_id: Option<u32>,
    ) -> Result<(), AppError>;
    async fn update_gatherer_id(
        &self,
        id: CardId,
        gatherer_id: Option<String>,
    ) -> Result<(), AppError>;
    async fn delete_all(&self, user: User) -> Result<(), AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait SetNameRepository: Send + Sync {
    /// Writes `sets` in bulk, `ON CONFLICT DO NOTHING` — matches the existing per-set behaviour
    /// where an already-known set's name is never overwritten.
    async fn save_all(&self, sets: &[SetName]) -> Result<(), AppError>;
    /// All known sets, ordered by name.
    async fn find_all(&self) -> Result<Vec<SetName>, AppError>;
    async fn find_by_code(&self, code: SetCode) -> Result<Option<SetName>, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait CollectionPriceHistoryRepository: Send + Sync {
    async fn get_date_and_user_to_update(&self) -> Result<Vec<(NaiveDate, User)>, AppError>;
    async fn update_for_date_and_user(&self, date: NaiveDate, user: User) -> Result<(), AppError>;
    async fn get_price_history(
        &self,
        user_id: &UserId,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<PriceHistoryEntry>, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait CardMarketPriceRepository: Send + Sync {
    async fn save(
        &self,
        date: NaiveDate,
        price_guides: Vec<FullPriceGuide>,
    ) -> Result<(), AppError>;

    async fn find_by_id_and_date(
        &self,
        id_product: u32,
        date: NaiveDate,
    ) -> Result<Option<FullPriceGuide>, AppError>;

    async fn find_by_id_and_date_range(
        &self,
        id_product: u32,
        foil: bool,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<PriceHistoryEntry>, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait CardPricesViewRepository: Send + Sync {
    /// Refreshes `mv_last_cardmarket_prices` before `mv_card_prices`, which reads it — this
    /// order is load-bearing, see the adapter implementation. Both refreshes are attempted even
    /// if the first one fails.
    async fn refresh(&self) -> Result<(), AppError>;
    /// The authenticated user's private collection. Always filtered by `user_id`.
    async fn get_paginated(
        &self,
        user_id: &UserId,
        query: CollectionQuery,
    ) -> Result<Paginated<Card>, AppError>;
    /// Public search across every user's cards. No `user_id` filter — rows are
    /// grouped by card, each returned as `CollectionEntry::Public { owner_count }`
    /// where `owner_count` is the number of distinct users owning that card. When
    /// `query.player_username` is set, results are restricted to that player's cards
    /// (exact match, case-insensitive) and `owner_count` is always `1`.
    async fn search_paginated(&self, query: SearchQuery) -> Result<Paginated<Card>, AppError>;
    /// Whether `card_id` exists in the catalog (table `card`), regardless of who owns it, or
    /// whether anyone owns it at all.
    async fn exists(&self, card_id: &CardId) -> Result<bool, AppError>;
    /// Other users' offers for `copy_id` (the caller's own entry, if any, is excluded).
    async fn get_offers(
        &self,
        user_id: &UserId,
        copy_id: &CopyId,
        sort_by: CardOfferSortField,
        pagination: Pagination,
    ) -> Result<Paginated<CollectionEntry>, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait StatsRepository: Send + Sync {
    async fn get_card_number(&self) -> Result<u32, AppError>;
    async fn get_card_price_number(&self) -> Result<u32, AppError>;
    async fn get_db_size(&self) -> Result<u16, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait CollectionStatsRepository: Send + Sync {
    async fn get_collection_stats(&self, user_id: &UserId) -> Result<CollectionStats, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait UserRepository: Send + Sync {
    async fn upsert(&self, user: &User) -> Result<(), AppError>;
    async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, AppError>;
    /// Exact match on username, case-insensitive.
    async fn find_by_username(&self, username: &str) -> Result<Option<User>, AppError>;
    /// Fuzzy trigram search on username (ILIKE substring + word_similarity), ordered by
    /// descending similarity score, capped at `limit`. `query` is expected already trimmed
    /// and non-empty (checked at the service level).
    async fn autocomplete(&self, query: &str, limit: i64) -> Result<Vec<UserSuggestion>, AppError>;

    /// Current visibility of `id`'s collection. `None` if the user doesn't exist in `users`.
    async fn get_visibility(&self, id: &UserId) -> Result<Option<CollectionVisibility>, AppError>;

    /// Updates the visibility of `id`'s collection. Returns `false` if no user with this id exists
    /// (nothing updated), `true` otherwise.
    async fn set_visibility(
        &self,
        id: &UserId,
        visibility: CollectionVisibility,
    ) -> Result<bool, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait TradingBinderRepository: Send + Sync {
    /// Names of the binders selected by `user_id`, sorted by name.
    async fn list(&self, user_id: &UserId) -> Result<Vec<String>, AppError>;

    /// Selects `binder_name` for `user_id`. Idempotent: selecting an already-selected binder
    /// is a no-op.
    async fn add(&self, user_id: &UserId, binder_name: &str) -> Result<(), AppError>;

    /// Deselects `binder_name` for `user_id`. Idempotent: removing a binder that isn't selected
    /// is a no-op.
    async fn remove(&self, user_id: &UserId, binder_name: &str) -> Result<(), AppError>;

    /// `true` if `binder_name` is used by at least one of `user_id`'s `collection_entry` rows.
    /// Reads `collection_entry` (read-only) rather than `trading_binders`.
    async fn binder_exists(&self, user_id: &UserId, binder_name: &str) -> Result<bool, AppError>;

    /// Removes `user_id`'s selected binders that no longer match any of their
    /// `collection_entry` rows. Reads `collection_entry` (read-only) to determine which
    /// selections are now orphaned.
    async fn purge_missing(&self, user_id: &UserId) -> Result<(), AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait RarityTradeFilterRepository: Send + Sync {
    /// Rarities actually owned by `user_id` within their binders selected for trade
    /// (`trading_binders`), each with its trade rule (defaulting to closed / 0 kept copies
    /// when no `collection_rarity_filters` row exists), its total copies, and the copies
    /// still proposable under that rule. Ordered `M, R, U, C, S`.
    async fn list_with_counts(&self, user_id: &UserId) -> Result<Vec<RarityTradeFilter>, AppError>;

    /// Creates or updates `user_id`'s trade rule for `rule.rarity`.
    async fn upsert(&self, user_id: &UserId, rule: &RarityTradeFilterRule) -> Result<(), AppError>;
}

/// Which quantity bounds a card added to a trade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardAvailability {
    /// Every copy the owner has in their collection: the caller disposes freely of their own side.
    Owned,
    /// Only what the owner actually offers to trade (`v_tradable_entry`: visibility, trade binders
    /// and rarity filters applied), for a card put up on the other party's behalf.
    Offered,
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait TradeRepository: Send + Sync {
    /// Returns the active trade (`PENDING`, `ONE_ACCEPTED` or `FULLY_ACCEPTED`) between the two
    /// users, whichever of them opened it, or creates it at `PENDING` with `initiator_id` as
    /// initiator when there is none. Atomic: the unique partial index `trade_one_active_per_pair`
    /// makes concurrent calls for the same pair converge on a single trade.
    async fn create_or_find_active(
        &self,
        initiator_id: &UserId,
        respondent_id: &UserId,
    ) -> Result<TradeId, AppError>;

    /// Fetches a trade by its id, if it exists.
    async fn find_by_id(&self, id: TradeId) -> Result<Option<Trade>, AppError>;

    /// Fetches every card offered in a trade.
    async fn find_trade_cards(&self, trade_id: TradeId) -> Result<Vec<TradeCard>, AppError>;

    /// Fetches every card offered in a trade, enriched with name/price/image ids for display.
    /// Unlike `find_trade_cards`, this survives the owner later removing the card from their
    /// collection (it joins `card`/`mv_last_cardmarket_prices`, not the collection-gated
    /// `mv_card_prices` view).
    async fn find_trade_cards_with_details(
        &self,
        trade_id: TradeId,
    ) -> Result<Vec<TradeCardDetail>, AppError>;

    /// Lists every trade `caller_id` is a party to (as initiator or respondent), optionally
    /// filtered by status, ordered by `updated_at` descending.
    async fn list_trades(
        &self,
        caller_id: &UserId,
        query: TradeListQuery,
    ) -> Result<Paginated<TradeSummary>, AppError>;

    /// Adds `copy_id` to the trade of `transition` (or increments its `quantity` if already
    /// present for `owner_id`), and writes `transition` (see `Trade::modify`).
    ///
    /// Fails, changing nothing, with:
    /// - `TradeNotFound` if the trade doesn't exist;
    /// - `TradeNotModifiable` if its status is no longer `transition.from`;
    /// - `CardNotFound` if the resulting total for (`copy_id`, `owner_id`) in this trade exceeds
    ///   what `availability` allows;
    /// - `CardAlreadyReserved` if the card is committed to another `ONE_ACCEPTED` or
    ///   `FULLY_ACCEPTED` trade.
    ///
    /// Atomic: concurrent additions to the same trade cannot together exceed the available
    /// quantity.
    async fn merge_card_into_trade(
        &self,
        transition: &TradeTransition,
        copy_id: &CopyId,
        owner_id: &UserId,
        quantity: u8,
        availability: CardAvailability,
    ) -> Result<(), AppError>;

    /// Removes the trade_card row identified by `copy_id` + `owner_id` from the trade of
    /// `transition`, entirely (no partial quantity decrement), and writes `transition` (see
    /// `Trade::modify`). Fails with `TradeNotModifiable` if the trade's status is no longer
    /// `transition.from`. Returns `false` if no matching row existed (nothing changed, trade
    /// untouched).
    async fn remove_card_from_trade(
        &self,
        transition: &TradeTransition,
        copy_id: &CopyId,
        owner_id: &UserId,
    ) -> Result<bool, AppError>;

    /// Writes the status and party columns of `transition.next`, provided the trade still has
    /// status `transition.from`. When the transition reserves the trade's cards, every other
    /// active trade (`PENDING`/`ONE_ACCEPTED`) sharing one of them is abandoned in the same
    /// transaction. Returns `false`, changing nothing, if the status no longer matched: the
    /// decision was taken on a stale read.
    async fn apply_transition(&self, transition: &TradeTransition) -> Result<bool, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait CardImportRepository: Send + Sync {
    /// Creates the import at `Pending`. Returns `FunctionalError::ImportAlreadyRunning` if the
    /// user already has an import at `Pending` or `Running` (enforced by a unique partial index,
    /// not a prior `SELECT`, so it stays correct under concurrent requests).
    async fn create(&self, import: &CardImport) -> Result<(), AppError>;

    async fn find_by_id(&self, id: &CardImportId) -> Result<Option<CardImport>, AppError>;

    /// Most recent first.
    async fn list_by_user(&self, user_id: &UserId) -> Result<Vec<CardImport>, AppError>;

    async fn mark_running(&self, id: &CardImportId) -> Result<(), AppError>;

    async fn update_progress(
        &self,
        id: &CardImportId,
        processed_lines: u32,
    ) -> Result<(), AppError>;

    async fn finish(
        &self,
        id: &CardImportId,
        status: CardImportStatus,
        error_message: Option<&str>,
    ) -> Result<(), AppError>;

    /// Marks every `Pending`/`Running` import as `Failed` (used at server startup). Returns the
    /// number of imports affected.
    async fn fail_all_active(&self, reason: &str) -> Result<u64, AppError>;

    /// Deletes the user's completed/failed imports beyond the `keep` most recent ones. An active
    /// (`Pending`/`Running`) import is never purged.
    async fn purge_old(&self, user_id: &UserId, keep: i64) -> Result<(), AppError>;
}
