use crate::application::error::AppError;
use crate::application::repository::UserRepository;
use crate::application::use_case::GetUserProfileUseCase;
use crate::domain::error::FunctionalError;
use crate::domain::user::User;
use async_trait::async_trait;
use std::sync::Arc;

pub struct GetUserProfileService {
    repository: Arc<dyn UserRepository>,
}

impl GetUserProfileService {
    pub fn new(repository: Arc<dyn UserRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl GetUserProfileUseCase for GetUserProfileService {
    async fn get_user_profile(&self, username: &str) -> Result<User, AppError> {
        self.repository
            .find_by_username(username)
            .await?
            .ok_or(FunctionalError::UserNotFound.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::error::InfraError;
    use crate::application::repository::MockUserRepository;
    use mockall::predicate::eq;

    fn make_user() -> User {
        User::new(
            "user_clerk123".to_string(),
            None,
            Some("alice".to_string()),
            Some("https://img.example.com/avatar.png".to_string()),
        )
    }

    #[tokio::test]
    async fn get_user_profile_returns_user_when_found() {
        let mut mock_repository = MockUserRepository::new();
        let user = make_user();
        mock_repository
            .expect_find_by_username()
            .times(1)
            .with(eq("alice"))
            .returning(move |_| {
                let value = user.clone();
                Box::pin(async move { Ok(Some(value)) })
            });

        let service = GetUserProfileService::new(Arc::new(mock_repository));
        let result = service.get_user_profile("alice").await;

        assert_eq!(result.unwrap(), make_user());
    }

    #[tokio::test]
    async fn get_user_profile_returns_user_not_found_when_absent() {
        let mut mock_repository = MockUserRepository::new();
        mock_repository
            .expect_find_by_username()
            .times(1)
            .with(eq("nobody"))
            .returning(|_| Box::pin(async { Ok(None) }));

        let service = GetUserProfileService::new(Arc::new(mock_repository));
        let result = service.get_user_profile("nobody").await;

        assert!(matches!(
            result.unwrap_err(),
            AppError::Functional(FunctionalError::UserNotFound)
        ));
    }

    #[tokio::test]
    async fn get_user_profile_propagates_repository_error() {
        let mut mock_repository = MockUserRepository::new();
        mock_repository
            .expect_find_by_username()
            .times(1)
            .with(eq("alice"))
            .returning(|_| {
                Box::pin(async {
                    Err(AppError::Infra(InfraError::RepositoryError(
                        "DB error".to_string(),
                    )))
                })
            });

        let service = GetUserProfileService::new(Arc::new(mock_repository));
        let result = service.get_user_profile("alice").await;

        assert!(matches!(
            result.unwrap_err(),
            AppError::Infra(InfraError::RepositoryError(_))
        ));
    }
}
