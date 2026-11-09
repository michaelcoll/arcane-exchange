use crate::application::error::AppError;
use crate::application::repository::BinderRepository;
use crate::application::use_case::ListBindersUseCase;
use crate::domain::binder::Binder;
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct ListBindersService {
    repository: Arc<dyn BinderRepository>,
}

impl ListBindersService {
    pub fn new(repository: Arc<dyn BinderRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl ListBindersUseCase for ListBindersService {
    async fn list_binders(&self, user_id: UserId) -> Result<Vec<Binder>, AppError> {
        self.repository.list(&user_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::MockBinderRepository;
    use crate::domain::binder::BinderId;

    #[tokio::test]
    async fn list_binders_returns_the_binders_of_the_player() {
        let mut mock_repository = MockBinderRepository::new();
        mock_repository
            .expect_list()
            .withf(|id| id.as_str() == "user_1")
            .times(1)
            .returning(|_| {
                Box::pin(async {
                    Ok(vec![Binder {
                        id: BinderId(uuid::Uuid::nil()),
                        name: "Trade".to_string(),
                        copies: 0,
                        is_open: false,
                    }])
                })
            });

        let service = ListBindersService::new(Arc::new(mock_repository));
        let binders = service.list_binders(UserId::new("user_1")).await.unwrap();

        assert_eq!(binders.len(), 1);
        assert_eq!(binders[0].name, "Trade");
    }
}
