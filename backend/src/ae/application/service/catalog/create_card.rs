use crate::application::error::AppError;
use crate::application::repository::{CardPricesViewRepository, CardRepository, CatalogRepository};
use crate::application::use_case::{
    CreateCardUseCase, EnqueueCardImageUpdateUseCase, EnqueueCardMarketIdUpdateUseCase,
};
use crate::domain::card::CardId;
use crate::domain::error::FunctionalError;
use async_trait::async_trait;
use std::sync::Arc;

pub struct CreateCardService {
    /// Only for `exists`: whether a Card is already in base, whoever owns it.
    known_cards: Arc<dyn CardPricesViewRepository>,
    catalog_repository: Arc<dyn CatalogRepository>,
    card_repository: Arc<dyn CardRepository>,
    enqueue_cardmarket_ids: Arc<dyn EnqueueCardMarketIdUpdateUseCase>,
    enqueue_card_images: Arc<dyn EnqueueCardImageUpdateUseCase>,
}

impl CreateCardService {
    pub fn new(
        known_cards: Arc<dyn CardPricesViewRepository>,
        catalog_repository: Arc<dyn CatalogRepository>,
        card_repository: Arc<dyn CardRepository>,
        enqueue_cardmarket_ids: Arc<dyn EnqueueCardMarketIdUpdateUseCase>,
        enqueue_card_images: Arc<dyn EnqueueCardImageUpdateUseCase>,
    ) -> Self {
        Self {
            known_cards,
            catalog_repository,
            card_repository,
            enqueue_cardmarket_ids,
            enqueue_card_images,
        }
    }
}

#[async_trait]
impl CreateCardUseCase for CreateCardService {
    #[tracing::instrument(skip(self))]
    async fn create_card(&self, card_id: CardId) -> Result<CardId, AppError> {
        let card_id = CardId::try_new(
            card_id.set_code,
            strip_leading_zeros(&card_id.collector_number),
            card_id.language_code,
        )?;

        // A Card already in base is returned as is, even when the catalog lacks it (a card
        // imported before the catalog, or one Scryfall dropped).
        if self.known_cards.exists(&card_id).await? {
            return Ok(card_id);
        }

        let catalog_card = self
            .catalog_repository
            .find(&card_id.set_code, &card_id.collector_number)
            .await?
            .ok_or(FunctionalError::CardNotInCatalog)?;

        let created = self
            .card_repository
            .create_if_absent(&card_id, &catalog_card)
            .await?;
        if created {
            if catalog_card.cardmarket_id.is_none() {
                self.enqueue_cardmarket_ids
                    .enqueue_pending_updates()
                    .await?;
            }
            self.enqueue_card_images.enqueue_pending_updates().await?;
        }

        Ok(card_id)
    }
}

/// `0012` becomes `12`; a number made of zeros keeps one (`000` becomes `0`).
fn strip_leading_zeros(collector_number: &str) -> &str {
    match collector_number.trim_start_matches('0') {
        "" if !collector_number.is_empty() => "0",
        stripped => stripped,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::{
        MockCardPricesViewRepository, MockCardRepository, MockCatalogRepository,
    };
    use crate::application::use_case::{
        MockEnqueueCardImageUpdateUseCase, MockEnqueueCardMarketIdUpdateUseCase,
    };
    use crate::domain::catalog::CatalogCard;
    use crate::domain::language_code::LanguageCode;
    use crate::domain::rarity_code::RarityCode;
    use crate::domain::set_name::SetCode;
    use uuid::Uuid;

    fn catalog_card(number: &str, cardmarket_id: Option<u32>) -> CatalogCard {
        CatalogCard {
            set_code: SetCode::new("MKM"),
            collector_number: number.to_string(),
            rarity: RarityCode::R,
            name: "Sample Card".to_string(),
            french_name: Some("Carte exemple".to_string()),
            scryfall_id: Uuid::from_u128(7),
            cardmarket_id,
        }
    }

    /// A catalog holding `catalog_card` only.
    fn catalog_with(catalog_card: Option<CatalogCard>) -> MockCatalogRepository {
        let mut catalog = MockCatalogRepository::new();
        catalog.expect_find().returning(move |set_code, number| {
            let found = catalog_card
                .clone()
                .filter(|p| &p.set_code == set_code && p.collector_number == number);
            Box::pin(async move { Ok(found) })
        });
        catalog
    }

    /// A card repository that creates (or finds already there) every card, and records nothing else.
    fn cards(created: bool) -> MockCardRepository {
        let mut cards = MockCardRepository::new();
        cards
            .expect_create_if_absent()
            .returning(move |_, _| Box::pin(async move { Ok(created) }));
        cards
    }

    /// The cards already in base: none, or exactly `existing`.
    fn known_cards(existing: Option<CardId>) -> MockCardPricesViewRepository {
        let mut known = MockCardPricesViewRepository::new();
        known.expect_exists().returning(move |card_id| {
            let found = existing.as_ref() == Some(card_id);
            Box::pin(async move { Ok(found) })
        });
        known
    }

    fn images(times: usize) -> MockEnqueueCardImageUpdateUseCase {
        let mut images = MockEnqueueCardImageUpdateUseCase::new();
        images
            .expect_enqueue_pending_updates()
            .times(times)
            .returning(|| Box::pin(async { Ok(1) }));
        images
    }

    fn cardmarket_ids(times: usize) -> MockEnqueueCardMarketIdUpdateUseCase {
        let mut ids = MockEnqueueCardMarketIdUpdateUseCase::new();
        ids.expect_enqueue_pending_updates()
            .times(times)
            .returning(|| Box::pin(async { Ok(1) }));
        ids
    }

    fn service(
        catalog: MockCatalogRepository,
        cards: MockCardRepository,
        cardmarket_ids: MockEnqueueCardMarketIdUpdateUseCase,
        images: MockEnqueueCardImageUpdateUseCase,
    ) -> CreateCardService {
        service_with_known(known_cards(None), catalog, cards, cardmarket_ids, images)
    }

    fn service_with_known(
        known: MockCardPricesViewRepository,
        catalog: MockCatalogRepository,
        cards: MockCardRepository,
        cardmarket_ids: MockEnqueueCardMarketIdUpdateUseCase,
        images: MockEnqueueCardImageUpdateUseCase,
    ) -> CreateCardService {
        CreateCardService::new(
            Arc::new(known),
            Arc::new(catalog),
            Arc::new(cards),
            Arc::new(cardmarket_ids),
            Arc::new(images),
        )
    }

    #[tokio::test]
    async fn creates_a_card_in_a_language_scryfall_knows() {
        let mut cards = MockCardRepository::new();
        cards
            .expect_create_if_absent()
            .withf(|id, catalog_card| {
                id == &CardId::new("MKM", "12", LanguageCode::FR)
                    && catalog_card.name == "Sample Card"
                    && catalog_card.rarity == RarityCode::R
                    && catalog_card.scryfall_id == Uuid::from_u128(7)
            })
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(true) }));
        let service = service(
            catalog_with(Some(catalog_card("12", Some(99)))),
            cards,
            cardmarket_ids(0),
            images(1),
        );

        let id = service
            .create_card(CardId::new("MKM", "12", LanguageCode::FR))
            .await
            .unwrap();

        assert_eq!(id, CardId::new("MKM", "12", LanguageCode::FR));
    }

    #[tokio::test]
    async fn creates_a_card_in_a_language_scryfall_does_not_have() {
        // The catalog has no language, and the one read is not checked against Scryfall: a
        // Japanese card Scryfall lacks is created like any other.
        let service = service(
            catalog_with(Some(catalog_card("12", Some(99)))),
            cards(true),
            cardmarket_ids(0),
            images(1),
        );

        let id = service
            .create_card(CardId::new("MKM", "12", LanguageCode::JA))
            .await
            .unwrap();

        assert_eq!(id, CardId::new("MKM", "12", LanguageCode::JA));
    }

    #[tokio::test]
    async fn strips_the_leading_zeros_of_the_collector_number() {
        let service = service(
            catalog_with(Some(catalog_card("12", Some(99)))),
            cards(true),
            cardmarket_ids(0),
            images(1),
        );

        let id = service
            .create_card(CardId::new("MKM", "0012", LanguageCode::EN))
            .await
            .unwrap();

        assert_eq!(id.collector_number, "12");
    }

    #[tokio::test]
    async fn keeps_a_collector_number_made_of_zeros_and_a_non_numeric_one() {
        let mut catalog = MockCatalogRepository::new();
        catalog
            .expect_find()
            .withf(|_, number| number == "0" || number == "H13")
            .times(2)
            .returning(|_, number| {
                let found = catalog_card(number, None);
                Box::pin(async move { Ok(Some(found)) })
            });
        let service = service(catalog, cards(true), cardmarket_ids(2), images(2));

        let zero = service
            .create_card(CardId::new("MKM", "000", LanguageCode::EN))
            .await
            .unwrap();
        let promo = service
            .create_card(CardId::new("MKM", "H13", LanguageCode::EN))
            .await
            .unwrap();

        assert_eq!(zero.collector_number, "0");
        assert_eq!(promo.collector_number, "H13");
    }

    #[tokio::test]
    async fn an_existing_card_is_returned_without_enrichment() {
        let service = service(
            catalog_with(Some(catalog_card("12", None))),
            cards(false),
            cardmarket_ids(0),
            images(0),
        );

        let id = service
            .create_card(CardId::new("MKM", "12", LanguageCode::EN))
            .await
            .unwrap();

        assert_eq!(id, CardId::new("MKM", "12", LanguageCode::EN));
    }

    #[tokio::test]
    async fn a_card_already_in_base_is_returned_even_when_the_catalog_lacks_it() {
        // An imported card the catalog does not have (a token, a card Scryfall dropped).
        let mut catalog = MockCatalogRepository::new();
        catalog.expect_find().never();
        let mut cards = MockCardRepository::new();
        cards.expect_create_if_absent().never();
        let service = service_with_known(
            known_cards(Some(CardId::new("TMKM", "12", LanguageCode::EN))),
            catalog,
            cards,
            cardmarket_ids(0),
            images(0),
        );

        let id = service
            .create_card(CardId::new("TMKM", "0012", LanguageCode::EN))
            .await
            .unwrap();

        assert_eq!(id, CardId::new("TMKM", "12", LanguageCode::EN));
    }

    #[tokio::test]
    async fn an_unknown_set_and_number_is_refused() {
        let mut cards = MockCardRepository::new();
        cards.expect_create_if_absent().never();
        let service = service(catalog_with(None), cards, cardmarket_ids(0), images(0));

        let error = service
            .create_card(CardId::new("MKM", "999", LanguageCode::EN))
            .await
            .unwrap_err();

        assert_eq!(error.code(), "card_not_in_catalog");
    }

    #[tokio::test]
    async fn a_token_is_refused_because_the_catalog_leaves_tokens_out() {
        // `TMKM/12` is the token of `MKM/12`: only the real card is in the catalog.
        let mut cards = MockCardRepository::new();
        cards.expect_create_if_absent().never();
        let service = service(
            catalog_with(Some(catalog_card("12", None))),
            cards,
            cardmarket_ids(0),
            images(0),
        );

        let error = service
            .create_card(CardId::new("TMKM", "12", LanguageCode::EN))
            .await
            .unwrap_err();

        assert_eq!(error.code(), "card_not_in_catalog");
    }

    #[tokio::test]
    async fn the_cardmarket_id_of_the_catalog_is_not_looked_up_again() {
        let mut cards = MockCardRepository::new();
        cards
            .expect_create_if_absent()
            .withf(|_, catalog_card| catalog_card.cardmarket_id == Some(99))
            .returning(|_, _| Box::pin(async { Ok(true) }));
        let service = service(
            catalog_with(Some(catalog_card("12", Some(99)))),
            cards,
            cardmarket_ids(0),
            images(1),
        );

        service
            .create_card(CardId::new("MKM", "12", LanguageCode::EN))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn a_missing_cardmarket_id_is_queued() {
        let service = service(
            catalog_with(Some(catalog_card("12", None))),
            cards(true),
            cardmarket_ids(1),
            images(1),
        );

        service
            .create_card(CardId::new("MKM", "12", LanguageCode::EN))
            .await
            .unwrap();
    }
}
