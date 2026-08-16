use crate::application::error::{AppError, InfraError};
use crate::application::repository::PersistenceError;
use sqlx::Error;

pub mod card_prices_view_repository_adapter;
pub mod card_repository_adapter;
pub mod cardmarket_price_repository_adapter;
pub mod collection_price_history_repository_adapter;
pub mod collection_stats_repository_adapter;
#[cfg(test)]
pub mod common_repository_tests;
pub mod entities;
pub mod set_names_repository_adapter;
pub mod stats_repository_adapter;
pub mod trade_repository_adapter;
pub mod trading_binders_repository_adapter;
pub mod user_repository_adapter;

impl From<Error> for AppError {
    fn from(err: Error) -> Self {
        AppError::Infra(InfraError::RepositoryError(err.to_string()))
    }
}

impl From<Error> for PersistenceError {
    fn from(_err: Error) -> Self {
        PersistenceError::DBError(_err.to_string())
    }
}
