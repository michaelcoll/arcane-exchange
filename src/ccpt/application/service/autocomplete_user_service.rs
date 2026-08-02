use crate::application::error::AppError;
use crate::application::repository::UserRepository;
use crate::application::use_case::AutocompleteUsersUseCase;
use crate::domain::user::UserSuggestion;
use async_trait::async_trait;
use std::sync::Arc;

const AUTOCOMPLETE_LIMIT: i64 = 10;

pub struct AutocompleteUserService {
    repository: Arc<dyn UserRepository>,
}

impl AutocompleteUserService {
    pub fn new(repository: Arc<dyn UserRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl AutocompleteUsersUseCase for AutocompleteUserService {
    async fn autocomplete(&self, query: Option<String>) -> Result<Vec<UserSuggestion>, AppError> {
        let trimmed = query.as_deref().map(str::trim).unwrap_or("");
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }

        self.repository
            .autocomplete(trimmed, AUTOCOMPLETE_LIMIT)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::error::InfraError;
    use crate::application::repository::MockUserRepository;

    #[tokio::test]
    async fn returns_empty_without_calling_repository_when_query_is_none() {
        // No `expect_autocomplete()` set up: any call to the repository would panic.
        let mock = MockUserRepository::new();
        let service = AutocompleteUserService::new(Arc::new(mock));

        let result = service.autocomplete(None).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn returns_empty_without_calling_repository_when_query_is_blank() {
        let mock = MockUserRepository::new();
        let service = AutocompleteUserService::new(Arc::new(mock));

        let result = service.autocomplete(Some("   ".to_string())).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn delegates_trimmed_query_and_fixed_limit_to_repository() {
        let mut mock = MockUserRepository::new();
        mock.expect_autocomplete()
            .withf(|q, limit| q == "ali" && *limit == 10)
            .returning(|_, _| {
                Box::pin(async {
                    Ok(vec![UserSuggestion {
                        username: "alice".to_string(),
                        card_count: 3,
                    }])
                })
            });

        let service = AutocompleteUserService::new(Arc::new(mock));
        let result = service.autocomplete(Some("  ali  ".to_string())).await;

        assert!(result.is_ok());
        let suggestions = result.unwrap();
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].username, "alice");
    }

    #[tokio::test]
    async fn propagates_repository_error() {
        let mut mock = MockUserRepository::new();
        mock.expect_autocomplete().returning(|_, _| {
            Box::pin(async {
                Err(AppError::Infra(InfraError::RepositoryError(
                    "db error".to_string(),
                )))
            })
        });

        let service = AutocompleteUserService::new(Arc::new(mock));
        let result = service.autocomplete(Some("ali".to_string())).await;

        assert!(result.is_err());
    }
}
