use crate::application::error::AppError;
use async_trait::async_trait;

use crate::domain::card::{Card, CardId, CollectionEntry};
use crate::domain::card_offer::CardOfferSortField;
use crate::domain::collection::{CollectionQuery, SearchQuery};
use crate::domain::collection_stats::CollectionStats;
use crate::domain::pagination::{Paginated, Pagination};
use crate::domain::price::PriceHistoryEntry;
use crate::domain::rarity_trade_filter::{RarityTradeFilter, RarityTradeFilterRule};
use crate::domain::stats::Stats;
use crate::domain::trade::{TradeDetail, TradeId, TradeListQuery, TradeSummary};
use crate::domain::user::{CollectionVisibility, User, UserId, UserSuggestion};
#[cfg(test)]
use mockall::automock;

#[async_trait]
#[cfg_attr(test, automock)]
pub trait ImportCardUseCase: Send + Sync {
    async fn import_cards(&self, csv: &str, user: User) -> Result<(), AppError>;
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
pub trait EnqueueGathererIdUpdateUseCase: Send + Sync {
    async fn enqueue_pending_updates(&self) -> Result<usize, AppError>;
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
pub trait StatsUseCase: Send + Sync {
    async fn get_stats(&self) -> Result<Stats, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait GetCollectionUseCase: Send + Sync {
    async fn get_collection(
        &self,
        user_id: &UserId,
        query: CollectionQuery,
    ) -> Result<Paginated<Card>, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait SearchCardsUseCase: Send + Sync {
    async fn search_cards(&self, query: SearchQuery) -> Result<Paginated<Card>, AppError>;
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
    async fn get_card_offers(
        &self,
        user_id: &UserId,
        card_id: CardId,
        sort_by: CardOfferSortField,
        pagination: Pagination,
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
        card_id: CardId,
        quantity: u8,
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
        card_id: CardId,
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
    async fn list_trades(
        &self,
        caller_id: UserId,
        query: TradeListQuery,
    ) -> Result<Paginated<TradeSummary>, AppError>;
}
