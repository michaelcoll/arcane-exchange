use crate::application::date_range::resolve_date_range;
use crate::application::error::AppError;
use crate::application::repository::CollectionPriceHistoryRepository;
use crate::application::use_case::GetCollectionPriceHistoryUseCase;
use crate::domain::price::PriceHistoryEntry;
use crate::domain::user::UserId;
use async_trait::async_trait;
use chrono::NaiveDate;
use std::sync::Arc;

pub struct CollectionPriceHistoryService {
    repository: Arc<dyn CollectionPriceHistoryRepository>,
}

impl CollectionPriceHistoryService {
    pub fn new(repository: Arc<dyn CollectionPriceHistoryRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl GetCollectionPriceHistoryUseCase for CollectionPriceHistoryService {
    async fn get_collection_price_history(
        &self,
        user_id: &UserId,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Result<Vec<PriceHistoryEntry>, AppError> {
        let (start_date, end_date) = resolve_date_range(start_date, end_date)?;
        self.repository
            .get_price_history(user_id, start_date, end_date)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::error::InfraError;
    use crate::application::repository::MockCollectionPriceHistoryRepository;
    use crate::domain::error::FunctionalError;
    use crate::domain::price::{Price, PriceGuide};
    use chrono::NaiveDate;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[tokio::test]
    async fn returns_history_from_repository() {
        let mut mock = MockCollectionPriceHistoryRepository::new();
        mock.expect_get_price_history()
            .withf(|uid, s, e| {
                uid == &UserId::new("user1")
                    && *s == NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()
                    && *e == NaiveDate::from_ymd_opt(2025, 1, 31).unwrap()
            })
            .returning(|_, _, _| {
                Box::pin(async {
                    Ok(vec![PriceHistoryEntry {
                        date: NaiveDate::from_ymd_opt(2025, 1, 15).unwrap(),
                        price_guide: PriceGuide {
                            low: Price { value: Some(100) },
                            trend: Price { value: Some(150) },
                            avg: Price { value: Some(130) },
                        },
                    }])
                })
            });

        let service = CollectionPriceHistoryService::new(Arc::new(mock));
        let result = service
            .get_collection_price_history(
                &UserId::new("user1"),
                Some(date(2025, 1, 1)),
                Some(date(2025, 1, 31)),
            )
            .await;

        assert!(result.is_ok());
        let entries = result.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].price_guide.low.value, Some(100));
        assert_eq!(entries[0].price_guide.trend.value, Some(150));
        assert_eq!(entries[0].price_guide.avg.value, Some(130));
    }

    #[tokio::test]
    async fn returns_error_when_start_after_end() {
        let mock = MockCollectionPriceHistoryRepository::new();
        let service = CollectionPriceHistoryService::new(Arc::new(mock));

        let result = service
            .get_collection_price_history(
                &UserId::new("user1"),
                Some(date(2025, 2, 1)),
                Some(date(2025, 1, 1)),
            )
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Functional(FunctionalError::WrongFormat(msg)) => {
                assert_eq!(msg, "start_date must be before or equal to end_date");
            }
            _ => panic!("Expected WrongFormat error"),
        }
    }

    #[tokio::test]
    async fn accepts_same_start_and_end_date() {
        let mut mock = MockCollectionPriceHistoryRepository::new();
        mock.expect_get_price_history()
            .returning(|_, _, _| Box::pin(async { Ok(vec![]) }));

        let service = CollectionPriceHistoryService::new(Arc::new(mock));
        let result = service
            .get_collection_price_history(
                &UserId::new("user1"),
                Some(date(2025, 6, 1)),
                Some(date(2025, 6, 1)),
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn propagates_repository_error() {
        let mut mock = MockCollectionPriceHistoryRepository::new();
        mock.expect_get_price_history().returning(|_, _, _| {
            Box::pin(async {
                Err(AppError::Infra(InfraError::RepositoryError(
                    "db error".to_string(),
                )))
            })
        });

        let service = CollectionPriceHistoryService::new(Arc::new(mock));
        let result = service
            .get_collection_price_history(
                &UserId::new("user1"),
                Some(date(2025, 1, 1)),
                Some(date(2025, 1, 31)),
            )
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Infra(InfraError::RepositoryError(msg)) => assert_eq!(msg, "db error"),
            _ => panic!("Expected RepositoryError"),
        }
    }

    #[tokio::test]
    async fn defaults_to_last_30_days_when_no_dates_provided() {
        use chrono::{Days, Utc};

        let today = Utc::now().date_naive();
        let expected_start = today - Days::new(30);

        let mut mock = MockCollectionPriceHistoryRepository::new();
        mock.expect_get_price_history()
            .withf(move |_, s, e| *s == expected_start && *e == today)
            .returning(|_, _, _| Box::pin(async { Ok(vec![]) }));

        let service = CollectionPriceHistoryService::new(Arc::new(mock));
        let result = service
            .get_collection_price_history(&UserId::new("user1"), None, None)
            .await;

        assert!(result.is_ok());
    }
}
