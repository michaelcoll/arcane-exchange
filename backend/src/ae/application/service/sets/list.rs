use crate::application::error::AppError;
use crate::application::repository::SetNameRepository;
use crate::application::use_case::ListSetsUseCase;
use crate::domain::set_name::SetName;
use async_trait::async_trait;
use std::sync::Arc;

pub struct ListSetsService {
    repository: Arc<dyn SetNameRepository>,
}

impl ListSetsService {
    pub fn new(repository: Arc<dyn SetNameRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl ListSetsUseCase for ListSetsService {
    async fn list_sets(&self) -> Result<Vec<SetName>, AppError> {
        self.repository.find_all().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::error::InfraError;
    use crate::application::repository::MockSetNameRepository;
    use crate::domain::set_name::SetCode;

    #[tokio::test]
    async fn list_sets_returns_repository_sets() {
        let mut mock = MockSetNameRepository::new();
        mock.expect_find_all().times(1).returning(|| {
            Box::pin(async {
                Ok(vec![SetName {
                    code: SetCode::new("ECL"),
                    name: "Eclipsed".to_string(),
                }])
            })
        });

        let service = ListSetsService::new(Arc::new(mock));
        let result = service.list_sets().await.unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "Eclipsed");
    }

    #[tokio::test]
    async fn list_sets_propagates_repository_error() {
        let mut mock = MockSetNameRepository::new();
        mock.expect_find_all().returning(|| {
            Box::pin(async {
                Err(AppError::Infra(InfraError::RepositoryError(
                    "db error".to_string(),
                )))
            })
        });

        let service = ListSetsService::new(Arc::new(mock));
        let result = service.list_sets().await;

        assert!(result.is_err());
    }
}
