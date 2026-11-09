use crate::application::error::AppError;
use crate::application::repository::{CardPricesRefreshRequester, CollectionEntryRepository};
use crate::application::use_case::RemoveCollectionEntryUseCase;
use crate::domain::collection_entry::CollectionEntryId;
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct RemoveCollectionEntryService {
    repository: Arc<dyn CollectionEntryRepository>,
    card_prices_refresh: Arc<dyn CardPricesRefreshRequester>,
}

impl RemoveCollectionEntryService {
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
impl RemoveCollectionEntryUseCase for RemoveCollectionEntryService {
    #[tracing::instrument(skip(self))]
    async fn remove_collection_entry(
        &self,
        user_id: &UserId,
        id: CollectionEntryId,
    ) -> Result<(), AppError> {
        self.repository.remove(user_id, id).await?;
        self.card_prices_refresh.request_card_prices_refresh();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::MockCollectionEntryRepository;
    use crate::application::service::collection::test_fixtures::{entry_id, refresh};
    use crate::domain::error::FunctionalError;

    #[tokio::test]
    async fn removes_the_entry_and_requests_a_refresh_once() {
        let mut repository = MockCollectionEntryRepository::new();
        repository
            .expect_remove()
            .withf(|user_id, id| user_id.as_str() == "user_1" && *id == entry_id())
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(()) }));
        let service = RemoveCollectionEntryService::new(Arc::new(repository), Arc::new(refresh(1)));

        let result = service
            .remove_collection_entry(&UserId::new("user_1"), entry_id())
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn a_refused_removal_requests_no_refresh_and_keeps_its_error() {
        let mut repository = MockCollectionEntryRepository::new();
        repository.expect_remove().returning(|_, _| {
            Box::pin(async { Err(FunctionalError::CollectionEntryNotFound.into()) })
        });
        let service = RemoveCollectionEntryService::new(Arc::new(repository), Arc::new(refresh(0)));

        let error = service
            .remove_collection_entry(&UserId::new("user_1"), entry_id())
            .await
            .unwrap_err();

        assert_eq!(error.code(), "collection_entry_not_found");
    }
}
