use crate::application::error::AppError;
use crate::application::repository::BinderRepository;
use crate::application::use_case::RenameBinderUseCase;
use crate::domain::binder::{Binder, BinderId, BinderName};
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct RenameBinderService {
    repository: Arc<dyn BinderRepository>,
}

impl RenameBinderService {
    pub fn new(repository: Arc<dyn BinderRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl RenameBinderUseCase for RenameBinderService {
    async fn rename_binder(
        &self,
        user_id: UserId,
        id: BinderId,
        name: &str,
    ) -> Result<Binder, AppError> {
        let name = BinderName::try_new(name)?;

        self.repository.rename(&user_id, id, &name).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::MockBinderRepository;
    use crate::domain::error::FunctionalError;

    fn binder_id() -> BinderId {
        BinderId(uuid::Uuid::nil())
    }

    #[tokio::test]
    async fn rename_binder_renames_it_with_the_trimmed_name() {
        let mut mock_repository = MockBinderRepository::new();
        mock_repository
            .expect_rename()
            .withf(|user, id, name| {
                user.as_str() == "user_1" && *id == binder_id() && name.as_str() == "Bulk"
            })
            .times(1)
            .returning(|_, id, name| {
                let name = name.as_str().to_string();
                Box::pin(async move {
                    Ok(Binder {
                        id,
                        name,
                        copies: 3,
                        is_open: true,
                    })
                })
            });

        let service = RenameBinderService::new(Arc::new(mock_repository));
        let binder = service
            .rename_binder(UserId::new("user_1"), binder_id(), " Bulk ")
            .await
            .unwrap();

        assert_eq!(binder.name, "Bulk");
    }

    #[tokio::test]
    async fn rename_binder_refuses_an_empty_name_without_touching_the_repository() {
        let service = RenameBinderService::new(Arc::new(MockBinderRepository::new()));

        let result = service
            .rename_binder(UserId::new("user_1"), binder_id(), "")
            .await;

        match result.unwrap_err() {
            AppError::Functional(FunctionalError::InvalidBinderName) => {}
            other => panic!("Expected InvalidBinderName, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn rename_binder_propagates_binder_not_found() {
        let mut mock_repository = MockBinderRepository::new();
        mock_repository
            .expect_rename()
            .times(1)
            .returning(|_, _, _| Box::pin(async { Err(FunctionalError::BinderNotFound.into()) }));

        let service = RenameBinderService::new(Arc::new(mock_repository));
        let result = service
            .rename_binder(UserId::new("user_1"), binder_id(), "Bulk")
            .await;

        match result.unwrap_err() {
            AppError::Functional(FunctionalError::BinderNotFound) => {}
            other => panic!("Expected BinderNotFound, got {other:?}"),
        }
    }
}
