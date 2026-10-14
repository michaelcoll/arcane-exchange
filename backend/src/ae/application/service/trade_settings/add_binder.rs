use crate::application::error::AppError;
use crate::application::repository::TradingBinderRepository;
use crate::application::use_case::AddTradeBinderUseCase;
use crate::domain::error::FunctionalError;
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct AddTradeBinderService {
    repository: Arc<dyn TradingBinderRepository>,
}

impl AddTradeBinderService {
    pub fn new(repository: Arc<dyn TradingBinderRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl AddTradeBinderUseCase for AddTradeBinderService {
    async fn add_trade_binder(&self, user_id: UserId, binder_name: String) -> Result<(), AppError> {
        let binder_name = binder_name.trim();
        if binder_name.is_empty() {
            return Err(FunctionalError::WrongFormat("Binder name is empty".to_string()).into());
        }

        if !self.repository.binder_exists(&user_id, binder_name).await? {
            return Err(FunctionalError::BinderNotFound.into());
        }

        self.repository.add(&user_id, binder_name).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::error::InfraError;
    use crate::application::repository::MockTradingBinderRepository;

    #[tokio::test]
    async fn add_trade_binder_adds_when_binder_exists() {
        let mut mock_repository = MockTradingBinderRepository::new();
        mock_repository
            .expect_binder_exists()
            .withf(|id, name| id.as_str() == "user_1" && name == "Trade Binder")
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(true) }));
        mock_repository
            .expect_add()
            .withf(|id, name| id.as_str() == "user_1" && name == "Trade Binder")
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(()) }));

        let service = AddTradeBinderService::new(Arc::new(mock_repository));
        let result = service
            .add_trade_binder(UserId::new("user_1"), "Trade Binder".to_string())
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn add_trade_binder_fails_with_binder_not_found_when_absent_from_collection() {
        let mut mock_repository = MockTradingBinderRepository::new();
        mock_repository
            .expect_binder_exists()
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(false) }));

        let service = AddTradeBinderService::new(Arc::new(mock_repository));
        let result = service
            .add_trade_binder(UserId::new("user_1"), "Unknown".to_string())
            .await;

        match result.unwrap_err() {
            AppError::Functional(FunctionalError::BinderNotFound) => {}
            other => panic!("Expected BinderNotFound, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn add_trade_binder_fails_with_wrong_format_when_name_is_blank() {
        let mock_repository = MockTradingBinderRepository::new();

        let service = AddTradeBinderService::new(Arc::new(mock_repository));
        let result = service
            .add_trade_binder(UserId::new("user_1"), "   ".to_string())
            .await;

        match result.unwrap_err() {
            AppError::Functional(FunctionalError::WrongFormat(_)) => {}
            other => panic!("Expected WrongFormat, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn add_trade_binder_propagates_repository_error() {
        let mut mock_repository = MockTradingBinderRepository::new();
        mock_repository
            .expect_binder_exists()
            .times(1)
            .returning(|_, _| {
                Box::pin(async {
                    Err(AppError::Infra(InfraError::RepositoryError(
                        "DB error".to_string(),
                    )))
                })
            });

        let service = AddTradeBinderService::new(Arc::new(mock_repository));
        let result = service
            .add_trade_binder(UserId::new("user_1"), "Trade Binder".to_string())
            .await;

        match result.unwrap_err() {
            AppError::Infra(InfraError::RepositoryError(msg)) => assert_eq!(msg, "DB error"),
            other => panic!("Expected RepositoryError, got {other:?}"),
        }
    }
}
