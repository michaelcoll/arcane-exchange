use crate::application::error::AppError;
use crate::application::repository::{CardPricesRefreshRequester, CollectionEntryRepository};
use crate::application::use_case::MoveCollectionEntryUseCase;
use crate::domain::binder::BinderId;
use crate::domain::collection_entry::{CardEntries, CollectionEntryId, validate_quantity};
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct MoveCollectionEntryService {
    repository: Arc<dyn CollectionEntryRepository>,
    card_prices_refresh: Arc<dyn CardPricesRefreshRequester>,
}

impl MoveCollectionEntryService {
    pub fn new(
        repository: Arc<dyn CollectionEntryRepository>,
        card_prices_refresh: Arc<dyn CardPricesRefreshRequester>,
    ) -> Self {
        Self {
            repository,
            card_prices_refresh,
        }
    }
}

#[async_trait]
impl MoveCollectionEntryUseCase for MoveCollectionEntryService {
    #[tracing::instrument(skip(self))]
    async fn move_collection_entry(
        &self,
        user_id: &UserId,
        id: CollectionEntryId,
        binder_id: Option<BinderId>,
        quantity: Option<i32>,
    ) -> Result<CardEntries, AppError> {
        let quantity = quantity.map(validate_quantity).transpose()?;

        let entries = self
            .repository
            .move_to_binder(user_id, id, binder_id, quantity)
            .await?;
        self.card_prices_refresh.request_card_prices_refresh();

        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::{
        MockCardPricesRefreshRequester, MockCollectionEntryRepository,
    };
    use crate::application::service::collection::test_fixtures::{entry_id, refresh};
    use crate::domain::error::FunctionalError;
    use uuid::Uuid;

    fn binder_id() -> BinderId {
        BinderId(Uuid::from_u128(2))
    }

    fn service(
        repository: MockCollectionEntryRepository,
        refresh: MockCardPricesRefreshRequester,
    ) -> MoveCollectionEntryService {
        MoveCollectionEntryService::new(Arc::new(repository), Arc::new(refresh))
    }

    #[tokio::test]
    async fn moves_and_requests_a_refresh_once() {
        let mut repository = MockCollectionEntryRepository::new();
        repository
            .expect_move_to_binder()
            .withf(|user_id, id, binder, quantity| {
                user_id.as_str() == "user_1"
                    && *id == entry_id()
                    && *binder == Some(binder_id())
                    && *quantity == Some(2)
            })
            .times(1)
            .returning(|_, _, _, _| Box::pin(async { Ok(CardEntries::default()) }));
        let service = service(repository, refresh(1));

        let result = service
            .move_collection_entry(
                &UserId::new("user_1"),
                entry_id(),
                Some(binder_id()),
                Some(2),
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn moves_every_copy_out_of_any_binder_by_default() {
        let mut repository = MockCollectionEntryRepository::new();
        repository
            .expect_move_to_binder()
            .withf(|_, _, binder, quantity| binder.is_none() && quantity.is_none())
            .times(1)
            .returning(|_, _, _, _| Box::pin(async { Ok(CardEntries::default()) }));
        let service = service(repository, refresh(1));

        let result = service
            .move_collection_entry(&UserId::new("user_1"), entry_id(), None, None)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn refuses_a_quantity_out_of_bounds_without_touching_anything() {
        for quantity in [-1, 0, 256] {
            let service = service(MockCollectionEntryRepository::new(), refresh(0));

            let error = service
                .move_collection_entry(&UserId::new("user_1"), entry_id(), None, Some(quantity))
                .await
                .unwrap_err();

            assert_eq!(error.code(), "invalid_quantity");
        }
    }

    #[tokio::test]
    async fn a_failed_move_requests_no_refresh_and_keeps_its_error() {
        let mut repository = MockCollectionEntryRepository::new();
        repository.expect_move_to_binder().returning(|_, _, _, _| {
            Box::pin(async { Err(FunctionalError::BinderNotFound.into()) })
        });
        let service = service(repository, refresh(0));

        let error = service
            .move_collection_entry(&UserId::new("user_1"), entry_id(), Some(binder_id()), None)
            .await
            .unwrap_err();

        assert_eq!(error.code(), "binder_not_found");
    }
}
