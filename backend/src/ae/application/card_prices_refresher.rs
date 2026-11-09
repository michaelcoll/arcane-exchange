//! Background refresh of the card prices views, on request.
//!
//! Asking for a refresh never waits for it: the request is recorded and the worker, running in
//! its own task, does the work. Requests are coalesced: one refresh runs at a time, and any
//! number of requests received while it runs trigger a single further refresh. Imports and the
//! price import refresh synchronously and do not go through here.

use crate::application::repository::{CardPricesRefreshRequester, CardPricesViewRepository};
use std::sync::Arc;
use tokio::sync::Notify;

/// Request side of the refresher. Its worker runs in its own task, see [`Self::spawn`].
pub struct CardPricesRefresher {
    requested: Arc<Notify>,
}

impl CardPricesRefresher {
    /// Creates the refresher and spawns its worker on the Tokio runtime.
    pub fn spawn(card_prices_view: Arc<dyn CardPricesViewRepository>) -> Arc<Self> {
        let (refresher, worker) = Self::new(card_prices_view);
        tokio::spawn(worker.run());
        Arc::new(refresher)
    }

    /// Creates the refresher and its worker, left to the caller to run.
    pub(super) fn new(
        card_prices_view: Arc<dyn CardPricesViewRepository>,
    ) -> (Self, CardPricesRefreshWorker) {
        let requested = Arc::new(Notify::new());
        let worker = CardPricesRefreshWorker {
            requested: requested.clone(),
            card_prices_view,
        };
        (Self { requested }, worker)
    }
}

impl CardPricesRefreshRequester for CardPricesRefresher {
    fn request_card_prices_refresh(&self) {
        // `notify_one` keeps at most one permit: requests received while a refresh runs
        // collapse into a single further refresh.
        self.requested.notify_one();
    }
}

pub(super) struct CardPricesRefreshWorker {
    requested: Arc<Notify>,
    card_prices_view: Arc<dyn CardPricesViewRepository>,
}

impl CardPricesRefreshWorker {
    /// Refreshes the views once per batch of requests, forever.
    async fn run(self) {
        tracing::info!("Card prices refresher started.");

        loop {
            self.requested.notified().await;
            if let Err(e) = self.card_prices_view.refresh().await {
                tracing::error!("Failed to refresh card price view: {:?}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::error::{AppError, InfraError};
    use crate::application::repository::MockCardPricesViewRepository;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    /// Counts the refreshes. The first one waits for `release`; each finished refresh notifies
    /// `finished`. A refresh whose index is in `failing` returns an error.
    #[derive(Default)]
    struct Probe {
        started: AtomicUsize,
        started_signal: Notify,
        release: Notify,
        finished: Notify,
        failing: Mutex<Vec<usize>>,
    }

    fn refresher(probe: &Arc<Probe>, hold_first: bool) -> Arc<CardPricesRefresher> {
        let mut view = MockCardPricesViewRepository::new();
        let probe = probe.clone();
        view.expect_refresh().returning(move || {
            let probe = probe.clone();
            Box::pin(async move {
                let index = probe.started.fetch_add(1, Ordering::SeqCst);
                probe.started_signal.notify_one();
                if hold_first && index == 0 {
                    probe.release.notified().await;
                }
                let fails = probe.failing.lock().unwrap().contains(&index);
                probe.finished.notify_one();
                if fails {
                    Err(AppError::from(InfraError::RepositoryError(
                        "refresh".into(),
                    )))
                } else {
                    Ok(())
                }
            })
        });
        CardPricesRefresher::spawn(Arc::new(view))
    }

    #[tokio::test]
    async fn a_burst_of_requests_during_a_refresh_triggers_a_single_further_refresh() {
        let probe = Arc::new(Probe::default());
        let refresher = refresher(&probe, true);

        refresher.request_card_prices_refresh();
        probe.started_signal.notified().await;
        for _ in 0..5 {
            refresher.request_card_prices_refresh();
        }
        probe.release.notify_one();
        probe.finished.notified().await;
        probe.finished.notified().await;
        tokio::time::sleep(Duration::from_millis(50)).await;

        assert_eq!(probe.started.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn a_request_returns_without_waiting_for_the_refresh() {
        let probe = Arc::new(Probe::default());
        let refresher = refresher(&probe, true);

        refresher.request_card_prices_refresh();
        probe.started_signal.notified().await;
        // The first refresh is still blocked: reaching the next line proves the request did not
        // wait for it.
        refresher.request_card_prices_refresh();

        assert_eq!(probe.started.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn a_failed_refresh_does_not_stop_the_refresher() {
        let probe = Arc::new(Probe::default());
        probe.failing.lock().unwrap().push(0);
        let refresher = refresher(&probe, false);

        refresher.request_card_prices_refresh();
        probe.finished.notified().await;
        refresher.request_card_prices_refresh();
        probe.finished.notified().await;

        assert_eq!(probe.started.load(Ordering::SeqCst), 2);
    }
}
