use crate::application::error::AppError;
use crate::application::repository::CardImportRepository;
use crate::application::use_case::GetCardImportUseCase;
use crate::domain::card_import::{CardImport, CardImportId};
use crate::domain::error::FunctionalError;
use crate::domain::user::User;
use async_trait::async_trait;
use std::sync::Arc;

pub struct CardImportQueryService {
    card_import_repository: Arc<dyn CardImportRepository>,
}

impl CardImportQueryService {
    pub fn new(card_import_repository: Arc<dyn CardImportRepository>) -> Self {
        Self {
            card_import_repository,
        }
    }
}

#[async_trait]
impl GetCardImportUseCase for CardImportQueryService {
    async fn find(&self, id: &CardImportId, user: &User) -> Result<CardImport, AppError> {
        let import = self
            .card_import_repository
            .find_by_id(id)
            .await?
            .filter(|import| import.user_id == user.id)
            .ok_or(FunctionalError::ImportNotFound)?;

        Ok(import)
    }

    async fn list(&self, user: &User) -> Result<Vec<CardImport>, AppError> {
        self.card_import_repository.list_by_user(&user.id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::MockCardImportRepository;
    use crate::domain::card_import::CardImportStatus;
    use chrono::Utc;

    fn import_for(user_id: &str) -> CardImport {
        CardImport {
            id: CardImportId::new(),
            user_id: crate::domain::user::UserId::new(user_id),
            status: CardImportStatus::Completed,
            source_lines: 1,
            total_lines: 1,
            processed_lines: 1,
            line_errors: vec![],
            line_error_count: 0,
            error_message: None,
            created_at: Utc::now(),
            finished_at: Some(Utc::now()),
        }
    }

    #[tokio::test]
    async fn find_returns_not_found_when_import_belongs_to_another_user() {
        let import = import_for("someone-else");
        let import_id = import.id;

        let mut repository = MockCardImportRepository::new();
        repository.expect_find_by_id().returning(move |_| {
            let import = import.clone();
            Box::pin(async move { Ok(Some(import)) })
        });

        let service = CardImportQueryService::new(Arc::new(repository));

        let result = service.find(&import_id, &User::for_testing()).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::ImportNotFound))
        ));
    }

    #[tokio::test]
    async fn find_returns_not_found_when_import_does_not_exist() {
        let mut repository = MockCardImportRepository::new();
        repository
            .expect_find_by_id()
            .returning(|_| Box::pin(async { Ok(None) }));

        let service = CardImportQueryService::new(Arc::new(repository));

        let result = service
            .find(&CardImportId::new(), &User::for_testing())
            .await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::ImportNotFound))
        ));
    }

    #[tokio::test]
    async fn find_returns_the_import_when_it_belongs_to_the_caller() {
        let import = import_for(User::for_testing().id.as_str());
        let import_id = import.id;

        let mut repository = MockCardImportRepository::new();
        repository.expect_find_by_id().returning(move |_| {
            let import = import.clone();
            Box::pin(async move { Ok(Some(import)) })
        });

        let service = CardImportQueryService::new(Arc::new(repository));

        let result = service.find(&import_id, &User::for_testing()).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn list_delegates_to_the_repository() {
        let mut repository = MockCardImportRepository::new();
        repository
            .expect_list_by_user()
            .returning(|_| Box::pin(async { Ok(vec![]) }));

        let service = CardImportQueryService::new(Arc::new(repository));

        let result = service.list(&User::for_testing()).await;

        assert!(result.is_ok());
    }
}
