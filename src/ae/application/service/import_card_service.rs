use crate::application::card_import_job::CardImportJob;
use crate::application::error::{AppError, InfraError};
use crate::application::imported_card::ImportedCard;
use crate::application::repository::{
    CardImportRepository, CardPricesViewRepository, CardRepository, SetNameRepository,
    TradingBinderRepository,
};
use crate::application::service::parse_service::{ParsedCollection, parse_cards};
use crate::application::use_case::{
    EnqueueCardMarketIdUpdateUseCase, EnqueueGathererIdUpdateUseCase, ImportCardUseCase,
    RunCardImportUseCase,
};
use crate::domain::card_import::{
    CardImport, CardImportId, CardImportStatus, MAX_STORED_LINE_ERRORS,
};
use crate::domain::error::FunctionalError;
use crate::domain::set_name::SetName;
use crate::domain::user::User;
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

/// Number of cards written per `save_all` call: keeps a single `collection_entry` insert (9
/// bound params per row) comfortably under Postgres' 65535 bound-parameter limit, and gives the
/// progress counter a granularity to update against.
const CHUNK_SIZE: usize = 500;

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

pub struct RunCardImportService {
    card_repository: Arc<dyn CardRepository>,
    set_name_repository: Arc<dyn SetNameRepository>,
    card_import_repository: Arc<dyn CardImportRepository>,
    enqueue_cardmarket_ids: Arc<dyn EnqueueCardMarketIdUpdateUseCase>,
    enqueue_gatherer_ids: Arc<dyn EnqueueGathererIdUpdateUseCase>,
    card_prices_view_repository: Arc<dyn CardPricesViewRepository>,
    trading_binder_repository: Arc<dyn TradingBinderRepository>,
}

impl RunCardImportService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        card_repository: Arc<dyn CardRepository>,
        set_name_repository: Arc<dyn SetNameRepository>,
        card_import_repository: Arc<dyn CardImportRepository>,
        enqueue_cardmarket_ids: Arc<dyn EnqueueCardMarketIdUpdateUseCase>,
        enqueue_gatherer_ids: Arc<dyn EnqueueGathererIdUpdateUseCase>,
        card_prices_view_repository: Arc<dyn CardPricesViewRepository>,
        trading_binder_repository: Arc<dyn TradingBinderRepository>,
    ) -> Self {
        Self {
            card_repository,
            set_name_repository,
            card_import_repository,
            enqueue_cardmarket_ids,
            enqueue_gatherer_ids,
            card_prices_view_repository,
            trading_binder_repository,
        }
    }

    async fn run_inner(
        &self,
        import_id: &CardImportId,
        user: &User,
        cards: &[ImportedCard],
    ) -> Result<(), AppError> {
        if cards.is_empty() {
            // The collection is deliberately left untouched: no `delete_all` on a fully-invalid
            // file.
            return Err(FunctionalError::WrongFormat("no valid line in file".to_string()).into());
        }

        self.card_repository.delete_all(user.clone()).await?;

        let sets = distinct_sets(cards);
        self.set_name_repository.save_all(&sets).await?;

        let mut processed = 0u32;
        for chunk in cards.chunks(CHUNK_SIZE) {
            self.card_repository.save_all(user, chunk).await?;
            processed += chunk.len() as u32;
            self.card_import_repository
                .update_progress(import_id, processed)
                .await?;
        }

        self.trading_binder_repository
            .purge_missing(&user.id)
            .await?;

        self.enqueue_cardmarket_ids
            .enqueue_pending_updates()
            .await?;
        self.enqueue_gatherer_ids.enqueue_pending_updates().await?;
        self.card_prices_view_repository.refresh().await?;

        Ok(())
    }
}

#[async_trait]
impl RunCardImportUseCase for RunCardImportService {
    async fn run(&self, job: CardImportJob) -> Result<(), AppError> {
        let CardImportJob {
            import_id,
            user,
            cards,
        } = job;

        self.card_import_repository.mark_running(&import_id).await?;

        let outcome = self.run_inner(&import_id, &user, &cards).await;

        // `finish` is the only way out of `running` short of a server restart (`fail_all_active`
        // at boot) — if it fails itself, that would otherwise silently strand the import (and,
        // with it, the user's one-active-import slot) until then. Log it explicitly rather than
        // propagate it and lose the original outcome, which is what the caller (and the row, had
        // `finish` succeeded) actually needs to reflect.
        let (status, error_message) = match &outcome {
            Ok(()) => (CardImportStatus::Completed, None),
            Err(e) => {
                let message: String = e.clone().into();
                tracing::error!(import_id = %import_id, error = %message, "card import failed");
                (CardImportStatus::Failed, Some(message))
            }
        };
        if let Err(finish_err) = self
            .card_import_repository
            .finish(&import_id, status, error_message.as_deref())
            .await
        {
            let finish_message: String = finish_err.into();
            tracing::error!(
                import_id = %import_id,
                error = %finish_message,
                "failed to record the outcome of a card import; it will stay stuck until the \
                 next server restart clears it via fail_all_active"
            );
        }

        outcome
    }
}

fn distinct_sets(cards: &[ImportedCard]) -> Vec<SetName> {
    let mut seen = HashSet::with_capacity(cards.len());
    cards
        .iter()
        .filter(|imported| seen.insert(imported.card.id.set_code.clone()))
        .map(|imported| imported.card.set_name.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::{
        MockCardImportRepository, MockCardPricesViewRepository, MockCardRepository,
        MockSetNameRepository, MockTradingBinderRepository,
    };
    use crate::application::use_case::{
        MockEnqueueCardMarketIdUpdateUseCase, MockEnqueueGathererIdUpdateUseCase,
    };
    use crate::domain::card::{Card, CollectionEntry};
    use crate::domain::language_code::LanguageCode;
    use crate::domain::rarity_code::RarityCode;
    use crate::domain::set_name::SetCode;
    use chrono::{DateTime, Utc};
    use mockall::predicate::eq;
    use tokio::sync::mpsc;
    use uuid::Uuid;

    fn imported_card(collector_number: &str, binder_name: Option<&str>) -> ImportedCard {
        ImportedCard {
            card: Card::new_full(
                SetCode::new("FDN"),
                "Foundations",
                collector_number,
                LanguageCode::FR,
                false,
                "Goblin Boarders",
                RarityCode::C,
                Uuid::parse_str("4409a063-bf2a-4a49-803e-3ce6bd474353").unwrap(),
                None,
                None,
                CollectionEntry::Mine {
                    quantity: 3,
                    purchase_price: 8,
                    added_at: DateTime::parse_from_rfc3339("2026-02-05T20:44:45.815Z")
                        .unwrap()
                        .with_timezone(&Utc),
                    reserved: false,
                },
            ),
            binder_name: binder_name.map(str::to_string),
        }
    }

    fn valid_csv() -> &'static str {
        "Binder Name,Binder Type,Name,Set code,Set name,Collector number,Foil,Rarity,Quantity,ManaBox ID,Scryfall ID,Purchase price,Misprint,Altered,Condition,Language,Purchase price currency,Added\n\
         bulk,binder,Goblin Boarders,FDN,Foundations,87,normal,common,3,101506,4409a063-bf2a-4a49-803e-3ce6bd474353,0.08,false,false,near_mint,fr,EUR,2026-02-05T20:44:45.815Z"
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

    fn run_service(
        card_repository: MockCardRepository,
        set_name_repository: MockSetNameRepository,
        card_import_repository: MockCardImportRepository,
        trading_binder_repository: MockTradingBinderRepository,
        enqueue_cardmarket: MockEnqueueCardMarketIdUpdateUseCase,
        enqueue_gatherer: MockEnqueueGathererIdUpdateUseCase,
        card_prices_view_repository: MockCardPricesViewRepository,
    ) -> RunCardImportService {
        RunCardImportService::new(
            Arc::new(card_repository),
            Arc::new(set_name_repository),
            Arc::new(card_import_repository),
            Arc::new(enqueue_cardmarket),
            Arc::new(enqueue_gatherer),
            Arc::new(card_prices_view_repository),
            Arc::new(trading_binder_repository),
        )
    }

    #[tokio::test]
    async fn run_writes_cards_in_chunks_updates_progress_and_completes() {
        let import_id = CardImportId::new();
        let cards: Vec<ImportedCard> = (0..3)
            .map(|i| imported_card(&i.to_string(), Some("bulk")))
            .collect();

        let mut card_repository = MockCardRepository::new();
        card_repository
            .expect_delete_all()
            .with(eq(User::for_testing()))
            .times(1)
            .returning(|_| Box::pin(async { Ok(()) }));
        card_repository
            .expect_save_all()
            .times(1) // 3 cards fit in a single CHUNK_SIZE=500 batch
            .returning(|_, _| Box::pin(async { Ok(()) }));

        let mut set_name_repository = MockSetNameRepository::new();
        set_name_repository
            .expect_save_all()
            .times(1)
            .returning(|_| Box::pin(async { Ok(()) }));

        let mut card_import_repository = MockCardImportRepository::new();
        card_import_repository
            .expect_mark_running()
            .with(eq(import_id))
            .times(1)
            .returning(|_| Box::pin(async { Ok(()) }));
        card_import_repository
            .expect_update_progress()
            .with(eq(import_id), eq(3u32))
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(()) }));
        card_import_repository
            .expect_finish()
            .withf(move |id, status, msg| {
                *id == import_id && *status == CardImportStatus::Completed && msg.is_none()
            })
            .times(1)
            .returning(|_, _, _| Box::pin(async { Ok(()) }));

        let mut trading_binder_repository = MockTradingBinderRepository::new();
        trading_binder_repository
            .expect_purge_missing()
            .times(1)
            .returning(|_| Box::pin(async { Ok(()) }));

        let mut enqueue_cardmarket = MockEnqueueCardMarketIdUpdateUseCase::new();
        enqueue_cardmarket
            .expect_enqueue_pending_updates()
            .times(1)
            .returning(|| Box::pin(async { Ok(0) }));
        let mut enqueue_gatherer = MockEnqueueGathererIdUpdateUseCase::new();
        enqueue_gatherer
            .expect_enqueue_pending_updates()
            .times(1)
            .returning(|| Box::pin(async { Ok(0) }));

        let mut card_prices_view_repository = MockCardPricesViewRepository::new();
        card_prices_view_repository
            .expect_refresh()
            .times(1)
            .returning(|| Box::pin(async { Ok(()) }));

        let service = run_service(
            card_repository,
            set_name_repository,
            card_import_repository,
            trading_binder_repository,
            enqueue_cardmarket,
            enqueue_gatherer,
            card_prices_view_repository,
        );

        let result = service
            .run(CardImportJob {
                import_id,
                user: User::for_testing(),
                cards,
            })
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn run_marks_failed_without_deleting_the_collection_when_no_cards() {
        let import_id = CardImportId::new();

        let mut card_repository = MockCardRepository::new();
        card_repository.expect_delete_all().times(0);
        card_repository.expect_save_all().times(0);

        let mut card_import_repository = MockCardImportRepository::new();
        card_import_repository
            .expect_mark_running()
            .times(1)
            .returning(|_| Box::pin(async { Ok(()) }));
        card_import_repository
            .expect_finish()
            .withf(move |id, status, msg| {
                *id == import_id && *status == CardImportStatus::Failed && msg.is_some()
            })
            .times(1)
            .returning(|_, _, _| Box::pin(async { Ok(()) }));

        let service = run_service(
            card_repository,
            MockSetNameRepository::new(),
            card_import_repository,
            MockTradingBinderRepository::new(),
            MockEnqueueCardMarketIdUpdateUseCase::new(),
            MockEnqueueGathererIdUpdateUseCase::new(),
            MockCardPricesViewRepository::new(),
        );

        let result = service
            .run(CardImportJob {
                import_id,
                user: User::for_testing(),
                cards: vec![],
            })
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn run_marks_failed_when_a_write_fails() {
        let import_id = CardImportId::new();
        let cards = vec![imported_card("87", Some("bulk"))];

        let mut card_repository = MockCardRepository::new();
        card_repository
            .expect_delete_all()
            .returning(|_| Box::pin(async { Ok(()) }));
        card_repository.expect_save_all().returning(|_, _| {
            Box::pin(async {
                Err(AppError::Infra(InfraError::RepositoryError(
                    "boom".to_string(),
                )))
            })
        });

        let mut set_name_repository = MockSetNameRepository::new();
        set_name_repository
            .expect_save_all()
            .returning(|_| Box::pin(async { Ok(()) }));

        let mut card_import_repository = MockCardImportRepository::new();
        card_import_repository
            .expect_mark_running()
            .returning(|_| Box::pin(async { Ok(()) }));
        card_import_repository
            .expect_finish()
            .withf(move |id, status, msg| {
                *id == import_id && *status == CardImportStatus::Failed && msg.is_some()
            })
            .times(1)
            .returning(|_, _, _| Box::pin(async { Ok(()) }));

        let service = run_service(
            card_repository,
            set_name_repository,
            card_import_repository,
            MockTradingBinderRepository::new(),
            MockEnqueueCardMarketIdUpdateUseCase::new(),
            MockEnqueueGathererIdUpdateUseCase::new(),
            MockCardPricesViewRepository::new(),
        );

        let result = service
            .run(CardImportJob {
                import_id,
                user: User::for_testing(),
                cards,
            })
            .await;

        assert!(result.is_err());
    }
}
