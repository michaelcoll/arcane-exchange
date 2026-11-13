use crate::application::error::AppError;
use crate::application::repository::{CardPricesRefreshRequester, PurchaseRepository};
use crate::application::use_case::{AttachCardToSealedProductUseCase, CreateCardUseCase};
use crate::domain::binder::BinderId;
use crate::domain::card::CardId;
use crate::domain::collection_entry::{CardEntries, validate_quantity};
use crate::domain::purchase::PurchaseId;
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct AttachCardToSealedProductService {
    create_card: Arc<dyn CreateCardUseCase>,
    repository: Arc<dyn PurchaseRepository>,
    card_prices_refresh: Arc<dyn CardPricesRefreshRequester>,
}

impl AttachCardToSealedProductService {
    pub fn new(
        create_card: Arc<dyn CreateCardUseCase>,
        repository: Arc<dyn PurchaseRepository>,
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
impl AttachCardToSealedProductUseCase for AttachCardToSealedProductService {
    #[tracing::instrument(skip(self))]
    async fn attach_card(
        &self,
        user_id: &UserId,
        purchase_id: PurchaseId,
        card_id: CardId,
        foil: bool,
        quantity: i32,
        binder_id: Option<BinderId>,
    ) -> Result<CardEntries, AppError> {
        let quantity = validate_quantity(quantity)?;

        let card_id = self.create_card.create_card(card_id).await?;
        let entries = self
            .repository
            .add_card_to_sealed_product(user_id, purchase_id, &card_id, foil, quantity, binder_id)
            .await?;
        self.card_prices_refresh.request_card_prices_refresh();

        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::{MockCardPricesRefreshRequester, MockPurchaseRepository};
    use crate::application::service::collection::test_fixtures::{creating_card, refresh};
    use crate::application::service::purchase::test_fixtures::purchase_id;
    use crate::application::use_case::MockCreateCardUseCase;
    use crate::domain::error::FunctionalError;
    use crate::domain::language_code::LanguageCode;
    use uuid::Uuid;

    fn service(
        create_card: MockCreateCardUseCase,
        repository: MockPurchaseRepository,
        refresh: MockCardPricesRefreshRequester,
    ) -> AttachCardToSealedProductService {
        AttachCardToSealedProductService::new(
            Arc::new(create_card),
            Arc::new(repository),
            Arc::new(refresh),
        )
    }

    #[tokio::test]
    async fn creates_the_card_attaches_the_copies_and_requests_a_refresh_once() {
        let binder_id = BinderId(Uuid::from_u128(5));
        let mut repository = MockPurchaseRepository::new();
        repository
            .expect_add_card_to_sealed_product()
            .withf(move |user_id, id, card_id, foil, quantity, binder| {
                user_id.as_str() == "user_1"
                    && *id == purchase_id()
                    // The id the card creation returned, not the one asked for.
                    && card_id == &CardId::new("MKM", "12", LanguageCode::JA)
                    && *foil
                    && *quantity == 4
                    && *binder == Some(binder_id)
            })
            .times(1)
            .returning(|_, _, _, _, _, _| Box::pin(async { Ok(CardEntries::default()) }));
        let service = service(creating_card(1), repository, refresh(1));

        let result = service
            .attach_card(
                &UserId::new("user_1"),
                purchase_id(),
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
            let mut repository = MockPurchaseRepository::new();
            repository.expect_add_card_to_sealed_product().never();
            let service = service(creating_card(0), repository, refresh(0));

            let error = service
                .attach_card(
                    &UserId::new("user_1"),
                    purchase_id(),
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
    async fn a_refused_attachment_keeps_its_error_and_requests_no_refresh() {
        let mut repository = MockPurchaseRepository::new();
        repository
            .expect_add_card_to_sealed_product()
            .returning(|_, _, _, _, _, _| {
                Box::pin(async { Err(FunctionalError::PurchaseNotFound.into()) })
            });
        let service = service(creating_card(1), repository, refresh(0));

        let error = service
            .attach_card(
                &UserId::new("user_1"),
                purchase_id(),
                CardId::new("MKM", "12", LanguageCode::FR),
                false,
                1,
                None,
            )
            .await
            .unwrap_err();

        assert_eq!(error.code(), "purchase_not_found");
    }
}
