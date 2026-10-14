use crate::application::card_import_job::CardImportJob;
use crate::application::error::{AppError, InfraError};
use crate::application::manabox_csv::{ParsedCollection, parse_cards};
use crate::application::repository::CardImportRepository;
use crate::application::use_case::ImportCardUseCase;
use crate::domain::card_import::{
    CardImport, CardImportId, CardImportStatus, MAX_STORED_LINE_ERRORS,
};
use crate::domain::user::User;
use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

pub struct ImportCardService {
    card_import_repository: Arc<dyn CardImportRepository>,
    sender: UnboundedSender<CardImportJob>,
}

impl ImportCardService {
    pub fn new(
        card_import_repository: Arc<dyn CardImportRepository>,
        sender: UnboundedSender<CardImportJob>,
    ) -> Self {
        Self {
            card_import_repository,
            sender,
        }
    }
}

#[async_trait]
impl ImportCardUseCase for ImportCardService {
    async fn start_import(&self, csv: &str, user: User) -> Result<CardImportId, AppError> {
        let owned_csv = csv.to_string();
        let parsed = tokio::task::spawn_blocking(move || parse_cards(&owned_csv))
            .await
            .map_err(|e| {
                AppError::Infra(InfraError::RepositoryError(format!(
                    "import parsing task panicked: {e}"
                )))
            })??;

        let ParsedCollection {
            cards,
            errors,
            source_lines,
        } = parsed;

        let line_error_count = errors.len() as u32;
        let line_errors = errors.into_iter().take(MAX_STORED_LINE_ERRORS).collect();

        let import = CardImport {
            id: CardImportId::new(),
            user_id: user.id.clone(),
            status: CardImportStatus::Pending,
            source_lines: source_lines as u32,
            total_lines: cards.len() as u32,
            processed_lines: 0,
            line_errors,
            line_error_count,
            error_message: None,
            error_code: None,
            created_at: Utc::now(),
            finished_at: None,
        };

        // Fails with `ImportAlreadyRunning` (409) if the user already has an active import —
        // before any other effect.
        self.card_import_repository.create(&import).await?;

        self.card_import_repository.purge_old(&user.id, 10).await?;

        self.sender
            .send(CardImportJob {
                import_id: import.id,
                user,
                cards,
            })
            .map_err(|_| {
                AppError::Infra(InfraError::QueueError(
                    "failed to enqueue card import job".to_string(),
                ))
            })?;

        Ok(import.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::MockCardImportRepository;
    use crate::domain::error::FunctionalError;
    use mockall::predicate::eq;
    use tokio::sync::mpsc;

    fn valid_csv() -> &'static str {
        "Binder Name,Binder Type,Name,Set code,Set name,Collector number,Foil,Rarity,Quantity,ManaBox ID,Scryfall ID,Purchase price,Misprint,Altered,Signed,Condition,Language,Proxy,Purchase price currency,Added\n\
         bulk,binder,Goblin Boarders,FDN,Foundations,87,normal,common,3,101506,4409a063-bf2a-4a49-803e-3ce6bd474353,0.08,false,false,false,near_mint,fr,false,EUR,2026-02-05T20:44:45.815Z"
    }

    #[tokio::test]
    async fn start_import_creates_the_import_purges_old_ones_and_enqueues_the_job() {
        let mut card_import_repository = MockCardImportRepository::new();
        card_import_repository
            .expect_create()
            .withf(|import| {
                import.status == CardImportStatus::Pending
                    && import.total_lines == 1
                    && import.processed_lines == 0
            })
            .returning(|_| Box::pin(async { Ok(()) }));
        card_import_repository
            .expect_purge_old()
            .with(eq(User::for_testing().id), eq(10i64))
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(()) }));

        let (sender, mut receiver) = mpsc::unbounded_channel();
        let service = ImportCardService::new(Arc::new(card_import_repository), sender);

        let id = service
            .start_import(valid_csv(), User::for_testing())
            .await
            .unwrap();

        let job = receiver.try_recv().expect("job should be enqueued");
        assert_eq!(job.import_id, id);
        assert_eq!(job.cards.len(), 1);
    }

    #[tokio::test]
    async fn start_import_propagates_already_running_before_purging_or_enqueuing() {
        let mut card_import_repository = MockCardImportRepository::new();
        card_import_repository.expect_create().returning(|_| {
            Box::pin(async { Err(AppError::Functional(FunctionalError::ImportAlreadyRunning)) })
        });
        card_import_repository.expect_purge_old().times(0);

        let (sender, _receiver) = mpsc::unbounded_channel();
        let service = ImportCardService::new(Arc::new(card_import_repository), sender);

        let result = service.start_import(valid_csv(), User::for_testing()).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::ImportAlreadyRunning))
        ));
    }

    #[tokio::test]
    async fn start_import_fails_with_a_queue_error_when_the_worker_is_gone() {
        let mut card_import_repository = MockCardImportRepository::new();
        card_import_repository
            .expect_create()
            .returning(|_| Box::pin(async { Ok(()) }));
        card_import_repository
            .expect_purge_old()
            .returning(|_, _| Box::pin(async { Ok(()) }));

        let (sender, receiver) = mpsc::unbounded_channel();
        drop(receiver);
        let service = ImportCardService::new(Arc::new(card_import_repository), sender);

        let result = service.start_import(valid_csv(), User::for_testing()).await;

        assert!(matches!(
            result,
            Err(AppError::Infra(InfraError::QueueError(_)))
        ));
    }

    #[tokio::test]
    async fn start_import_does_not_create_an_import_when_parsing_fails() {
        let mut card_import_repository = MockCardImportRepository::new();
        card_import_repository.expect_create().times(0);
        card_import_repository.expect_purge_old().times(0);

        let (sender, _receiver) = mpsc::unbounded_channel();
        let service = ImportCardService::new(Arc::new(card_import_repository), sender);

        let result = service
            .start_import("Invalid,Data", User::for_testing())
            .await;

        assert!(result.is_err());
    }
}
