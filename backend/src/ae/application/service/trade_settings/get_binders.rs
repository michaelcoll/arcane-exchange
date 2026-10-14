use crate::application::error::AppError;
use crate::application::repository::TradingBinderRepository;
use crate::application::use_case::GetTradeBindersUseCase;
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct GetTradeBindersService {
    repository: Arc<dyn TradingBinderRepository>,
}

impl GetTradeBindersService {
    pub fn new(repository: Arc<dyn TradingBinderRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl GetTradeBindersUseCase for GetTradeBindersService {
    async fn get_trade_binders(&self, user_id: UserId) -> Result<Vec<String>, AppError> {
        self.repository.list(&user_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::error::InfraError;
    use crate::application::repository::MockTradingBinderRepository;

    #[tokio::test]
    async fn get_trade_binders_returns_value_from_repository() {
        let mut mock_repository = MockTradingBinderRepository::new();
        mock_repository
            .expect_list()
            .times(1)
            .returning(|_| Box::pin(async { Ok(vec!["Trade Binder".to_string()]) }));

        let service = GetTradeBindersService::new(Arc::new(mock_repository));
        let result = service.get_trade_binders(UserId::new("user_1")).await;

        assert_eq!(result.unwrap(), vec!["Trade Binder".to_string()]);
    }

    #[tokio::test]
    async fn get_trade_binders_propagates_repository_error() {
        let mut mock_repository = MockTradingBinderRepository::new();
        mock_repository.expect_list().times(1).returning(|_| {
            Box::pin(async {
                Err(AppError::Infra(InfraError::RepositoryError(
                    "DB error".to_string(),
                )))
            })
        });

        let service = GetTradeBindersService::new(Arc::new(mock_repository));
        let result = service.get_trade_binders(UserId::new("user_1")).await;

        match result.unwrap_err() {
            AppError::Infra(InfraError::RepositoryError(msg)) => assert_eq!(msg, "DB error"),
            other => panic!("Expected RepositoryError, got {other:?}"),
        }
    }
}
