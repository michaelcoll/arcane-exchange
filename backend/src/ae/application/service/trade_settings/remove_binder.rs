use crate::application::error::AppError;
use crate::application::repository::TradingBinderRepository;
use crate::application::use_case::RemoveTradeBinderUseCase;
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct RemoveTradeBinderService {
    repository: Arc<dyn TradingBinderRepository>,
}

impl RemoveTradeBinderService {
    pub fn new(repository: Arc<dyn TradingBinderRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl RemoveTradeBinderUseCase for RemoveTradeBinderService {
    async fn remove_trade_binder(
        &self,
        user_id: UserId,
        binder_name: String,
    ) -> Result<(), AppError> {
        self.repository.remove(&user_id, &binder_name).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::error::InfraError;
    use crate::application::repository::MockTradingBinderRepository;

    #[tokio::test]
    async fn remove_trade_binder_calls_repository_with_id_and_name() {
        let mut mock_repository = MockTradingBinderRepository::new();
        mock_repository
            .expect_remove()
            .withf(|id, name| id.as_str() == "user_1" && name == "Trade Binder")
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(()) }));

        let service = RemoveTradeBinderService::new(Arc::new(mock_repository));
        let result = service
            .remove_trade_binder(UserId::new("user_1"), "Trade Binder".to_string())
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn remove_trade_binder_propagates_repository_error() {
        let mut mock_repository = MockTradingBinderRepository::new();
        mock_repository.expect_remove().times(1).returning(|_, _| {
            Box::pin(async {
                Err(AppError::Infra(InfraError::RepositoryError(
                    "DB error".to_string(),
                )))
            })
        });

        let service = RemoveTradeBinderService::new(Arc::new(mock_repository));
        let result = service
            .remove_trade_binder(UserId::new("user_1"), "Trade Binder".to_string())
            .await;

        match result.unwrap_err() {
            AppError::Infra(InfraError::RepositoryError(msg)) => assert_eq!(msg, "DB error"),
            other => panic!("Expected RepositoryError, got {other:?}"),
        }
    }
}
