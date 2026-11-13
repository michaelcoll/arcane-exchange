use crate::application::error::AppError;
use crate::application::repository::{CardPricesRefreshRequester, CollectionEntryRepository};
use crate::application::use_case::{
    AddCollectionEntryUseCase, AttachCardToSealedProductUseCase, CreateCardUseCase,
};
use crate::domain::binder::BinderId;
use crate::domain::card::CardId;
use crate::domain::collection_entry::{CardEntries, validate_quantity};
use crate::domain::error::FunctionalError;
use crate::domain::purchase::{NewSingleCardPurchase, PurchaseAmount, PurchaseId};
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct AddCollectionEntryService {
    create_card: Arc<dyn CreateCardUseCase>,
    repository: Arc<dyn CollectionEntryRepository>,
    card_prices_refresh: Arc<dyn CardPricesRefreshRequester>,
    attach_card: Arc<dyn AttachCardToSealedProductUseCase>,
}

impl AddCollectionEntryService {
    pub fn new(
        create_card: Arc<dyn CreateCardUseCase>,
        repository: Arc<dyn CollectionEntryRepository>,
        card_prices_refresh: Arc<dyn CardPricesRefreshRequester>,
        attach_card: Arc<dyn AttachCardToSealedProductUseCase>,
    ) -> Self {
        Self {
            create_card,
            repository,
            card_prices_refresh,
            attach_card,
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
        purchase_price: Option<PurchaseAmount>,
        sealed_product_id: Option<PurchaseId>,
    ) -> Result<CardEntries, AppError> {
        if let Some(sealed_product_id) = sealed_product_id {
            if purchase_price.is_some() {
                return Err(FunctionalError::InvalidRequest(
                    "purchase_price and sealed_product_id are exclusive".to_string(),
                )
                .into());
            }
            return self
                .attach_card
                .attach_card(
                    user_id,
                    sealed_product_id,
                    card_id,
                    foil,
                    quantity,
                    binder_id,
                )
                .await;
        }

        let quantity = validate_quantity(quantity)?;
        let purchase = purchase_price
            .map(|price| {
                NewSingleCardPurchase::try_new(price, quantity, chrono::Local::now().date_naive())
            })
            .transpose()?;

        let card_id = self.create_card.create_card(card_id).await?;
        let entries = self
            .repository
            .add(user_id, &card_id, foil, quantity, binder_id, purchase)
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
    use crate::application::service::collection::test_fixtures::{creating_card, refresh};
    use crate::application::use_case::{
        MockAttachCardToSealedProductUseCase, MockCreateCardUseCase,
    };
    use crate::domain::language_code::LanguageCode;
    use uuid::Uuid;

    fn service_attaching(
        create_card: MockCreateCardUseCase,
        repository: MockCollectionEntryRepository,
        refresh: MockCardPricesRefreshRequester,
        attach_card: MockAttachCardToSealedProductUseCase,
    ) -> AddCollectionEntryService {
        AddCollectionEntryService::new(
            Arc::new(create_card),
            Arc::new(repository),
            Arc::new(refresh),
            Arc::new(attach_card),
        )
    }

    /// A service whose attachment is never called.
    fn service(
        create_card: MockCreateCardUseCase,
        repository: MockCollectionEntryRepository,
        refresh: MockCardPricesRefreshRequester,
    ) -> AddCollectionEntryService {
        service_attaching(
            create_card,
            repository,
            refresh,
            MockAttachCardToSealedProductUseCase::new(),
        )
    }

    fn price(cents: i32) -> Option<PurchaseAmount> {
        Some(PurchaseAmount::try_new(cents).unwrap())
    }

    #[tokio::test]
    async fn creates_the_card_adds_the_copies_and_requests_a_refresh_once() {
        let binder_id = BinderId(Uuid::from_u128(5));
        let mut repository = MockCollectionEntryRepository::new();
        repository
            .expect_add()
            .withf(move |user_id, card_id, foil, quantity, binder, purchase| {
                user_id.as_str() == "user_1"
                    // The id the card creation returned, not the one asked for.
                    && card_id == &CardId::new("MKM", "12", LanguageCode::JA)
                    && *foil
                    && *quantity == 4
                    && *binder == Some(binder_id)
                    && purchase.is_none()
            })
            .times(1)
            .returning(|_, _, _, _, _, _| Box::pin(async { Ok(CardEntries::default()) }));
        let service = service(creating_card(1), repository, refresh(1));

        let result = service
            .add_collection_entry(
                &UserId::new("user_1"),
                CardId::new("MKM", "012", LanguageCode::JA),
                true,
                4,
                Some(binder_id),
                None,
                None,
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
                    None,
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
                None,
                None,
            )
            .await
            .unwrap_err();

        assert_eq!(error.code(), "card_not_in_catalog");
    }

    #[tokio::test]
    async fn a_failed_add_requests_no_refresh_and_keeps_its_error() {
        let mut repository = MockCollectionEntryRepository::new();
        repository.expect_add().returning(|_, _, _, _, _, _| {
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
                None,
                None,
            )
            .await
            .unwrap_err();

        assert_eq!(error.code(), "binder_not_found");
    }

    #[tokio::test]
    async fn a_purchase_price_records_a_single_card_purchase_of_the_price_times_the_quantity() {
        let mut repository = MockCollectionEntryRepository::new();
        repository
            .expect_add()
            .withf(|_, _, _, quantity, _, purchase| {
                let purchase = purchase.expect("a purchase is recorded");
                *quantity == 4
                    && purchase.price_per_copy.cents() == 200
                    && purchase.amount.cents() == 800
                    && purchase.purchased_on == chrono::Local::now().date_naive()
            })
            .times(1)
            .returning(|_, _, _, _, _, _| Box::pin(async { Ok(CardEntries::default()) }));
        let service = service(creating_card(1), repository, refresh(1));

        let result = service
            .add_collection_entry(
                &UserId::new("user_1"),
                CardId::new("MKM", "12", LanguageCode::FR),
                false,
                4,
                None,
                price(200),
                None,
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn a_sealed_product_hands_the_copies_to_the_attachment_alone() {
        let binder_id = BinderId(Uuid::from_u128(5));
        let sealed_product_id = PurchaseId(Uuid::from_u128(7));
        let mut attach_card = MockAttachCardToSealedProductUseCase::new();
        attach_card
            .expect_attach_card()
            .withf(move |user_id, id, card_id, foil, quantity, binder| {
                user_id.as_str() == "user_1"
                    && *id == sealed_product_id
                    && card_id == &CardId::new("MKM", "012", LanguageCode::JA)
                    && *foil
                    && *quantity == 4
                    && *binder == Some(binder_id)
            })
            .times(1)
            .returning(|_, _, _, _, _, _| Box::pin(async { Ok(CardEntries::default()) }));
        let mut repository = MockCollectionEntryRepository::new();
        repository.expect_add().never();
        let service = service_attaching(creating_card(0), repository, refresh(0), attach_card);

        let result = service
            .add_collection_entry(
                &UserId::new("user_1"),
                CardId::new("MKM", "012", LanguageCode::JA),
                true,
                4,
                Some(binder_id),
                None,
                Some(sealed_product_id),
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn refuses_a_purchase_price_together_with_a_sealed_product_and_writes_nothing() {
        let mut repository = MockCollectionEntryRepository::new();
        repository.expect_add().never();
        // The attachment is never called: the mock has no expectation.
        let service = service(creating_card(0), repository, refresh(0));

        let error = service
            .add_collection_entry(
                &UserId::new("user_1"),
                CardId::new("MKM", "12", LanguageCode::FR),
                false,
                1,
                None,
                price(200),
                Some(PurchaseId(Uuid::from_u128(7))),
            )
            .await
            .unwrap_err();

        assert_eq!(error.code(), "invalid_request");
    }
}
