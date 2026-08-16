use crate::application::error::AppError;
use crate::application::repository::CollectionStatsRepository;
use crate::application::use_case::GetCollectionStatsUseCase;
use crate::domain::collection_stats::CollectionStats;
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct CollectionStatsService {
    repository: Arc<dyn CollectionStatsRepository>,
}

impl CollectionStatsService {
    pub fn new(repository: Arc<dyn CollectionStatsRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl GetCollectionStatsUseCase for CollectionStatsService {
    async fn get_collection_stats(&self, user_id: &UserId) -> Result<CollectionStats, AppError> {
        self.repository.get_collection_stats(user_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::error::InfraError;
    use crate::application::repository::MockCollectionStatsRepository;
    use crate::domain::price::Price;

    #[tokio::test]
    async fn delegates_to_repository() {
        let mut mock = MockCollectionStatsRepository::new();
        mock.expect_get_collection_stats()
            .withf(|uid| uid == &UserId::new("user-1"))
            .returning(|_| {
                Box::pin(async {
                    Ok(CollectionStats {
                        total_cards: 10,
                        unique_cards: 5,
                        price_trend_min: Price::from_cents(100),
                        price_trend_max: Price::from_cents(1000),
                        sets: vec![],
                        binders: vec![],
                    })
                })
            });

        let service = CollectionStatsService::new(Arc::new(mock));
        let result = service.get_collection_stats(&UserId::new("user-1")).await;
        assert!(result.is_ok());
        let stats = result.unwrap();
        assert_eq!(stats.total_cards, 10);
        assert_eq!(stats.unique_cards, 5);
    }

    #[tokio::test]
    async fn propagates_repository_error() {
        let mut mock = MockCollectionStatsRepository::new();
        mock.expect_get_collection_stats().returning(|_| {
            Box::pin(async {
                Err(AppError::Infra(InfraError::RepositoryError(
                    "db error".to_string(),
                )))
            })
        });

        let service = CollectionStatsService::new(Arc::new(mock));
        let result = service.get_collection_stats(&UserId::new("user-1")).await;
        assert!(result.is_err());
    }
}
