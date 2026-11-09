use crate::application::error::AppError;
use crate::application::repository::BinderRepository;
use crate::application::use_case::CreateBinderUseCase;
use crate::domain::binder::{Binder, BinderName};
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct CreateBinderService {
    repository: Arc<dyn BinderRepository>,
}

impl CreateBinderService {
    pub fn new(repository: Arc<dyn BinderRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl CreateBinderUseCase for CreateBinderService {
    async fn create_binder(&self, user_id: UserId, name: &str) -> Result<Binder, AppError> {
        let name = BinderName::try_new(name)?;

        self.repository.create(&user_id, &name).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::MockBinderRepository;
    use crate::domain::binder::BinderId;
    use crate::domain::error::FunctionalError;

    #[tokio::test]
    async fn create_binder_creates_it_with_the_trimmed_name() {
        let mut mock_repository = MockBinderRepository::new();
        mock_repository
            .expect_create()
            .withf(|id, name| id.as_str() == "user_1" && name.as_str() == "Trade")
            .times(1)
            .returning(|_, name| {
                let name = name.as_str().to_string();
                Box::pin(async move {
                    Ok(Binder {
                        id: BinderId(uuid::Uuid::nil()),
                        name,
                        copies: 0,
                        is_open: false,
                    })
                })
            });

        let service = CreateBinderService::new(Arc::new(mock_repository));
        let binder = service
            .create_binder(UserId::new("user_1"), "  Trade ")
            .await
            .unwrap();

        assert_eq!(binder.name, "Trade");
        assert!(!binder.is_open);
    }

    #[tokio::test]
    async fn create_binder_refuses_a_blank_name_without_touching_the_repository() {
        let service = CreateBinderService::new(Arc::new(MockBinderRepository::new()));

        let result = service.create_binder(UserId::new("user_1"), "   ").await;

        match result.unwrap_err() {
            AppError::Functional(FunctionalError::InvalidBinderName) => {}
            other => panic!("Expected InvalidBinderName, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn create_binder_propagates_a_taken_name() {
        let mut mock_repository = MockBinderRepository::new();
        mock_repository
            .expect_create()
            .times(1)
            .returning(|_, _| Box::pin(async { Err(FunctionalError::BinderNameTaken.into()) }));

        let service = CreateBinderService::new(Arc::new(mock_repository));
        let result = service.create_binder(UserId::new("user_1"), "Trade").await;

        match result.unwrap_err() {
            AppError::Functional(FunctionalError::BinderNameTaken) => {}
            other => panic!("Expected BinderNameTaken, got {other:?}"),
        }
    }
}
