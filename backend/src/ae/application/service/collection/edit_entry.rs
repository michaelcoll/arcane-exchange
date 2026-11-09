use crate::application::error::AppError;
use crate::application::repository::CollectionEntryRepository;
use crate::application::use_case::{EditCollectionEntryUseCase, RequestCardPricesRefreshUseCase};
use crate::domain::collection_entry::{
    CardEntries, CollectionEntryId, EntryEdit, validate_quantity,
};
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct EditCollectionEntryService {
    repository: Arc<dyn CollectionEntryRepository>,
    card_prices_refresh: Arc<dyn RequestCardPricesRefreshUseCase>,
}

impl EditCollectionEntryService {
    pub fn new(
        repository: Arc<dyn CollectionEntryRepository>,
        card_prices_refresh: Arc<dyn RequestCardPricesRefreshUseCase>,
    ) -> Self {
        Self {
            repository,
            card_prices_refresh,
        }
    }
}

#[async_trait]
impl EditCollectionEntryUseCase for EditCollectionEntryService {
    #[tracing::instrument(skip(self))]
    async fn edit_collection_entry(
        &self,
        user_id: &UserId,
        id: CollectionEntryId,
        quantity: Option<i32>,
        foil: Option<bool>,
    ) -> Result<CardEntries, AppError> {
        let edit = EntryEdit {
            quantity: quantity.map(validate_quantity).transpose()?,
            foil,
        };

        let entries = self.repository.update(user_id, id, edit).await?;
        self.card_prices_refresh.request_card_prices_refresh();

        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::MockCollectionEntryRepository;
    use crate::application::use_case::MockRequestCardPricesRefreshUseCase;
    use crate::domain::error::FunctionalError;
    use uuid::Uuid;

    fn entry_id() -> CollectionEntryId {
        CollectionEntryId(Uuid::from_u128(1))
    }

    fn refresh(times: usize) -> MockRequestCardPricesRefreshUseCase {
        let mut refresh = MockRequestCardPricesRefreshUseCase::new();
        refresh
            .expect_request_card_prices_refresh()
            .times(times)
            .return_const(());
        refresh
    }

    fn service(
        repository: MockCollectionEntryRepository,
        refresh: MockRequestCardPricesRefreshUseCase,
    ) -> EditCollectionEntryService {
        EditCollectionEntryService::new(Arc::new(repository), Arc::new(refresh))
    }

    #[tokio::test]
    async fn applies_the_edit_and_requests_a_refresh_once() {
        let mut repository = MockCollectionEntryRepository::new();
        repository
            .expect_update()
            .withf(|user_id, id, edit| {
                user_id.as_str() == "user_1"
                    && *id == entry_id()
                    && *edit
                        == EntryEdit {
                            quantity: Some(3),
                            foil: Some(true),
                        }
            })
            .times(1)
            .returning(|_, _, _| Box::pin(async { Ok(CardEntries::default()) }));
        let service = service(repository, refresh(1));

        let result = service
            .edit_collection_entry(&UserId::new("user_1"), entry_id(), Some(3), Some(true))
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn refuses_a_quantity_out_of_bounds_without_touching_anything() {
        for quantity in [-1, 0, 256] {
            let service = service(MockCollectionEntryRepository::new(), refresh(0));

            let error = service
                .edit_collection_entry(&UserId::new("user_1"), entry_id(), Some(quantity), None)
                .await
                .unwrap_err();

            assert_eq!(error.code(), "invalid_quantity");
        }
    }

    #[tokio::test]
    async fn a_failed_edit_requests_no_refresh_and_keeps_its_error() {
        let mut repository = MockCollectionEntryRepository::new();
        repository.expect_update().returning(|_, _, _| {
            Box::pin(async { Err(FunctionalError::ReservedQuantityExceeded.into()) })
        });
        let service = service(repository, refresh(0));

        let error = service
            .edit_collection_entry(&UserId::new("user_1"), entry_id(), Some(1), None)
            .await
            .unwrap_err();

        assert_eq!(error.code(), "reserved_quantity_exceeded");
    }
}
