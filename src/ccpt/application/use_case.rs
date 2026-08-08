use crate::application::error::AppError;
use async_trait::async_trait;

use crate::domain::card::CardId;
use crate::domain::card_offer::{CardOfferSortField, PaginatedCardOffers};
use crate::domain::collection::{CollectionQuery, PaginatedCollection, SearchQuery};
use crate::domain::collection_stats::CollectionStats;
use crate::domain::price::PriceHistoryEntry;
use crate::domain::stats::Stats;
use crate::domain::trade::TradeId;
use crate::domain::user::{User, UserId, UserSuggestion};
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
    ) -> Result<PaginatedCollection, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait SearchCardsUseCase: Send + Sync {
    async fn search_cards(&self, query: SearchQuery) -> Result<PaginatedCollection, AppError>;
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
        page: u32,
        page_size: u32,
    ) -> Result<PaginatedCardOffers, AppError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait CreateTradeUseCase: Send + Sync {
    async fn create_trade(
        &self,
        initiator_user_id: UserId,
        respondent_user_id: UserId,
        card_id: CardId,
        quantity: u8,
    ) -> Result<TradeId, AppError>;
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
