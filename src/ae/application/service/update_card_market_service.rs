use crate::application::caller::ScryfallCaller;
use crate::application::error::{AppError, InfraError};
use crate::application::repository::{CardPricesViewRepository, CardRepository};
use crate::application::use_case::CardCollectionPriceCalculationUseCase;
use crate::domain::card::CardId;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::UnboundedReceiver;
use uuid::Uuid;

pub struct CardMarketIdWorker {
    card_repository: Arc<dyn CardRepository>,
    scryfall_caller: Arc<dyn ScryfallCaller>,
    price_calculation: Arc<dyn CardCollectionPriceCalculationUseCase>,
    card_prices_view_repository: Arc<dyn CardPricesViewRepository>,
    dedup_set: Arc<Mutex<HashSet<CardId>>>,
}

impl CardMarketIdWorker {
    pub fn new(
        card_repository: Arc<dyn CardRepository>,
        scryfall_caller: Arc<dyn ScryfallCaller>,
        price_calculation: Arc<dyn CardCollectionPriceCalculationUseCase>,
        card_prices_view_repository: Arc<dyn CardPricesViewRepository>,
        dedup_set: Arc<Mutex<HashSet<CardId>>>,
    ) -> Self {
        Self {
            card_repository,
            scryfall_caller,
            price_calculation,
            card_prices_view_repository,
            dedup_set,
        }
    }

    pub async fn run(
        self,
        mut receiver: UnboundedReceiver<(CardId, Uuid)>,
    ) -> Result<(), AppError> {
        tracing::info!("Card market id updater started.");

        while let Some((card_id, scryfall_id)) = receiver.recv().await {
            let cardmarket_id = self.scryfall_caller.get_card_market_id(scryfall_id).await;
            if let Ok(id) = cardmarket_id {
                if let Err(e) = self
                    .card_repository
                    .update_cardmarket_id(card_id.clone(), id)
                    .await
                {
                    tracing::error!("Failed to update card with CardMarket ID: {:?}", e);
                } else {
                    tracing::info!("{} -> {:?}", card_id, id);
                }
            } else if let Err(e) = cardmarket_id {
                tracing::error!(
                    "Failed to fetch CardMarket ID for card {}: {:?}",
                    card_id,
                    e
                );
            }

            {
                let mut set = self
                    .dedup_set
                    .lock()
                    .map_err(|_| InfraError::QueueError("Mutex poisoned".into()))?;
                set.remove(&card_id);
            }

            if receiver.is_empty()
                && let Err(e) = self.price_calculation.calculate_total_price().await
            {
                tracing::error!("Failed to calculate total price: {:?}", e);
            }

            if receiver.is_empty()
                && let Err(e) = self.card_prices_view_repository.refresh().await
            {
                tracing::error!("Failed to refresh card price view: {:?}", e);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::caller::MockScryfallCaller;
    use crate::application::error::{AppError, InfraError};
    use crate::application::repository::{MockCardPricesViewRepository, MockCardRepository};
    use crate::application::use_case::MockCardCollectionPriceCalculationUseCase;
    use crate::domain::language_code::LanguageCode;
    use crate::domain::set_name::SetCode;
    use tokio::sync::mpsc::unbounded_channel;

    fn make_card_id(n: &str) -> CardId {
        CardId::new(SetCode::new("FDN"), n, LanguageCode::FR, false)
    }

    fn mock_prices_view() -> MockCardPricesViewRepository {
        let mut r = MockCardPricesViewRepository::new();
        r.expect_refresh().returning(|| Box::pin(async { Ok(()) }));
        r
    }

    #[tokio::test]
    async fn worker_updates_card_and_triggers_price_calc_when_queue_empty() {
        let mut card_repository = MockCardRepository::new();
        let mut scryfall_caller = MockScryfallCaller::new();
        let mut price_calc = MockCardCollectionPriceCalculationUseCase::new();

        scryfall_caller
            .expect_get_card_market_id()
            .returning(|_| Box::pin(async { Ok(Some(42)) }));
        card_repository
            .expect_update_cardmarket_id()
            .returning(|_, _| Box::pin(async { Ok(()) }));
        price_calc
            .expect_calculate_total_price()
            .times(1)
            .returning(|| Box::pin(async { Ok(()) }));

        let dedup_set = Arc::new(Mutex::new(HashSet::new()));
        let (tx, rx) = unbounded_channel::<(CardId, Uuid)>();

        let worker = CardMarketIdWorker::new(
            Arc::new(card_repository),
            Arc::new(scryfall_caller),
            Arc::new(price_calc),
            Arc::new(mock_prices_view()),
            dedup_set,
        );

        let card_id = make_card_id("0");
        tx.send((card_id, Uuid::default())).unwrap();
        drop(tx); // ferme le canal pour que run() se termine

        worker.run(rx).await.unwrap();
    }

    #[tokio::test]
    async fn worker_removes_card_id_from_dedup_set_after_processing() {
        let mut card_repository = MockCardRepository::new();
        let mut scryfall_caller = MockScryfallCaller::new();
        let mut price_calc = MockCardCollectionPriceCalculationUseCase::new();

        scryfall_caller
            .expect_get_card_market_id()
            .returning(|_| Box::pin(async { Ok(Some(1)) }));
        card_repository
            .expect_update_cardmarket_id()
            .returning(|_, _| Box::pin(async { Ok(()) }));
        price_calc
            .expect_calculate_total_price()
            .returning(|| Box::pin(async { Ok(()) }));

        let dedup_set = Arc::new(Mutex::new(HashSet::new()));
        let card_id = make_card_id("0");
        dedup_set.lock().unwrap().insert(card_id.clone());

        let (tx, rx) = unbounded_channel::<(CardId, Uuid)>();
        tx.send((card_id.clone(), Uuid::default())).unwrap();
        drop(tx);

        let worker = CardMarketIdWorker::new(
            Arc::new(card_repository),
            Arc::new(scryfall_caller),
            Arc::new(price_calc),
            Arc::new(mock_prices_view()),
            dedup_set.clone(),
        );
        worker.run(rx).await.unwrap();

        assert!(!dedup_set.lock().unwrap().contains(&card_id));
    }

    #[tokio::test]
    async fn worker_continues_on_scryfall_error() {
        let mut card_repository = MockCardRepository::new();
        let mut scryfall_caller = MockScryfallCaller::new();
        let mut price_calc = MockCardCollectionPriceCalculationUseCase::new();

        scryfall_caller.expect_get_card_market_id().returning(|_| {
            Box::pin(async {
                Err(AppError::Infra(InfraError::CallError(
                    "Scryfall error".to_string(),
                )))
            })
        });
        // update_cardmarket_id should not be called
        card_repository.expect_update_cardmarket_id().times(0);
        price_calc
            .expect_calculate_total_price()
            .returning(|| Box::pin(async { Ok(()) }));

        let dedup_set = Arc::new(Mutex::new(HashSet::new()));
        let (tx, rx) = unbounded_channel::<(CardId, Uuid)>();
        tx.send((make_card_id("0"), Uuid::default())).unwrap();
        drop(tx);

        let worker = CardMarketIdWorker::new(
            Arc::new(card_repository),
            Arc::new(scryfall_caller),
            Arc::new(price_calc),
            Arc::new(mock_prices_view()),
            dedup_set,
        );
        worker.run(rx).await.unwrap();
    }

    #[tokio::test]
    async fn worker_triggers_price_calc_and_refresh_after_batch() {
        let mut card_repository = MockCardRepository::new();
        let mut scryfall_caller = MockScryfallCaller::new();
        let mut price_calc = MockCardCollectionPriceCalculationUseCase::new();
        let mut prices_view = MockCardPricesViewRepository::new();

        scryfall_caller
            .expect_get_card_market_id()
            .returning(|_| Box::pin(async { Ok(Some(99)) }));
        card_repository
            .expect_update_cardmarket_id()
            .returning(|_, _| Box::pin(async { Ok(()) }));
        price_calc
            .expect_calculate_total_price()
            .times(1)
            .returning(|| Box::pin(async { Ok(()) }));
        prices_view
            .expect_refresh()
            .times(1)
            .returning(|| Box::pin(async { Ok(()) }));

        let dedup_set = Arc::new(Mutex::new(HashSet::new()));
        let (tx, rx) = unbounded_channel::<(CardId, Uuid)>();
        tx.send((make_card_id("0"), Uuid::default())).unwrap();
        drop(tx);

        let worker = CardMarketIdWorker::new(
            Arc::new(card_repository),
            Arc::new(scryfall_caller),
            Arc::new(price_calc),
            Arc::new(prices_view),
            dedup_set,
        );
        worker.run(rx).await.unwrap();
    }
}
