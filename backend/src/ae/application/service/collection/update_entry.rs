use crate::application::error::AppError;
use crate::application::repository::{CardPricesRefreshRequester, CollectionEntryRepository};
use crate::application::use_case::UpdateCollectionEntryUseCase;
use crate::domain::collection_entry::{
    CardEntries, CollectionEntryId, EntryUpdate, validate_quantity,
};
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct UpdateCollectionEntryService {
    repository: Arc<dyn CollectionEntryRepository>,
    card_prices_refresh: Arc<dyn CardPricesRefreshRequester>,
}

impl UpdateCollectionEntryService {
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
impl UpdateCollectionEntryUseCase for UpdateCollectionEntryService {
    #[tracing::instrument(skip(self))]
    async fn update_collection_entry(
        &self,
        user_id: &UserId,
        id: CollectionEntryId,
        quantity: Option<i32>,
        foil: Option<bool>,
    ) -> Result<CardEntries, AppError> {
        let update = EntryUpdate {
            quantity: quantity.map(validate_quantity).transpose()?,
            foil,
        };

        let entries = self.repository.update(user_id, id, update).await?;
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

    fn service(
        repository: MockCollectionEntryRepository,
        refresh: MockCardPricesRefreshRequester,
    ) -> UpdateCollectionEntryService {
        UpdateCollectionEntryService::new(Arc::new(repository), Arc::new(refresh))
    }

    #[tokio::test]
    async fn applies_the_update_and_requests_a_refresh_once() {
        let mut repository = MockCollectionEntryRepository::new();
        repository
            .expect_update()
            .withf(|user_id, id, update| {
                user_id.as_str() == "user_1"
                    && *id == entry_id()
                    && *update
                        == EntryUpdate {
                            quantity: Some(3),
                            foil: Some(true),
                        }
            })
            .times(1)
            .returning(|_, _, _| Box::pin(async { Ok(CardEntries::default()) }));
        let service = service(repository, refresh(1));

        let result = service
            .update_collection_entry(&UserId::new("user_1"), entry_id(), Some(3), Some(true))
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn refuses_a_quantity_out_of_bounds_without_touching_anything() {
        for quantity in [-1, 0, 256] {
            let service = service(MockCollectionEntryRepository::new(), refresh(0));

            let error = service
                .update_collection_entry(&UserId::new("user_1"), entry_id(), Some(quantity), None)
                .await
                .unwrap_err();

            assert_eq!(error.code(), "invalid_quantity");
        }
    }

    #[tokio::test]
    async fn a_failed_update_requests_no_refresh_and_keeps_its_error() {
        let mut repository = MockCollectionEntryRepository::new();
        repository.expect_update().returning(|_, _, _| {
            Box::pin(async { Err(FunctionalError::ReservedQuantityExceeded.into()) })
        });
        let service = service(repository, refresh(0));

        let error = service
            .update_collection_entry(&UserId::new("user_1"), entry_id(), Some(1), None)
            .await
            .unwrap_err();

        assert_eq!(error.code(), "reserved_quantity_exceeded");
    }
}
