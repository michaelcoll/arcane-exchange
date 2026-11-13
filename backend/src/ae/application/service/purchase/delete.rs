use crate::application::error::AppError;
use crate::application::repository::PurchaseRepository;
use crate::application::use_case::DeletePurchaseUseCase;
use crate::domain::purchase::PurchaseId;
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct DeletePurchaseService {
    repository: Arc<dyn PurchaseRepository>,
}

impl DeletePurchaseService {
    pub fn new(repository: Arc<dyn PurchaseRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl DeletePurchaseUseCase for DeletePurchaseService {
    async fn delete_purchase(&self, user_id: UserId, id: PurchaseId) -> Result<(), AppError> {
        self.repository.delete(&user_id, id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::MockPurchaseRepository;
    use crate::application::service::purchase::test_fixtures::purchase_id;
    use crate::domain::error::FunctionalError;

    #[tokio::test]
    async fn delete_purchase_calls_the_repository_with_the_player_and_the_purchase() {
        let mut mock_repository = MockPurchaseRepository::new();
        mock_repository
            .expect_delete()
            .withf(|user, id| user.as_str() == "user_1" && *id == purchase_id())
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(()) }));

        let service = DeletePurchaseService::new(Arc::new(mock_repository));
        let result = service
            .delete_purchase(UserId::new("user_1"), purchase_id())
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn delete_purchase_propagates_purchase_not_found() {
        let mut mock_repository = MockPurchaseRepository::new();
        mock_repository
            .expect_delete()
            .times(1)
            .returning(|_, _| Box::pin(async { Err(FunctionalError::PurchaseNotFound.into()) }));

        let service = DeletePurchaseService::new(Arc::new(mock_repository));
        let result = service
            .delete_purchase(UserId::new("user_1"), purchase_id())
            .await;

        match result.unwrap_err() {
            AppError::Functional(FunctionalError::PurchaseNotFound) => {}
            other => panic!("Expected PurchaseNotFound, got {other:?}"),
        }
    }
}
