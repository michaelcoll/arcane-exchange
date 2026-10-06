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
