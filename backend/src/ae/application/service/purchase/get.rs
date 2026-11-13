use crate::application::error::AppError;
use crate::application::repository::PurchaseRepository;
use crate::application::use_case::GetPurchaseUseCase;
use crate::domain::purchase::{Purchase, PurchaseId};
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct GetPurchaseService {
    repository: Arc<dyn PurchaseRepository>,
}

impl GetPurchaseService {
    pub fn new(repository: Arc<dyn PurchaseRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl GetPurchaseUseCase for GetPurchaseService {
    async fn get_purchase(&self, user_id: UserId, id: PurchaseId) -> Result<Purchase, AppError> {
        self.repository.find(&user_id, id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::MockPurchaseRepository;
    use crate::application::service::purchase::test_fixtures::{booster, purchase_id};
    use crate::domain::error::FunctionalError;

    #[tokio::test]
    async fn get_purchase_returns_the_purchase_of_the_player() {
        let mut mock_repository = MockPurchaseRepository::new();
        mock_repository
            .expect_find()
            .withf(|user, id| user.as_str() == "user_1" && *id == purchase_id())
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(booster(None)) }));

        let service = GetPurchaseService::new(Arc::new(mock_repository));
        let purchase = service
            .get_purchase(UserId::new("user_1"), purchase_id())
            .await
            .unwrap();

        assert_eq!(purchase, booster(None));
    }

    #[tokio::test]
    async fn get_purchase_propagates_purchase_not_found() {
        let mut mock_repository = MockPurchaseRepository::new();
        mock_repository
            .expect_find()
            .times(1)
            .returning(|_, _| Box::pin(async { Err(FunctionalError::PurchaseNotFound.into()) }));

        let service = GetPurchaseService::new(Arc::new(mock_repository));
        let result = service
            .get_purchase(UserId::new("user_1"), purchase_id())
            .await;

        match result.unwrap_err() {
            AppError::Functional(FunctionalError::PurchaseNotFound) => {}
            other => panic!("Expected PurchaseNotFound, got {other:?}"),
        }
    }
}
