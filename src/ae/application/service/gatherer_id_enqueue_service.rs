use crate::application::error::{AppError, InfraError};
use crate::application::repository::CardRepository;
use crate::application::use_case::EnqueueGathererIdUpdateUseCase;
use crate::domain::card::CardId;
use async_trait::async_trait;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::UnboundedSender;

pub struct GathererIdEnqueueService {
    card_repository: Arc<dyn CardRepository>,
    sender: UnboundedSender<(CardId, String)>,
    dedup_set: Arc<Mutex<HashSet<CardId>>>,
}

impl GathererIdEnqueueService {
    pub fn new(
        card_repository: Arc<dyn CardRepository>,
        sender: UnboundedSender<(CardId, String)>,
        dedup_set: Arc<Mutex<HashSet<CardId>>>,
    ) -> Self {
        Self {
            card_repository,
            sender,
            dedup_set,
        }
    }
}

#[async_trait]
impl EnqueueGathererIdUpdateUseCase for GathererIdEnqueueService {
    async fn enqueue_pending_updates(&self) -> Result<usize, AppError> {
        let cards = self.card_repository.get_all_without_gatherer_id().await?;
        let mut enqueued = 0;
        let mut set = self.dedup_set.lock().unwrap();
        for (card_id, name) in cards {
            if set.insert(card_id.clone()) {
                if self.sender.send((card_id, name)).is_err() {
                    return Err(InfraError::QueueError(
                        "Worker channel closed, cannot enqueue card".into(),
                    )
                    .into());
                } else {
                    enqueued += 1;
                }
            }
        }
        Ok(enqueued)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::MockCardRepository;
    use crate::domain::language_code::LanguageCode;
    use crate::domain::set_name::SetCode;
    use tokio::sync::mpsc::unbounded_channel;

    fn make_card_id(n: &str) -> CardId {
        CardId::new(SetCode::new("FDN"), n, LanguageCode::FR, false)
    }

    #[tokio::test]
    async fn enqueue_returns_count_of_newly_enqueued_cards() {
        let mut card_repository = MockCardRepository::new();
        card_repository
            .expect_get_all_without_gatherer_id()
            .returning(|| {
                Box::pin(async {
                    Ok(vec![
                        (
                            CardId::new(SetCode::new("FDN"), "0", LanguageCode::FR, false),
                            "Card A".to_string(),
                        ),
                        (
                            CardId::new(SetCode::new("FDN"), "1", LanguageCode::FR, false),
                            "Card B".to_string(),
                        ),
                    ])
                })
            });

        let (tx, _rx) = unbounded_channel();
        let dedup_set = Arc::new(Mutex::new(HashSet::new()));
        let service = GathererIdEnqueueService::new(Arc::new(card_repository), tx, dedup_set);

        let result = service.enqueue_pending_updates().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);
    }

    #[tokio::test]
    async fn enqueue_deduplicates_already_enqueued_cards() {
        let mut card_repository = MockCardRepository::new();
        card_repository
            .expect_get_all_without_gatherer_id()
            .returning(|| {
                Box::pin(async {
                    Ok(vec![
                        (
                            CardId::new(SetCode::new("FDN"), "0", LanguageCode::FR, false),
                            "Card A".to_string(),
                        ),
                        (
                            CardId::new(SetCode::new("FDN"), "1", LanguageCode::FR, false),
                            "Card B".to_string(),
                        ),
                    ])
                })
            });

        let (tx, _rx) = unbounded_channel();
        let dedup_set = Arc::new(Mutex::new(HashSet::new()));
        dedup_set.lock().unwrap().insert(make_card_id("0"));

        let service = GathererIdEnqueueService::new(Arc::new(card_repository), tx, dedup_set);

        let result = service.enqueue_pending_updates().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
    }

    #[tokio::test]
    async fn enqueue_returns_error_on_repository_failure() {
        let mut card_repository = MockCardRepository::new();
        card_repository
            .expect_get_all_without_gatherer_id()
            .returning(|| {
                Box::pin(async {
                    Err(AppError::Infra(InfraError::RepositoryError(
                        "DB error".to_string(),
                    )))
                })
            });

        let (tx, _rx) = unbounded_channel();
        let dedup_set = Arc::new(Mutex::new(HashSet::new()));
        let service = GathererIdEnqueueService::new(Arc::new(card_repository), tx, dedup_set);

        let result = service.enqueue_pending_updates().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn enqueue_returns_zero_for_empty_card_list() {
        let mut card_repository = MockCardRepository::new();
        card_repository
            .expect_get_all_without_gatherer_id()
            .returning(|| Box::pin(async { Ok(vec![]) }));

        let (tx, _rx) = unbounded_channel();
        let dedup_set = Arc::new(Mutex::new(HashSet::new()));
        let service = GathererIdEnqueueService::new(Arc::new(card_repository), tx, dedup_set);

        let result = service.enqueue_pending_updates().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }
}
