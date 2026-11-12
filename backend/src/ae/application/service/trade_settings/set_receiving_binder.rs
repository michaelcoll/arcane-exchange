use crate::application::error::AppError;
use crate::application::repository::{BinderRepository, ReceivingBinderRepository};
use crate::application::use_case::SetReceivingBinderUseCase;
use crate::domain::binder::{BinderId, ReceivingBinder};
use crate::domain::error::FunctionalError;
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct SetReceivingBinderService {
    repository: Arc<dyn ReceivingBinderRepository>,
    binder_repository: Arc<dyn BinderRepository>,
}

impl SetReceivingBinderService {
    pub fn new(
        repository: Arc<dyn ReceivingBinderRepository>,
        binder_repository: Arc<dyn BinderRepository>,
    ) -> Self {
        Self {
            repository,
            binder_repository,
        }
    }
}

#[async_trait]
impl SetReceivingBinderUseCase for SetReceivingBinderService {
    async fn set_receiving_binder(
        &self,
        user_id: UserId,
        binder_id: Option<BinderId>,
    ) -> Result<Option<ReceivingBinder>, AppError> {
        if let Some(binder_id) = binder_id {
            let binders = self.binder_repository.list(&user_id).await?;
            if !binders.iter().any(|binder| binder.id == binder_id) {
                return Err(FunctionalError::BinderNotFound.into());
            }
        }

        self.repository.set(&user_id, binder_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::{MockBinderRepository, MockReceivingBinderRepository};
    use crate::domain::binder::Binder;
    use uuid::Uuid;

    fn binder_id() -> BinderId {
        BinderId(Uuid::from_u128(1))
    }

    fn binder(id: BinderId) -> Binder {
        Binder {
            id,
            name: "Reçus".to_string(),
            copies: 0,
            is_open: false,
        }
    }

    fn binders_of_the_player(binders: Vec<Binder>) -> MockBinderRepository {
        let mut binder_repository = MockBinderRepository::new();
        binder_repository
            .expect_list()
            .withf(|id| id.as_str() == "user_1")
            .times(1)
            .returning(move |_| {
                let binders = binders.clone();
                Box::pin(async { Ok(binders) })
            });
        binder_repository
    }

    #[tokio::test]
    async fn chooses_the_binder_and_returns_the_stored_setting() {
        let mut repository = MockReceivingBinderRepository::new();
        repository
            .expect_set()
            .withf(|id, binder| id.as_str() == "user_1" && *binder == Some(binder_id()))
            .times(1)
            .returning(|_, _| {
                Box::pin(async {
                    Ok(Some(ReceivingBinder {
                        id: binder_id(),
                        name: "Reçus".to_string(),
                    }))
                })
            });

        let service = SetReceivingBinderService::new(
            Arc::new(repository),
            Arc::new(binders_of_the_player(vec![binder(binder_id())])),
        );
        let result = service
            .set_receiving_binder(UserId::new("user_1"), Some(binder_id()))
            .await;

        assert_eq!(result.unwrap().unwrap().name, "Reçus");
    }

    #[tokio::test]
    async fn an_absent_binder_goes_back_to_the_default_without_looking_binders_up() {
        let mut repository = MockReceivingBinderRepository::new();
        repository
            .expect_set()
            .withf(|_, binder| binder.is_none())
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(None) }));

        let service = SetReceivingBinderService::new(
            Arc::new(repository),
            Arc::new(MockBinderRepository::new()),
        );
        let result = service
            .set_receiving_binder(UserId::new("user_1"), None)
            .await;

        assert_eq!(result.unwrap(), None);
    }

    #[tokio::test]
    async fn another_players_binder_is_binder_not_found_and_nothing_is_stored() {
        let own = BinderId(Uuid::from_u128(2));
        let service = SetReceivingBinderService::new(
            Arc::new(MockReceivingBinderRepository::new()),
            Arc::new(binders_of_the_player(vec![binder(own)])),
        );

        let result = service
            .set_receiving_binder(UserId::new("user_1"), Some(binder_id()))
            .await;

        assert_eq!(result.unwrap_err().code(), "binder_not_found");
    }
}
