use crate::application::error::AppError;
use crate::application::repository::UserRepository;
use crate::application::use_case::SetCollectionVisibilityUseCase;
use crate::domain::error::FunctionalError;
use crate::domain::user::{CollectionVisibility, UserId};
use async_trait::async_trait;
use std::sync::Arc;

pub struct SetCollectionVisibilityService {
    repository: Arc<dyn UserRepository>,
}

impl SetCollectionVisibilityService {
    pub fn new(repository: Arc<dyn UserRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl SetCollectionVisibilityUseCase for SetCollectionVisibilityService {
    async fn set_visibility(
        &self,
        user_id: UserId,
        visibility: CollectionVisibility,
    ) -> Result<(), AppError> {
        let updated = self.repository.set_visibility(&user_id, visibility).await?;
        if updated {
            Ok(())
        } else {
            Err(FunctionalError::UserNotFound.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::error::InfraError;
    use crate::application::repository::MockUserRepository;

    #[tokio::test]
    async fn set_visibility_calls_repository_with_id_and_value() {
        let mut mock_repository = MockUserRepository::new();
        mock_repository
            .expect_set_visibility()
            .withf(|id, visibility| {
                id.as_str() == "user_1" && *visibility == CollectionVisibility::Public
            })
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(true) }));

        let service = SetCollectionVisibilityService::new(Arc::new(mock_repository));
        let result = service
            .set_visibility(UserId::new("user_1"), CollectionVisibility::Public)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn set_visibility_fails_with_user_not_found_when_repository_returns_false() {
        let mut mock_repository = MockUserRepository::new();
        mock_repository
            .expect_set_visibility()
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(false) }));

        let service = SetCollectionVisibilityService::new(Arc::new(mock_repository));
        let result = service
            .set_visibility(UserId::new("user_1"), CollectionVisibility::Public)
            .await;

        match result.unwrap_err() {
            AppError::Functional(FunctionalError::UserNotFound) => {}
            other => panic!("Expected UserNotFound, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn set_visibility_propagates_repository_error() {
        let mut mock_repository = MockUserRepository::new();
        mock_repository
            .expect_set_visibility()
            .times(1)
            .returning(|_, _| {
                Box::pin(async {
                    Err(AppError::Infra(InfraError::RepositoryError(
                        "DB error".to_string(),
                    )))
                })
            });

        let service = SetCollectionVisibilityService::new(Arc::new(mock_repository));
        let result = service
            .set_visibility(UserId::new("user_1"), CollectionVisibility::Public)
            .await;

        match result.unwrap_err() {
            AppError::Infra(InfraError::RepositoryError(msg)) => assert_eq!(msg, "DB error"),
            other => panic!("Expected RepositoryError, got {other:?}"),
        }
    }
}
