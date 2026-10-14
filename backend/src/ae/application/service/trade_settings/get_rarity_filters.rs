use crate::application::error::AppError;
use crate::application::repository::RarityTradeFilterRepository;
use crate::application::use_case::GetRarityTradeFiltersUseCase;
use crate::domain::rarity_trade_filter::RarityTradeFilter;
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct GetRarityTradeFiltersService {
    repository: Arc<dyn RarityTradeFilterRepository>,
}

impl GetRarityTradeFiltersService {
    pub fn new(repository: Arc<dyn RarityTradeFilterRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl GetRarityTradeFiltersUseCase for GetRarityTradeFiltersService {
    async fn get_rarity_trade_filters(
        &self,
        user_id: UserId,
    ) -> Result<Vec<RarityTradeFilter>, AppError> {
        self.repository.list_with_counts(&user_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::error::InfraError;
    use crate::application::repository::MockRarityTradeFilterRepository;
    use crate::domain::rarity_code::RarityCode;

    #[tokio::test]
    async fn get_rarity_trade_filters_returns_value_from_repository() {
        let mut mock_repository = MockRarityTradeFilterRepository::new();
        mock_repository
            .expect_list_with_counts()
            .times(1)
            .returning(|_| {
                Box::pin(async {
                    Ok(vec![RarityTradeFilter {
                        rarity: RarityCode::R,
                        is_open: true,
                        kept_copies: 1,
                        copies: 4,
                        proposed: 2,
                    }])
                })
            });

        let service = GetRarityTradeFiltersService::new(Arc::new(mock_repository));
        let result = service
            .get_rarity_trade_filters(UserId::new("user_1"))
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].rarity, RarityCode::R);
        assert_eq!(result[0].proposed, 2);
    }

    #[tokio::test]
    async fn get_rarity_trade_filters_propagates_repository_error() {
        let mut mock_repository = MockRarityTradeFilterRepository::new();
        mock_repository
            .expect_list_with_counts()
            .times(1)
            .returning(|_| {
                Box::pin(async {
                    Err(AppError::Infra(InfraError::RepositoryError(
                        "DB error".to_string(),
                    )))
                })
            });

        let service = GetRarityTradeFiltersService::new(Arc::new(mock_repository));
        let result = service
            .get_rarity_trade_filters(UserId::new("user_1"))
            .await;

        match result.unwrap_err() {
            AppError::Infra(InfraError::RepositoryError(msg)) => assert_eq!(msg, "DB error"),
            other => panic!("Expected RepositoryError, got {other:?}"),
        }
    }
}
