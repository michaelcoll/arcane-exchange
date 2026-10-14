use crate::application::card_import_job::CardImportJob;
use crate::application::use_case::RunCardImportUseCase;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;

/// Consumes `CardImportJob`s in series and runs them. An error on one job is logged and does
/// not stop the loop — the failure is already recorded on the `CardImport` row by `run()`.
pub struct CardImportWorker {
    run_use_case: Arc<dyn RunCardImportUseCase>,
}

impl CardImportWorker {
    pub fn new(run_use_case: Arc<dyn RunCardImportUseCase>) -> Self {
        Self { run_use_case }
    }

    pub async fn run(self, mut receiver: UnboundedReceiver<CardImportJob>) {
        while let Some(job) = receiver.recv().await {
            if let Err(e) = self.run_use_case.run(job).await {
                tracing::error!(error = %e, "card import job failed");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::error::{AppError, InfraError};
    use crate::application::use_case::MockRunCardImportUseCase;
    use crate::domain::card_import::CardImportId;
    use crate::domain::user::User;
    use std::sync::Mutex;
    use std::time::Duration;
    use tokio::sync::mpsc;

    fn job(import_id: CardImportId) -> CardImportJob {
        CardImportJob {
            import_id,
            user: User::for_testing(),
            cards: vec![],
        }
    }

    #[tokio::test]
    async fn runs_the_received_jobs_in_order_and_keeps_going_after_a_failed_one() {
        let ids = [
            CardImportId::new(),
            CardImportId::new(),
            CardImportId::new(),
        ];
        let failing_id = ids[0];
        let run_ids = Arc::new(Mutex::new(Vec::new()));

        let mut run_use_case = MockRunCardImportUseCase::new();
        let recorded = Arc::clone(&run_ids);
        run_use_case.expect_run().times(3).returning(move |job| {
            recorded.lock().unwrap().push(job.import_id);
            let failed = job.import_id == failing_id;
            Box::pin(async move {
                if failed {
                    Err(AppError::Infra(InfraError::RepositoryError(
                        "boom".to_string(),
                    )))
                } else {
                    Ok(())
                }
            })
        });

        let (sender, receiver) = mpsc::unbounded_channel();
        for id in ids {
            sender.send(job(id)).unwrap();
        }
        drop(sender);

        tokio::time::timeout(
            Duration::from_secs(5),
            CardImportWorker::new(Arc::new(run_use_case)).run(receiver),
        )
        .await
        .expect("the worker should stop once the channel is closed");

        assert_eq!(*run_ids.lock().unwrap(), ids);
    }

    #[tokio::test]
    async fn stops_when_the_channel_is_closed() {
        let mut run_use_case = MockRunCardImportUseCase::new();
        run_use_case.expect_run().times(0);

        let (sender, receiver) = mpsc::unbounded_channel::<CardImportJob>();
        let worker = tokio::spawn(CardImportWorker::new(Arc::new(run_use_case)).run(receiver));

        drop(sender);

        tokio::time::timeout(Duration::from_secs(5), worker)
            .await
            .expect("the worker should stop once the channel is closed")
            .unwrap();
    }
}
