use crate::application::error::AppError;
use crate::application::repository::StatsRepository;
use crate::application::use_case::StatsUseCase;
use crate::domain::stats::Stats;
use async_trait::async_trait;
use std::sync::Arc;

pub struct StatsService {
    repository: Arc<dyn StatsRepository>,
}

impl StatsService {
    pub fn new(repository: Arc<dyn StatsRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl StatsUseCase for StatsService {
    async fn get_stats(&self) -> Result<Stats, AppError> {
        let card_number = self.repository.get_card_number().await?;
        let card_price_number = self.repository.get_card_price_number().await?;
        let db_size_mb = self.repository.get_db_size().await?;
        let last_price_date = self.repository.get_last_price_date().await?;

        Ok(Stats {
            card_number,
            card_price_number,
            db_size_mb,
            last_price_date,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::error::InfraError;
    use crate::application::repository::MockStatsRepository;
    use chrono::NaiveDate;

    #[tokio::test]
    async fn test_get_stats_returns_stats_on_success() {
        let mut mock_repository = MockStatsRepository::new();

        mock_repository
            .expect_get_card_number()
            .times(1)
            .returning(|| Box::pin(async { Ok(42) }));

        mock_repository
            .expect_get_card_price_number()
            .times(1)
            .returning(|| Box::pin(async { Ok(85) }));

        mock_repository
            .expect_get_db_size()
            .times(1)
            .returning(|| Box::pin(async { Ok(128) }));

        mock_repository
            .expect_get_last_price_date()
            .times(1)
            .returning(|| Box::pin(async { Ok(NaiveDate::from_ymd_opt(2026, 10, 1)) }));

        let service = StatsService::new(Arc::new(mock_repository));
        let result = service.get_stats().await;

        assert!(result.is_ok());
        let stats = result.unwrap();
        assert_eq!(stats.card_number, 42);
        assert_eq!(stats.card_price_number, 85);
        assert_eq!(stats.db_size_mb, 128);
        assert_eq!(stats.last_price_date, NaiveDate::from_ymd_opt(2026, 10, 1));
    }

    #[tokio::test]
    async fn test_get_stats_returns_error_on_last_price_date_repository_error() {
        let mut mock_repository = MockStatsRepository::new();
        mock_repository
            .expect_get_card_number()
            .returning(|| Box::pin(async { Ok(42) }));
        mock_repository
            .expect_get_card_price_number()
            .returning(|| Box::pin(async { Ok(85) }));
        mock_repository
            .expect_get_db_size()
            .returning(|| Box::pin(async { Ok(128) }));
        mock_repository
            .expect_get_last_price_date()
            .times(1)
            .returning(|| {
                Box::pin(async {
                    Err(AppError::Infra(InfraError::RepositoryError(
                        "Date DB error".to_string(),
                    )))
                })
            });

        let service = StatsService::new(Arc::new(mock_repository));
        let result = service.get_stats().await;

        match result.unwrap_err() {
            AppError::Infra(InfraError::RepositoryError(msg)) => assert_eq!(msg, "Date DB error"),
            _ => panic!("Expected RepositoryError"),
        }
    }

    #[tokio::test]
    async fn test_get_stats_returns_error_on_card_number_repository_error() {
        let mut mock_repository = MockStatsRepository::new();

        mock_repository
            .expect_get_card_number()
            .times(1)
            .returning(|| {
                Box::pin(async {
                    Err(AppError::Infra(InfraError::RepositoryError(
                        "DB error".to_string(),
                    )))
                })
            });

        let service = StatsService::new(Arc::new(mock_repository));
        let result = service.get_stats().await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Infra(InfraError::RepositoryError(msg)) => assert_eq!(msg, "DB error"),
            _ => panic!("Expected RepositoryError"),
        }
    }

    #[tokio::test]
    async fn test_get_stats_returns_error_on_card_price_number_repository_error() {
        let mut mock_repository = MockStatsRepository::new();

        mock_repository
            .expect_get_card_number()
            .times(1)
            .returning(|| Box::pin(async { Ok(42) }));

        mock_repository
            .expect_get_card_price_number()
            .times(1)
            .returning(|| {
                Box::pin(async {
                    Err(AppError::Infra(InfraError::RepositoryError(
                        "Price DB error".to_string(),
                    )))
                })
            });

        let service = StatsService::new(Arc::new(mock_repository));
        let result = service.get_stats().await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Infra(InfraError::RepositoryError(msg)) => assert_eq!(msg, "Price DB error"),
            _ => panic!("Expected RepositoryError"),
        }
    }

    #[tokio::test]
    async fn test_get_stats_returns_error_on_db_size_repository_error() {
        let mut mock_repository = MockStatsRepository::new();

        mock_repository
            .expect_get_card_number()
            .times(1)
            .returning(|| Box::pin(async { Ok(42) }));

        mock_repository
            .expect_get_card_price_number()
            .times(1)
            .returning(|| Box::pin(async { Ok(85) }));

        mock_repository.expect_get_db_size().times(1).returning(|| {
            Box::pin(async {
                Err(AppError::Infra(InfraError::RepositoryError(
                    "Size DB error".to_string(),
                )))
            })
        });

        let service = StatsService::new(Arc::new(mock_repository));
        let result = service.get_stats().await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Infra(InfraError::RepositoryError(msg)) => assert_eq!(msg, "Size DB error"),
            _ => panic!("Expected RepositoryError"),
        }
    }

    #[tokio::test]
    async fn test_get_stats_with_zero_values() {
        let mut mock_repository = MockStatsRepository::new();

        mock_repository
            .expect_get_card_number()
            .times(1)
            .returning(|| Box::pin(async { Ok(0) }));

        mock_repository
            .expect_get_card_price_number()
            .times(1)
            .returning(|| Box::pin(async { Ok(0) }));

        mock_repository
            .expect_get_db_size()
            .times(1)
            .returning(|| Box::pin(async { Ok(0) }));

        mock_repository
            .expect_get_last_price_date()
            .times(1)
            .returning(|| Box::pin(async { Ok(None) }));

        let service = StatsService::new(Arc::new(mock_repository));
        let result = service.get_stats().await;

        assert!(result.is_ok());
        let stats = result.unwrap();
        assert_eq!(stats.card_number, 0);
        assert_eq!(stats.card_price_number, 0);
        assert_eq!(stats.db_size_mb, 0);
        assert_eq!(stats.last_price_date, None);
    }

    #[tokio::test]
    async fn test_get_stats_with_max_values() {
        let mut mock_repository = MockStatsRepository::new();

        mock_repository
            .expect_get_card_number()
            .times(1)
            .returning(|| Box::pin(async { Ok(u32::MAX) }));

        mock_repository
            .expect_get_card_price_number()
            .times(1)
            .returning(|| Box::pin(async { Ok(u32::MAX) }));

        mock_repository
            .expect_get_db_size()
            .times(1)
            .returning(|| Box::pin(async { Ok(u16::MAX) }));

        mock_repository
            .expect_get_last_price_date()
            .times(1)
            .returning(|| Box::pin(async { Ok(None) }));

        let service = StatsService::new(Arc::new(mock_repository));
        let result = service.get_stats().await;

        assert!(result.is_ok());
        let stats = result.unwrap();
        assert_eq!(stats.card_number, u32::MAX);
        assert_eq!(stats.card_price_number, u32::MAX);
        assert_eq!(stats.db_size_mb, u16::MAX);
    }

    #[tokio::test]
    async fn test_get_stats_with_multiple_calls() {
        let mut mock_repository = MockStatsRepository::new();

        mock_repository
            .expect_get_card_number()
            .times(2)
            .returning(|| Box::pin(async { Ok(10) }));

        mock_repository
            .expect_get_card_price_number()
            .times(2)
            .returning(|| Box::pin(async { Ok(20) }));

        mock_repository
            .expect_get_db_size()
            .times(2)
            .returning(|| Box::pin(async { Ok(30) }));

        mock_repository
            .expect_get_last_price_date()
            .times(2)
            .returning(|| Box::pin(async { Ok(None) }));

        let service = Arc::new(StatsService::new(Arc::new(mock_repository)));

        let result1 = service.get_stats().await;
        assert!(result1.is_ok());
        let stats1 = result1.unwrap();
        assert_eq!(stats1.card_number, 10);

        let result2 = service.get_stats().await;
        assert!(result2.is_ok());
        let stats2 = result2.unwrap();
        assert_eq!(stats2.card_number, 10);
    }
}
