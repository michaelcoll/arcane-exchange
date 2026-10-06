use crate::application::error::AppError;
use crate::application::repository::UserRepository;
use crate::application::use_case::RegisterUserUseCase;
use crate::domain::user::User;
use async_trait::async_trait;
use std::sync::Arc;

pub struct RegisterUserService {
    repository: Arc<dyn UserRepository>,
}

impl RegisterUserService {
    pub fn new(repository: Arc<dyn UserRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl RegisterUserUseCase for RegisterUserService {
    async fn register_user(&self, user: &User) -> Result<(), AppError> {
        self.repository.upsert(user).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::error::InfraError;
    use crate::application::repository::MockUserRepository;

    fn make_user() -> User {
        User::new(
            "user_clerk123".to_string(),
            None,
            Some("testuser".to_string()),
            None,
        )
    }

    #[tokio::test]
    async fn register_user_calls_repository_upsert() {
        let mut mock_repository = MockUserRepository::new();
        mock_repository
            .expect_upsert()
            .times(1)
            .returning(|_| Box::pin(async { Ok(()) }));

        let service = RegisterUserService::new(Arc::new(mock_repository));
        let result = service.register_user(&make_user()).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn register_user_returns_error_on_repository_error() {
        let mut mock_repository = MockUserRepository::new();
        mock_repository.expect_upsert().times(1).returning(|_| {
            Box::pin(async {
                Err(AppError::Infra(InfraError::RepositoryError(
                    "DB error".to_string(),
                )))
            })
        });

        let service = RegisterUserService::new(Arc::new(mock_repository));
        let result = service.register_user(&make_user()).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Infra(InfraError::RepositoryError(msg)) => assert_eq!(msg, "DB error"),
            _ => panic!("Expected RepositoryError"),
        }
    }
}
