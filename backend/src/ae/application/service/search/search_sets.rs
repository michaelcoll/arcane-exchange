use crate::application::error::AppError;
use crate::application::repository::CardPricesViewRepository;
use crate::application::use_case::SearchSetsUseCase;
use crate::domain::collection::SearchScope;
use crate::domain::set_name::SetName;
use async_trait::async_trait;
use std::sync::Arc;

pub struct SearchSetsService {
    repository: Arc<dyn CardPricesViewRepository>,
}

impl SearchSetsService {
    pub fn new(repository: Arc<dyn CardPricesViewRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl SearchSetsUseCase for SearchSetsService {
    async fn search_sets(&self, scope: SearchScope) -> Result<Vec<SetName>, AppError> {
        self.repository.search_sets(&scope).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::error::InfraError;
    use crate::application::repository::MockCardPricesViewRepository;
    use crate::domain::set_name::SetCode;

    #[tokio::test]
    async fn search_sets_passes_the_scope_to_the_repository_and_returns_its_sets() {
        let mut mock_repo = MockCardPricesViewRepository::new();
        mock_repo
            .expect_search_sets()
            .withf(|scope| {
                scope.search_query == Some("bolt".to_string())
                    && scope.player_username == Some("alice".to_string())
            })
            .times(1)
            .returning(|_| {
                Box::pin(async {
                    Ok(vec![SetName {
                        code: SetCode::new("ECL"),
                        name: "Eclipsed".to_string(),
                    }])
                })
            });

        let service = SearchSetsService::new(Arc::new(mock_repo));
        let result = service
            .search_sets(SearchScope {
                search_query: Some("bolt".to_string()),
                player_username: Some("alice".to_string()),
            })
            .await
            .unwrap();

        assert_eq!(
            result,
            vec![SetName {
                code: SetCode::new("ECL"),
                name: "Eclipsed".to_string(),
            }]
        );
    }

    #[tokio::test]
    async fn search_sets_propagates_repository_error() {
        let mut mock_repo = MockCardPricesViewRepository::new();
        mock_repo.expect_search_sets().returning(|_| {
            Box::pin(async {
                Err(AppError::Infra(InfraError::RepositoryError(
                    "db error".to_string(),
                )))
            })
        });

        let service = SearchSetsService::new(Arc::new(mock_repo));
        let result = service.search_sets(SearchScope::default()).await;

        assert!(matches!(
            result,
            Err(AppError::Infra(InfraError::RepositoryError(_)))
        ));
    }
}
