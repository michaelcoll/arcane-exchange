use crate::application::error::AppError;
use crate::domain::card::{Card, CardId};
use crate::domain::card_offer::{CardOfferSortField, PaginatedCardOffers};
use crate::domain::collection::{CollectionQuery, PaginatedCollection};
use crate::domain::collection_stats::CollectionStats;
use crate::domain::price::{FullPriceGuide, PriceHistoryEntry};
use crate::domain::set_name::{SetCode, SetName};
use crate::domain::trade::{Trade, TradeCard, TradeId, TradeStatus};
use crate::domain::user::{User, UserId};
use async_trait::async_trait;
use chrono::NaiveDate;
#[cfg(test)]
use mockall::automock;

#[derive(Debug, PartialEq, Eq)]
pub enum PersistenceError {
    DBError(String),
}

impl From<PersistenceError> for String {
    fn from(val: PersistenceError) -> String {
        match val {
            PersistenceError::DBError(msg) => msg,
        }
    }
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait CardRepository: Send + Sync {
    #[allow(dead_code)]
    async fn get_all(&self, user: User) -> Result<Vec<Card>, AppError>;
    async fn get_all_without_cardmarket_id(&self) -> Result<Vec<(CardId, uuid::Uuid)>, AppError>;
    async fn get_all_without_gatherer_id(&self) -> Result<Vec<(CardId, String)>, AppError>;
    /// Returns `(cardmarket_id, foil)` for the card matching `scryfall_id`, if any.
    async fn find_by_scryfall_id(
        &self,
        scryfall_id: uuid::Uuid,
    ) -> Result<Option<(Option<u32>, bool)>, AppError>;
    async fn save(&self, user: User, card: Card) -> Result<(), AppError>;
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
    async fn exists_by_code(&self, code: SetCode) -> Result<bool, AppError>;
    async fn save(&self, set: SetName) -> Result<(), AppError>;
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
    async fn refresh(&self) -> Result<(), AppError>;
    async fn get_paginated(
        &self,
        user_id: &UserId,
        query: CollectionQuery,
    ) -> Result<PaginatedCollection, AppError>;
    /// Whether any user owns a card matching `card_id`, regardless of who.
    async fn exists(&self, card_id: &CardId) -> Result<bool, AppError>;
    /// Other users' offers for `card_id` (the caller's own entry, if any, is excluded).
    async fn get_offers(
        &self,
        user_id: &UserId,
        card_id: &CardId,
        sort_by: CardOfferSortField,
        page: u32,
        page_size: u32,
    ) -> Result<PaginatedCardOffers, AppError>;
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
pub trait CollectionRepository: Send + Sync {
    async fn get_paginated(
        &self,
        user_id: &UserId,
        query: CollectionQuery,
    ) -> Result<PaginatedCollection, AppError>;
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
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait TradeRepository: Send + Sync {
    async fn find_collection_entry_quantity(
        &self,
        user_id: &UserId,
        card_id: &CardId,
    ) -> Result<Option<i32>, AppError>;

    /// Finds the active trade (`PENDING`, `ONE_ACCEPTED` or `FULLY_ACCEPTED`) between two users, if any,
    /// regardless of which one is `initiator_user_id` vs `respondent_user_id` on that trade.
    async fn find_active_trade(
        &self,
        user_a: &UserId,
        user_b: &UserId,
    ) -> Result<Option<(TradeId, TradeStatus)>, AppError>;

    /// Fetches a trade by its id, if it exists.
    async fn find_by_id(&self, id: TradeId) -> Result<Option<Trade>, AppError>;

    /// Fetches every card offered in a trade.
    async fn find_trade_cards(&self, trade_id: TradeId) -> Result<Vec<TradeCard>, AppError>;

    async fn create(
        &self,
        id: TradeId,
        initiator_id: &UserId,
        respondent_id: &UserId,
        card_id: &CardId,
        quantity: u8,
    ) -> Result<(), AppError>;

    /// Adds `card_id` to an existing trade (or increments its `quantity` if already present for `owner_id`).
    /// When `reopen_to_pending` is true, the trade's status is reset to `PENDING`.
    async fn merge_card_into_trade(
        &self,
        trade_id: TradeId,
        card_id: &CardId,
        owner_id: &UserId,
        quantity: u8,
        reopen_to_pending: bool,
    ) -> Result<(), AppError>;
}
