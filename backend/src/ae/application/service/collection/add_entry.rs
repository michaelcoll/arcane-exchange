use crate::application::error::AppError;
use crate::application::repository::{CardPricesRefreshRequester, CollectionEntryRepository};
use crate::application::use_case::{AddCollectionEntryUseCase, CreateCardUseCase};
use crate::domain::binder::BinderId;
use crate::domain::card::CardId;
use crate::domain::collection_entry::{CardEntries, validate_quantity};
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct AddCollectionEntryService {
    create_card: Arc<dyn CreateCardUseCase>,
    repository: Arc<dyn CollectionEntryRepository>,
    card_prices_refresh: Arc<dyn CardPricesRefreshRequester>,
}

impl AddCollectionEntryService {
    pub fn new(
        create_card: Arc<dyn CreateCardUseCase>,
        repository: Arc<dyn CollectionEntryRepository>,
        card_prices_refresh: Arc<dyn CardPricesRefreshRequester>,
    ) -> Self {
        Self {
            create_card,
            repository,
            card_prices_refresh,
        }
    }
}

#[async_trait]
impl AddCollectionEntryUseCase for AddCollectionEntryService {
    #[tracing::instrument(skip(self))]
    async fn add_collection_entry(
        &self,
        user_id: &UserId,
        card_id: CardId,
        foil: bool,
        quantity: i32,
        binder_id: Option<BinderId>,
    ) -> Result<CardEntries, AppError> {
        let quantity = validate_quantity(quantity)?;

        let card_id = self.create_card.create_card(card_id).await?;
        let entries = self
            .repository
            .add(user_id, &card_id, foil, quantity, binder_id)
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
    use crate::application::service::collection::test_fixtures::refresh;
    use crate::application::use_case::MockCreateCardUseCase;
    use crate::domain::error::FunctionalError;
    use crate::domain::language_code::LanguageCode;
    use uuid::Uuid;

    /// A card creation that hands back the card id with its collector number normalized.
    fn creating_card(times: usize) -> MockCreateCardUseCase {
        let mut create_card = MockCreateCardUseCase::new();
        create_card
            .expect_create_card()
            .times(times)
            .returning(|card_id| {
                Box::pin(async move {
                    Ok(CardId::new(
                        card_id.set_code,
                        card_id.collector_number.trim_start_matches('0').to_string(),
                        card_id.language_code,
                    ))
                })
            });
        create_card
    }

    fn service(
        create_card: MockCreateCardUseCase,
        repository: MockCollectionEntryRepository,
        refresh: MockCardPricesRefreshRequester,
    ) -> AddCollectionEntryService {
        AddCollectionEntryService::new(
            Arc::new(create_card),
            Arc::new(repository),
            Arc::new(refresh),
        )
    }

    #[tokio::test]
    async fn creates_the_card_adds_the_copies_and_requests_a_refresh_once() {
        let binder_id = BinderId(Uuid::from_u128(5));
        let mut repository = MockCollectionEntryRepository::new();
        repository
            .expect_add()
            .withf(move |user_id, card_id, foil, quantity, binder| {
                user_id.as_str() == "user_1"
                    // The id the card creation returned, not the one asked for.
                    && card_id == &CardId::new("MKM", "12", LanguageCode::JA)
                    && *foil
                    && *quantity == 4
                    && *binder == Some(binder_id)
            })
            .times(1)
            .returning(|_, _, _, _, _| Box::pin(async { Ok(CardEntries::default()) }));
        let service = service(creating_card(1), repository, refresh(1));

        let result = service
            .add_collection_entry(
                &UserId::new("user_1"),
                CardId::new("MKM", "012", LanguageCode::JA),
                true,
                4,
                Some(binder_id),
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn refuses_a_quantity_out_of_bounds_without_creating_the_card() {
        for quantity in [-1, 0, 256] {
            let service = service(
                creating_card(0),
                MockCollectionEntryRepository::new(),
                refresh(0),
            );

            let error = service
                .add_collection_entry(
                    &UserId::new("user_1"),
                    CardId::new("MKM", "12", LanguageCode::FR),
                    false,
                    quantity,
                    None,
                )
                .await
                .unwrap_err();

            assert_eq!(error.code(), "invalid_quantity");
        }
    }

    #[tokio::test]
    async fn a_card_the_catalog_does_not_have_is_refused_and_nothing_is_added() {
        let mut create_card = MockCreateCardUseCase::new();
        create_card
            .expect_create_card()
            .returning(|_| Box::pin(async { Err(FunctionalError::CardNotInCatalog.into()) }));
        let mut repository = MockCollectionEntryRepository::new();
        repository.expect_add().never();
        let service = service(create_card, repository, refresh(0));

        let error = service
            .add_collection_entry(
                &UserId::new("user_1"),
                CardId::new("MKM", "999", LanguageCode::FR),
                false,
                1,
                None,
            )
            .await
            .unwrap_err();

        assert_eq!(error.code(), "card_not_in_catalog");
    }

    #[tokio::test]
    async fn a_failed_add_requests_no_refresh_and_keeps_its_error() {
        let mut repository = MockCollectionEntryRepository::new();
        repository.expect_add().returning(|_, _, _, _, _| {
            Box::pin(async { Err(FunctionalError::BinderNotFound.into()) })
        });
        let service = service(creating_card(1), repository, refresh(0));

        let error = service
            .add_collection_entry(
                &UserId::new("user_1"),
                CardId::new("MKM", "12", LanguageCode::FR),
                false,
                1,
                Some(BinderId(Uuid::from_u128(9))),
            )
            .await
            .unwrap_err();

        assert_eq!(error.code(), "binder_not_found");
    }
}
