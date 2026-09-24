use crate::application::caller::GathererCaller;
use crate::application::error::AppError;
use crate::application::repository::CardRepository;
use crate::application::service::enrichment_queue::{Enricher, EnrichmentQueue};
use crate::application::use_case::EnqueueGathererIdUpdateUseCase;
use crate::domain::card::CardId;
use async_trait::async_trait;
use std::sync::Arc;

/// Resolves a card's Gatherer id from its identity and name.
pub struct GathererIdEnricher {
    card_repository: Arc<dyn CardRepository>,
    gatherer_caller: Arc<dyn GathererCaller>,
}

impl GathererIdEnricher {
    pub fn new(
        card_repository: Arc<dyn CardRepository>,
        gatherer_caller: Arc<dyn GathererCaller>,
    ) -> Self {
        Self {
            card_repository,
            gatherer_caller,
        }
    }
}

#[async_trait]
impl Enricher for GathererIdEnricher {
    /// The card's name.
    type Job = String;
    const NAME: &'static str = "Gatherer id";

    async fn pending(&self) -> Result<Vec<(CardId, String)>, AppError> {
        self.card_repository.get_all_without_gatherer_id().await
    }

    /// A card Gatherer does not know is left without id, which is not an error.
    async fn resolve(&self, card_id: &CardId, name: String) -> Result<(), AppError> {
        let gatherer_id = self
            .gatherer_caller
            .get_gatherer_id(
                card_id.set_code.clone(),
                card_id.collector_number.clone(),
                card_id.language_code.clone(),
                name,
            )
            .await?;

        match gatherer_id {
            Some(id) => {
                self.card_repository
                    .update_gatherer_id(card_id.clone(), Some(id))
                    .await?;
                tracing::info!("{} ✓", card_id);
            }
            None => {
                tracing::trace!("No Gatherer ID found for card {}, leaving empty", card_id);
            }
        }
        Ok(())
    }
}

#[async_trait]
impl EnqueueGathererIdUpdateUseCase for EnrichmentQueue<GathererIdEnricher> {
    async fn enqueue_pending_updates(&self) -> Result<usize, AppError> {
        self.enqueue_pending().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::caller::MockGathererCaller;
    use crate::application::error::InfraError;
    use crate::application::repository::MockCardRepository;
    use crate::domain::language_code::LanguageCode;
    use crate::domain::set_name::SetCode;

    fn make_card_id(n: &str) -> CardId {
        CardId::new(SetCode::new("FDN"), n, LanguageCode::FR)
    }

    fn enricher(
        card_repository: MockCardRepository,
        gatherer_caller: MockGathererCaller,
    ) -> GathererIdEnricher {
        GathererIdEnricher::new(Arc::new(card_repository), Arc::new(gatherer_caller))
    }

    #[tokio::test]
    async fn pending_lists_cards_without_gatherer_id() {
        let mut card_repository = MockCardRepository::new();
        card_repository
            .expect_get_all_without_gatherer_id()
            .returning(|| Box::pin(async { Ok(vec![(make_card_id("0"), "Card A".to_string())]) }));
        let enricher = enricher(card_repository, MockGathererCaller::new());

        let pending = enricher.pending().await.unwrap();

        assert_eq!(pending, vec![(make_card_id("0"), "Card A".to_string())]);
    }

    #[tokio::test]
    async fn resolve_stores_the_gatherer_id_found() {
        let mut card_repository = MockCardRepository::new();
        let mut gatherer_caller = MockGathererCaller::new();
        gatherer_caller
            .expect_get_gatherer_id()
            .withf(|_, collector_number, _, name| {
                collector_number == "0" && name == "Goblin Boarders"
            })
            .returning(|_, _, _, _| Box::pin(async { Ok(Some("abc123".to_string())) }));
        card_repository
            .expect_update_gatherer_id()
            .withf(|id, gatherer_id| {
                *id == make_card_id("0") && gatherer_id.as_deref() == Some("abc123")
            })
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(()) }));
        let enricher = enricher(card_repository, gatherer_caller);

        enricher
            .resolve(&make_card_id("0"), "Goblin Boarders".to_string())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn resolve_leaves_the_card_untouched_when_gatherer_does_not_know_it() {
        let mut card_repository = MockCardRepository::new();
        let mut gatherer_caller = MockGathererCaller::new();
        gatherer_caller
            .expect_get_gatherer_id()
            .returning(|_, _, _, _| Box::pin(async { Ok(None) }));
        card_repository.expect_update_gatherer_id().times(0);
        let enricher = enricher(card_repository, gatherer_caller);

        enricher
            .resolve(&make_card_id("0"), "Unknown Card".to_string())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn resolve_fails_without_update_on_gatherer_error() {
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
        let enricher = enricher(card_repository, gatherer_caller);

        let result = enricher
            .resolve(&make_card_id("0"), "Name".to_string())
            .await;

        assert!(result.is_err());
    }
}
