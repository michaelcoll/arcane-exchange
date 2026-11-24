//! In-memory queue that enriches cards with an external identifier, one source per queue.
//!
//! The queue owns the whole protocol shared by every source: the unbounded channel, the set of
//! cards currently in flight (a card is never queued twice) and the refresh of the card prices
//! views once the queue drains. A source only says which cards are pending and how to resolve
//! one of them — see [`Enricher`].

use crate::application::error::{AppError, InfraError};
use crate::application::repository::CardPricesViewRepository;
use crate::domain::card::CardId;
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, PoisonError};
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::task::JoinSet;

/// One enrichment source (e.g. Cardmarket id via Scryfall, Gatherer id).
#[async_trait]
pub trait Enricher: Send + Sync + 'static {
    /// What `resolve` needs, besides the card id, to look the card up in the external source.
    type Lookup: Send + 'static;

    /// Source name, used in logs.
    const NAME: &'static str;

    /// How many cards are resolved at the same time at most.
    const CONCURRENCY: usize = 1;

    /// Cards still missing the enriched attribute.
    async fn pending(&self) -> Result<Vec<(CardId, Self::Lookup)>, AppError>;

    /// Looks the card up and stores the result. An error is logged by the queue and does not
    /// stop it.
    async fn resolve(&self, card_id: &CardId, lookup: Self::Lookup) -> Result<(), AppError>;

    /// Source-specific work run when the queue drains, before the views are refreshed.
    async fn after_drain(&self) -> Result<(), AppError> {
        Ok(())
    }
}

/// Enqueue side of an enrichment queue. Its worker runs in its own task, see [`Self::spawn`].
pub struct EnrichmentQueue<E: Enricher> {
    enricher: Arc<E>,
    sender: UnboundedSender<(CardId, E::Lookup)>,
    in_flight: Arc<InFlightCards>,
}

impl<E: Enricher> EnrichmentQueue<E> {
    pub fn spawn(enricher: E, card_prices_view: Arc<dyn CardPricesViewRepository>) -> Arc<Self> {
        let (queue, worker) = Self::new(enricher, card_prices_view);
        tokio::spawn(worker.run());
        Arc::new(queue)
    }

    /// Creates the queue and its worker, left to the caller to run.
    pub(super) fn new(
        enricher: E,
        card_prices_view: Arc<dyn CardPricesViewRepository>,
    ) -> (Self, EnrichmentWorker<E>) {
        let enricher = Arc::new(enricher);
        let (sender, receiver) = unbounded_channel();
        let in_flight = Arc::new(InFlightCards::default());
        let worker = EnrichmentWorker {
            enricher: enricher.clone(),
            receiver,
            in_flight: in_flight.clone(),
            card_prices_view,
        };
        let queue = Self {
            enricher,
            sender,
            in_flight,
        };
        (queue, worker)
    }

    /// Queues every pending card that is not already in flight and returns how many were queued.
    pub async fn enqueue_pending(&self) -> Result<usize, AppError> {
        let cards = self.enricher.pending().await?;
        self.enqueue(cards)
    }

    pub fn enricher(&self) -> &E {
        &self.enricher
    }

    /// Queues the given cards that are not already in flight, whether pending or not, and
    /// returns how many were queued.
    pub fn enqueue(&self, cards: Vec<(CardId, E::Lookup)>) -> Result<usize, AppError> {
        let mut enqueued = 0;
        for (card_id, lookup) in cards {
            if !self.in_flight.insert(card_id.clone()) {
                continue;
            }
            if let Err(SendError((card_id, _))) = self.sender.send((card_id, lookup)) {
                self.in_flight.remove(&card_id);
                return Err(InfraError::QueueError(format!(
                    "{} worker channel closed, cannot enqueue card",
                    E::NAME
                ))
                .into());
            }
            enqueued += 1;
        }
        Ok(enqueued)
    }
}

pub(super) struct EnrichmentWorker<E: Enricher> {
    enricher: Arc<E>,
    receiver: UnboundedReceiver<(CardId, E::Lookup)>,
    in_flight: Arc<InFlightCards>,
    card_prices_view: Arc<dyn CardPricesViewRepository>,
}

impl<E: Enricher> EnrichmentWorker<E> {
    /// Resolves up to `E::CONCURRENCY` cards at once until every sender is dropped and the
    /// cards in progress are done.
    async fn run(mut self) {
        const {
            assert!(
                E::CONCURRENCY > 0,
                "an enricher resolves at least one card at a time"
            )
        };
        tracing::info!("{} enrichment worker started.", E::NAME);

        let mut tasks: JoinSet<Result<(), AppError>> = JoinSet::new();
        // The card of each task, found again by task id (a panicked task returns nothing).
        let mut task_cards: HashMap<tokio::task::Id, CardId> = HashMap::new();
        let mut channel_open = true;

        while channel_open || !tasks.is_empty() {
            tokio::select! {
                received = self.receiver.recv(),
                    if channel_open && tasks.len() < E::CONCURRENCY =>
                {
                    match received {
                        Some((card_id, lookup)) => {
                            let enricher = self.enricher.clone();
                            let handle = tasks.spawn({
                                let card_id = card_id.clone();
                                async move { enricher.resolve(&card_id, lookup).await }
                            });
                            task_cards.insert(handle.id(), card_id);
                        }
                        None => channel_open = false,
                    }
                }
                Some(joined) = tasks.join_next_with_id() => {
                    let (task_id, outcome) = match joined {
                        Ok((task_id, result)) => (task_id, result.map_err(|e| format!("{e:?}"))),
                        Err(e) => (e.id(), Err(format!("task failed: {e}"))),
                    };
                    let Some(card_id) = task_cards.remove(&task_id) else {
                        continue;
                    };
                    if let Err(e) = outcome {
                        tracing::error!(
                            "{} enrichment failed for card {}: {}",
                            E::NAME,
                            card_id,
                            e
                        );
                    }
                    self.in_flight.remove(&card_id);

                    if tasks.is_empty() && self.receiver.is_empty() {
                        self.on_drained().await;
                    }
                }
            }
        }
    }

    async fn on_drained(&self) {
        if let Err(e) = self.enricher.after_drain().await {
            tracing::error!("{} post-enrichment step failed: {:?}", E::NAME, e);
        }
        if let Err(e) = self.card_prices_view.refresh().await {
            tracing::error!("Failed to refresh card price view: {:?}", e);
        }
    }
}

/// Cards queued and not yet processed. The lock is never held across an `.await`, and no code
/// path can panic while holding it, so a poisoned lock still guards a consistent set.
#[derive(Default)]
struct InFlightCards(Mutex<HashSet<CardId>>);

impl InFlightCards {
    /// Returns `false` if the card was already in flight.
    fn insert(&self, card_id: CardId) -> bool {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(card_id)
    }

    fn remove(&self, card_id: &CardId) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(card_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::MockCardPricesViewRepository;
    use crate::domain::language_code::LanguageCode;
    use crate::domain::set_name::SetCode;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;
    use tokio::sync::Notify;

    fn make_card_id(n: &str) -> CardId {
        CardId::new(SetCode::new("FDN"), n, LanguageCode::FR)
    }

    /// Records every call; `pending` returns the same cards on every call.
    #[derive(Default)]
    struct FakeEnricher {
        pending: Vec<CardId>,
        pending_fails: bool,
        failing_card: Option<CardId>,
        resolved: Mutex<Vec<CardId>>,
        drains: AtomicUsize,
        drained: Notify,
    }

    impl FakeEnricher {
        fn with_pending(cards: &[&str]) -> Self {
            Self {
                pending: cards.iter().copied().map(make_card_id).collect(),
                ..Self::default()
            }
        }
    }

    #[async_trait]
    impl Enricher for FakeEnricher {
        type Lookup = ();
        const NAME: &'static str = "Fake";

        async fn pending(&self) -> Result<Vec<(CardId, ())>, AppError> {
            if self.pending_fails {
                return Err(InfraError::RepositoryError("DB error".into()).into());
            }
            Ok(self.pending.iter().cloned().map(|c| (c, ())).collect())
        }

        async fn resolve(&self, card_id: &CardId, _: ()) -> Result<(), AppError> {
            self.resolved.lock().unwrap().push(card_id.clone());
            if self.failing_card.as_ref() == Some(card_id) {
                return Err(InfraError::CallError("source error".into()).into());
            }
            Ok(())
        }

        async fn after_drain(&self) -> Result<(), AppError> {
            self.drains.fetch_add(1, Ordering::SeqCst);
            self.drained.notify_one();
            Ok(())
        }
    }

    fn prices_view(refreshes: usize) -> Arc<MockCardPricesViewRepository> {
        let mut view = MockCardPricesViewRepository::new();
        view.expect_refresh()
            .times(refreshes)
            .returning(|| Box::pin(async { Ok(()) }));
        Arc::new(view)
    }

    #[tokio::test]
    async fn enqueue_returns_count_of_newly_enqueued_cards() {
        let (queue, _worker) =
            EnrichmentQueue::new(FakeEnricher::with_pending(&["0", "1"]), prices_view(0));

        assert_eq!(queue.enqueue_pending().await.unwrap(), 2);
    }

    #[tokio::test]
    async fn enqueue_returns_zero_for_empty_pending_list() {
        let (queue, _worker) =
            EnrichmentQueue::new(FakeEnricher::with_pending(&[]), prices_view(0));

        assert_eq!(queue.enqueue_pending().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn a_card_in_flight_is_never_queued_twice() {
        let (queue, _worker) =
            EnrichmentQueue::new(FakeEnricher::with_pending(&["0"]), prices_view(0));

        assert_eq!(queue.enqueue_pending().await.unwrap(), 1);
        assert_eq!(queue.enqueue_pending().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn only_cards_not_in_flight_are_queued() {
        let (queue, _worker) =
            EnrichmentQueue::new(FakeEnricher::with_pending(&["0", "1"]), prices_view(0));
        queue.in_flight.insert(make_card_id("0"));

        assert_eq!(queue.enqueue_pending().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn given_cards_are_queued_whether_pending_or_not_but_never_twice() {
        let (queue, _worker) =
            EnrichmentQueue::new(FakeEnricher::with_pending(&["0"]), prices_view(0));
        queue.enqueue_pending().await.unwrap();

        let enqueued = queue
            .enqueue(vec![(make_card_id("0"), ()), (make_card_id("9"), ())])
            .unwrap();

        assert_eq!(enqueued, 1);
    }

    #[tokio::test]
    async fn enqueue_propagates_pending_error() {
        let enricher = FakeEnricher {
            pending_fails: true,
            ..FakeEnricher::default()
        };
        let (queue, _worker) = EnrichmentQueue::new(enricher, prices_view(0));

        assert!(queue.enqueue_pending().await.is_err());
    }

    #[tokio::test]
    async fn enqueue_fails_when_the_worker_is_gone_and_leaves_no_card_in_flight() {
        let (queue, worker) =
            EnrichmentQueue::new(FakeEnricher::with_pending(&["0"]), prices_view(0));
        drop(worker);

        let result = queue.enqueue_pending().await;

        assert!(matches!(
            result,
            Err(AppError::Infra(InfraError::QueueError(_)))
        ));
        assert!(queue.in_flight.insert(make_card_id("0")));
    }

    #[tokio::test]
    async fn a_card_can_be_queued_again_once_processed() {
        let (queue, worker) =
            EnrichmentQueue::new(FakeEnricher::with_pending(&["0"]), prices_view(2));
        let enricher = queue.enricher.clone();
        let handle = tokio::spawn(worker.run());

        assert_eq!(queue.enqueue_pending().await.unwrap(), 1);
        enricher.drained.notified().await;
        assert_eq!(queue.enqueue_pending().await.unwrap(), 1);
        enricher.drained.notified().await;

        drop(queue);
        handle.await.unwrap();
        assert_eq!(
            *enricher.resolved.lock().unwrap(),
            vec![make_card_id("0"), make_card_id("0")]
        );
    }

    #[tokio::test]
    async fn views_are_refreshed_once_when_the_queue_drains() {
        let (queue, worker) =
            EnrichmentQueue::new(FakeEnricher::with_pending(&["0", "1", "2"]), prices_view(1));
        let enricher = queue.enricher.clone();

        assert_eq!(queue.enqueue_pending().await.unwrap(), 3);
        drop(queue);
        worker.run().await;

        assert_eq!(enricher.resolved.lock().unwrap().len(), 3);
        assert_eq!(enricher.drains.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn a_failed_resolution_does_not_stop_the_worker() {
        let enricher = FakeEnricher {
            failing_card: Some(make_card_id("0")),
            ..FakeEnricher::with_pending(&["0", "1"])
        };
        let (queue, worker) = EnrichmentQueue::new(enricher, prices_view(1));
        let enricher = queue.enricher.clone();

        queue.enqueue_pending().await.unwrap();
        drop(queue);
        worker.run().await;

        assert_eq!(
            *enricher.resolved.lock().unwrap(),
            vec![make_card_id("0"), make_card_id("1")]
        );
        assert_eq!(enricher.drains.load(Ordering::SeqCst), 1);
    }

    /// Resolves slowly, keeping track of how many `resolve` calls overlap.
    #[derive(Default)]
    struct SlowEnricher {
        pending: Vec<CardId>,
        panicking_card: Option<CardId>,
        running: AtomicUsize,
        max_running: AtomicUsize,
        resolved: AtomicUsize,
        drains: AtomicUsize,
    }

    #[async_trait]
    impl Enricher for SlowEnricher {
        type Lookup = ();
        const NAME: &'static str = "Slow";
        const CONCURRENCY: usize = 3;

        async fn pending(&self) -> Result<Vec<(CardId, ())>, AppError> {
            Ok(self.pending.iter().cloned().map(|c| (c, ())).collect())
        }

        async fn resolve(&self, card_id: &CardId, _: ()) -> Result<(), AppError> {
            let running = self.running.fetch_add(1, Ordering::SeqCst) + 1;
            self.max_running.fetch_max(running, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(30)).await;
            self.running.fetch_sub(1, Ordering::SeqCst);
            if self.panicking_card.as_ref() == Some(card_id) {
                panic!("resolve panicked");
            }
            self.resolved.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn after_drain(&self) -> Result<(), AppError> {
            self.drains.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    fn slow_enricher(cards: &[&str]) -> SlowEnricher {
        SlowEnricher {
            pending: cards.iter().copied().map(make_card_id).collect(),
            ..SlowEnricher::default()
        }
    }

    #[tokio::test]
    async fn at_most_concurrency_cards_are_resolved_at_once() {
        let (queue, worker) = EnrichmentQueue::new(
            slow_enricher(&["0", "1", "2", "3", "4", "5", "6", "7"]),
            prices_view(1),
        );
        let enricher = queue.enricher.clone();

        queue.enqueue_pending().await.unwrap();
        drop(queue);
        worker.run().await;

        assert_eq!(enricher.resolved.load(Ordering::SeqCst), 8);
        assert_eq!(enricher.max_running.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn the_drained_hook_runs_once_after_every_concurrent_resolution() {
        let (queue, worker) =
            EnrichmentQueue::new(slow_enricher(&["0", "1", "2", "3", "4"]), prices_view(1));
        let enricher = queue.enricher.clone();

        queue.enqueue_pending().await.unwrap();
        drop(queue);
        worker.run().await;

        assert_eq!(enricher.drains.load(Ordering::SeqCst), 1);
        assert_eq!(enricher.resolved.load(Ordering::SeqCst), 5);
    }

    #[tokio::test]
    async fn a_panicking_resolution_is_logged_and_frees_its_card() {
        let enricher = SlowEnricher {
            panicking_card: Some(make_card_id("1")),
            ..slow_enricher(&["0", "1", "2"])
        };
        let (queue, worker) = EnrichmentQueue::new(enricher, prices_view(1));
        let enricher = queue.enricher.clone();

        queue.enqueue_pending().await.unwrap();
        let in_flight = queue.in_flight.clone();
        drop(queue);
        worker.run().await;

        assert_eq!(enricher.resolved.load(Ordering::SeqCst), 2);
        assert_eq!(enricher.drains.load(Ordering::SeqCst), 1);
        assert!(in_flight.insert(make_card_id("1")));
    }

    #[tokio::test]
    async fn an_enricher_without_a_concurrency_override_resolves_one_card_at_a_time() {
        struct SerialEnricher {
            running: AtomicUsize,
            max_running: AtomicUsize,
        }

        #[async_trait]
        impl Enricher for SerialEnricher {
            type Lookup = ();
            const NAME: &'static str = "Serial";

            async fn pending(&self) -> Result<Vec<(CardId, ())>, AppError> {
                Ok(vec![])
            }

            async fn resolve(&self, _: &CardId, _: ()) -> Result<(), AppError> {
                let running = self.running.fetch_add(1, Ordering::SeqCst) + 1;
                self.max_running.fetch_max(running, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(20)).await;
                self.running.fetch_sub(1, Ordering::SeqCst);
                Ok(())
            }
        }

        let enricher = SerialEnricher {
            running: AtomicUsize::new(0),
            max_running: AtomicUsize::new(0),
        };
        let (queue, worker) = EnrichmentQueue::new(enricher, prices_view(1));
        let enricher = queue.enricher.clone();

        queue
            .enqueue(["0", "1", "2", "3"].map(|n| (make_card_id(n), ())).into())
            .unwrap();
        drop(queue);
        worker.run().await;

        assert_eq!(enricher.max_running.load(Ordering::SeqCst), 1);
    }
}
