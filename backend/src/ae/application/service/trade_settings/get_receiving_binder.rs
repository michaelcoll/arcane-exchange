use crate::application::error::AppError;
use crate::application::repository::ReceivingBinderRepository;
use crate::application::use_case::GetReceivingBinderUseCase;
use crate::domain::binder::ReceivingBinder;
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct GetReceivingBinderService {
    repository: Arc<dyn ReceivingBinderRepository>,
}

impl GetReceivingBinderService {
    pub fn new(repository: Arc<dyn ReceivingBinderRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl GetReceivingBinderUseCase for GetReceivingBinderService {
    async fn get_receiving_binder(
        &self,
        user_id: UserId,
    ) -> Result<Option<ReceivingBinder>, AppError> {
        self.repository.find(&user_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::error::InfraError;
    use crate::application::repository::MockReceivingBinderRepository;
    use crate::domain::binder::BinderId;
    use uuid::Uuid;

    #[tokio::test]
    async fn returns_the_chosen_binder() {
        let binder = ReceivingBinder {
            id: BinderId(Uuid::from_u128(1)),
            name: "Reçus".to_string(),
        };
        let expected = binder.clone();
        let mut repository = MockReceivingBinderRepository::new();
        repository
            .expect_find()
            .withf(|id| id.as_str() == "user_1")
            .times(1)
            .returning(move |_| {
                let binder = binder.clone();
                Box::pin(async move { Ok(Some(binder)) })
            });

        let service = GetReceivingBinderService::new(Arc::new(repository));
        let result = service.get_receiving_binder(UserId::new("user_1")).await;

        assert_eq!(result.unwrap(), Some(expected));
    }

    #[tokio::test]
    async fn returns_none_when_no_binder_is_chosen() {
        let mut repository = MockReceivingBinderRepository::new();
        repository
            .expect_find()
            .times(1)
            .returning(|_| Box::pin(async { Ok(None) }));

        let service = GetReceivingBinderService::new(Arc::new(repository));
        let result = service.get_receiving_binder(UserId::new("user_1")).await;

        assert_eq!(result.unwrap(), None);
    }

    #[tokio::test]
    async fn propagates_repository_error() {
        let mut repository = MockReceivingBinderRepository::new();
        repository.expect_find().times(1).returning(|_| {
            Box::pin(async {
                Err(AppError::Infra(InfraError::RepositoryError(
                    "DB error".to_string(),
                )))
            })
        });

        let service = GetReceivingBinderService::new(Arc::new(repository));
        let result = service.get_receiving_binder(UserId::new("user_1")).await;

        match result.unwrap_err() {
            AppError::Infra(InfraError::RepositoryError(msg)) => assert_eq!(msg, "DB error"),
            other => panic!("Expected RepositoryError, got {other:?}"),
        }
    }
}
