use crate::application::caller::ScryfallCaller;
use crate::application::error::AppError;
use crate::application::repository::CardRepository;
use crate::application::service::enrichment_queue::{Enricher, EnrichmentQueue};
use crate::application::use_case::{
    CardCollectionPriceCalculationUseCase, EnqueueCardMarketIdUpdateUseCase,
};
use crate::domain::card::CardId;
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

/// Resolves a card's Cardmarket id from its Scryfall id.
pub struct CardMarketIdEnricher {
    card_repository: Arc<dyn CardRepository>,
    scryfall_caller: Arc<dyn ScryfallCaller>,
    price_calculation: Arc<dyn CardCollectionPriceCalculationUseCase>,
}

impl CardMarketIdEnricher {
    pub fn new(
        card_repository: Arc<dyn CardRepository>,
        scryfall_caller: Arc<dyn ScryfallCaller>,
        price_calculation: Arc<dyn CardCollectionPriceCalculationUseCase>,
    ) -> Self {
        Self {
            card_repository,
            scryfall_caller,
            price_calculation,
        }
    }
}

#[async_trait]
impl Enricher for CardMarketIdEnricher {
    /// The card's Scryfall id.
    type Job = Uuid;
    const NAME: &'static str = "CardMarket id";

    async fn pending(&self) -> Result<Vec<(CardId, Uuid)>, AppError> {
        self.card_repository.get_all_without_cardmarket_id().await
    }

    async fn resolve(&self, card_id: &CardId, scryfall_id: Uuid) -> Result<(), AppError> {
        let cardmarket_id = self.scryfall_caller.get_card_market_id(scryfall_id).await?;
        self.card_repository
            .update_cardmarket_id(card_id.clone(), cardmarket_id)
            .await?;
        tracing::info!("{} -> {:?}", card_id, cardmarket_id);
        Ok(())
    }

    /// Newly resolved ids give prices to cards that had none: the collection value changes.
    async fn after_drain(&self) -> Result<(), AppError> {
        self.price_calculation.calculate_total_price().await
    }
}

#[async_trait]
impl EnqueueCardMarketIdUpdateUseCase for EnrichmentQueue<CardMarketIdEnricher> {
    async fn enqueue_pending_updates(&self) -> Result<usize, AppError> {
        self.enqueue_pending().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::caller::MockScryfallCaller;
    use crate::application::error::InfraError;
    use crate::application::repository::MockCardRepository;
    use crate::application::use_case::MockCardCollectionPriceCalculationUseCase;
    use crate::domain::language_code::LanguageCode;
    use crate::domain::set_name::SetCode;

    fn make_card_id(n: &str) -> CardId {
        CardId::new(SetCode::new("FDN"), n, LanguageCode::FR)
    }

    fn enricher(
        card_repository: MockCardRepository,
        scryfall_caller: MockScryfallCaller,
        price_calc: MockCardCollectionPriceCalculationUseCase,
    ) -> CardMarketIdEnricher {
        CardMarketIdEnricher::new(
            Arc::new(card_repository),
            Arc::new(scryfall_caller),
            Arc::new(price_calc),
        )
    }

    #[tokio::test]
    async fn pending_lists_cards_without_cardmarket_id() {
        let mut card_repository = MockCardRepository::new();
        card_repository
            .expect_get_all_without_cardmarket_id()
            .returning(|| Box::pin(async { Ok(vec![(make_card_id("0"), Uuid::default())]) }));
        let enricher = enricher(
            card_repository,
            MockScryfallCaller::new(),
            MockCardCollectionPriceCalculationUseCase::new(),
        );

        let pending = enricher.pending().await.unwrap();

        assert_eq!(pending, vec![(make_card_id("0"), Uuid::default())]);
    }

    #[tokio::test]
    async fn resolve_stores_the_cardmarket_id_found_by_scryfall() {
        let mut card_repository = MockCardRepository::new();
        let mut scryfall_caller = MockScryfallCaller::new();
        scryfall_caller
            .expect_get_card_market_id()
            .returning(|_| Box::pin(async { Ok(Some(42)) }));
        card_repository
            .expect_update_cardmarket_id()
            .withf(|id, cardmarket_id| *id == make_card_id("0") && *cardmarket_id == Some(42))
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(()) }));
        let enricher = enricher(
            card_repository,
            scryfall_caller,
            MockCardCollectionPriceCalculationUseCase::new(),
        );

        enricher
            .resolve(&make_card_id("0"), Uuid::default())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn resolve_fails_without_update_on_scryfall_error() {
        let mut card_repository = MockCardRepository::new();
        let mut scryfall_caller = MockScryfallCaller::new();
        scryfall_caller.expect_get_card_market_id().returning(|_| {
            Box::pin(async {
                Err(AppError::Infra(InfraError::CallError(
                    "Scryfall error".to_string(),
                )))
            })
        });
        card_repository.expect_update_cardmarket_id().times(0);
        let enricher = enricher(
            card_repository,
            scryfall_caller,
            MockCardCollectionPriceCalculationUseCase::new(),
        );

        let result = enricher.resolve(&make_card_id("0"), Uuid::default()).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn after_drain_recalculates_the_collection_value() {
        let mut price_calc = MockCardCollectionPriceCalculationUseCase::new();
        price_calc
            .expect_calculate_total_price()
            .times(1)
            .returning(|| Box::pin(async { Ok(()) }));
        let enricher = enricher(
            MockCardRepository::new(),
            MockScryfallCaller::new(),
            price_calc,
        );

        enricher.after_drain().await.unwrap();
    }
}
