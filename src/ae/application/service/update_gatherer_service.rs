use crate::application::caller::GathererCaller;
use crate::application::error::{AppError, InfraError};
use crate::application::repository::{CardPricesViewRepository, CardRepository};
use crate::domain::card::CardId;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::UnboundedReceiver;

pub struct GathererIdWorker {
    card_repository: Arc<dyn CardRepository>,
    gatherer_caller: Arc<dyn GathererCaller>,
    card_prices_view_repository: Arc<dyn CardPricesViewRepository>,
    dedup_set: Arc<Mutex<HashSet<CardId>>>,
}

impl GathererIdWorker {
    pub fn new(
        card_repository: Arc<dyn CardRepository>,
        gatherer_caller: Arc<dyn GathererCaller>,
        card_prices_view_repository: Arc<dyn CardPricesViewRepository>,
        dedup_set: Arc<Mutex<HashSet<CardId>>>,
    ) -> Self {
        Self {
            card_repository,
            gatherer_caller,
            card_prices_view_repository,
            dedup_set,
        }
    }

    pub async fn run(
        self,
        mut receiver: UnboundedReceiver<(CardId, String)>,
    ) -> Result<(), AppError> {
        tracing::info!("Gatherer id updater started.");

        while let Some((card_id, name)) = receiver.recv().await {
            let gatherer_id = self
                .gatherer_caller
                .get_gatherer_id(
                    card_id.set_code.clone(),
                    card_id.collector_number.clone(),
                    card_id.language_code.clone(),
                    name,
                )
                .await;

            match gatherer_id {
                Ok(Some(id)) => {
                    if let Err(e) = self
                        .card_repository
                        .update_gatherer_id(card_id.clone(), Some(id))
                        .await
                    {
                        tracing::error!("Failed to update card with Gatherer ID: {:?}", e);
                    } else {
                        tracing::info!("{} ✓", card_id);
                    }
                }
                Ok(None) => {
                    tracing::trace!("No Gatherer ID found for card {}, leaving empty", card_id);
                }
                Err(e) => {
                    tracing::error!("Failed to fetch Gatherer ID for card {}: {:?}", card_id, e);
                }
            }

            {
                let mut set = self
                    .dedup_set
                    .lock()
                    .map_err(|_| InfraError::QueueError("Mutex poisoned".into()))?;
                set.remove(&card_id);
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
    use crate::application::caller::MockGathererCaller;
    use crate::application::error::{AppError, InfraError};
    use crate::application::repository::{MockCardPricesViewRepository, MockCardRepository};
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
    async fn worker_updates_card_and_refreshes_view_when_queue_empty() {
        let mut card_repository = MockCardRepository::new();
        let mut gatherer_caller = MockGathererCaller::new();

        gatherer_caller
            .expect_get_gatherer_id()
            .returning(|_, _, _, _| Box::pin(async { Ok(Some("abc123".to_string())) }));
        card_repository
            .expect_update_gatherer_id()
            .returning(|_, _| Box::pin(async { Ok(()) }));

        let dedup_set = Arc::new(Mutex::new(HashSet::new()));
        let (tx, rx) = unbounded_channel::<(CardId, String)>();

        let worker = GathererIdWorker::new(
            Arc::new(card_repository),
            Arc::new(gatherer_caller),
            Arc::new(mock_prices_view()),
            dedup_set,
        );

        let card_id = make_card_id("0");
        tx.send((card_id, "Goblin Boarders".to_string())).unwrap();
        drop(tx);

        worker.run(rx).await.unwrap();
    }

    #[tokio::test]
    async fn worker_removes_card_id_from_dedup_set_after_processing() {
        let mut card_repository = MockCardRepository::new();
        let mut gatherer_caller = MockGathererCaller::new();

        gatherer_caller
            .expect_get_gatherer_id()
            .returning(|_, _, _, _| Box::pin(async { Ok(Some("abc".to_string())) }));
        card_repository
            .expect_update_gatherer_id()
            .returning(|_, _| Box::pin(async { Ok(()) }));

        let dedup_set = Arc::new(Mutex::new(HashSet::new()));
        let card_id = make_card_id("0");
        dedup_set.lock().unwrap().insert(card_id.clone());

        let (tx, rx) = unbounded_channel::<(CardId, String)>();
        tx.send((card_id.clone(), "Name".to_string())).unwrap();
        drop(tx);

        let worker = GathererIdWorker::new(
            Arc::new(card_repository),
            Arc::new(gatherer_caller),
            Arc::new(mock_prices_view()),
            dedup_set.clone(),
        );
        worker.run(rx).await.unwrap();

        assert!(!dedup_set.lock().unwrap().contains(&card_id));
    }

    #[tokio::test]
    async fn worker_continues_and_leaves_column_empty_when_gatherer_returns_none() {
        let mut card_repository = MockCardRepository::new();
        let mut gatherer_caller = MockGathererCaller::new();

        gatherer_caller
            .expect_get_gatherer_id()
            .returning(|_, _, _, _| Box::pin(async { Ok(None) }));
        card_repository.expect_update_gatherer_id().times(0);

        let dedup_set = Arc::new(Mutex::new(HashSet::new()));
        let (tx, rx) = unbounded_channel::<(CardId, String)>();
        tx.send((make_card_id("0"), "Unknown Card".to_string()))
            .unwrap();
        drop(tx);

        let worker = GathererIdWorker::new(
            Arc::new(card_repository),
            Arc::new(gatherer_caller),
            Arc::new(mock_prices_view()),
            dedup_set,
        );
        worker.run(rx).await.unwrap();
    }

    #[tokio::test]
    async fn worker_continues_on_gatherer_call_error() {
        let mut card_repository = MockCardRepository::new();
        let mut gatherer_caller = MockGathererCaller::new();

        gatherer_caller
            .expect_get_gatherer_id()
            .returning(|_, _, _, _| {
                Box::pin(async {
                    Err(AppError::Infra(InfraError::CallError(
                        "Gatherer error".to_string(),
                    )))
                })
            });
        card_repository.expect_update_gatherer_id().times(0);

        let dedup_set = Arc::new(Mutex::new(HashSet::new()));
        let (tx, rx) = unbounded_channel::<(CardId, String)>();
        tx.send((make_card_id("0"), "Name".to_string())).unwrap();
        drop(tx);

        let worker = GathererIdWorker::new(
            Arc::new(card_repository),
            Arc::new(gatherer_caller),
            Arc::new(mock_prices_view()),
            dedup_set,
        );
        worker.run(rx).await.unwrap();
    }
}
