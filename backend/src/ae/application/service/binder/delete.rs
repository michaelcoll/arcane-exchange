use crate::application::error::AppError;
use crate::application::repository::BinderRepository;
use crate::application::use_case::DeleteBinderUseCase;
use crate::domain::binder::BinderId;
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct DeleteBinderService {
    repository: Arc<dyn BinderRepository>,
}

impl DeleteBinderService {
    pub fn new(repository: Arc<dyn BinderRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl DeleteBinderUseCase for DeleteBinderService {
    async fn delete_binder(&self, user_id: UserId, id: BinderId) -> Result<(), AppError> {
        self.repository.delete(&user_id, id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::MockBinderRepository;
    use crate::domain::error::FunctionalError;

    #[tokio::test]
    async fn delete_binder_calls_the_repository_with_the_player_and_the_binder() {
        let mut mock_repository = MockBinderRepository::new();
        mock_repository
            .expect_delete()
            .withf(|user, id| user.as_str() == "user_1" && *id == BinderId(uuid::Uuid::nil()))
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(()) }));

        let service = DeleteBinderService::new(Arc::new(mock_repository));
        let result = service
            .delete_binder(UserId::new("user_1"), BinderId(uuid::Uuid::nil()))
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn delete_binder_propagates_binder_not_empty() {
        let mut mock_repository = MockBinderRepository::new();
        mock_repository
            .expect_delete()
            .times(1)
            .returning(|_, _| Box::pin(async { Err(FunctionalError::BinderNotEmpty.into()) }));

        let service = DeleteBinderService::new(Arc::new(mock_repository));
        let result = service
            .delete_binder(UserId::new("user_1"), BinderId(uuid::Uuid::nil()))
            .await;

        match result.unwrap_err() {
            AppError::Functional(FunctionalError::BinderNotEmpty) => {}
            other => panic!("Expected BinderNotEmpty, got {other:?}"),
        }
    }
}
