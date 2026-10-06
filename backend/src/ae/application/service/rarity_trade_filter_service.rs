use crate::application::error::AppError;
use crate::application::repository::RarityTradeFilterRepository;
use crate::application::use_case::{GetRarityTradeFiltersUseCase, SetRarityTradeFilterUseCase};
use crate::domain::error::FunctionalError;
use crate::domain::rarity_trade_filter::{
    MAX_KEPT_COPIES, RarityTradeFilter, RarityTradeFilterRule,
};
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

pub struct SetRarityTradeFilterService {
    repository: Arc<dyn RarityTradeFilterRepository>,
}

impl SetRarityTradeFilterService {
    pub fn new(repository: Arc<dyn RarityTradeFilterRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl SetRarityTradeFilterUseCase for SetRarityTradeFilterService {
    async fn set_rarity_trade_filter(
        &self,
        user_id: UserId,
        rule: RarityTradeFilterRule,
    ) -> Result<(), AppError> {
        if rule.kept_copies > MAX_KEPT_COPIES {
            return Err(FunctionalError::WrongFormat(format!(
                "Kept copies must be between 0 and {}",
                MAX_KEPT_COPIES
            ))
            .into());
        }

        self.repository.upsert(&user_id, &rule).await
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

    #[tokio::test]
    async fn set_rarity_trade_filter_upserts_the_rule() {
        let mut mock_repository = MockRarityTradeFilterRepository::new();
        mock_repository
            .expect_upsert()
            .withf(|id, rule| {
                id.as_str() == "user_1" && rule.rarity == RarityCode::M && rule.kept_copies == 2
            })
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(()) }));

        let service = SetRarityTradeFilterService::new(Arc::new(mock_repository));
        let result = service
            .set_rarity_trade_filter(
                UserId::new("user_1"),
                RarityTradeFilterRule {
                    rarity: RarityCode::M,
                    is_open: true,
                    kept_copies: 2,
                },
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn set_rarity_trade_filter_rejects_kept_copies_above_max() {
        let mock_repository = MockRarityTradeFilterRepository::new();

        let service = SetRarityTradeFilterService::new(Arc::new(mock_repository));
        let result = service
            .set_rarity_trade_filter(
                UserId::new("user_1"),
                RarityTradeFilterRule {
                    rarity: RarityCode::M,
                    is_open: true,
                    kept_copies: 5,
                },
            )
            .await;

        match result.unwrap_err() {
            AppError::Functional(FunctionalError::WrongFormat(_)) => {}
            other => panic!("Expected WrongFormat, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn set_rarity_trade_filter_propagates_repository_error() {
        let mut mock_repository = MockRarityTradeFilterRepository::new();
        mock_repository.expect_upsert().times(1).returning(|_, _| {
            Box::pin(async {
                Err(AppError::Infra(InfraError::RepositoryError(
                    "DB error".to_string(),
                )))
            })
        });

        let service = SetRarityTradeFilterService::new(Arc::new(mock_repository));
        let result = service
            .set_rarity_trade_filter(
                UserId::new("user_1"),
                RarityTradeFilterRule {
                    rarity: RarityCode::M,
                    is_open: true,
                    kept_copies: 1,
                },
            )
            .await;

        match result.unwrap_err() {
            AppError::Infra(InfraError::RepositoryError(msg)) => assert_eq!(msg, "DB error"),
            other => panic!("Expected RepositoryError, got {other:?}"),
        }
    }
}
