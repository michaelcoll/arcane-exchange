use crate::application::caller::EdhRecCaller;
use crate::application::enrichment_queue::{Enricher, EnrichmentQueue};
use crate::application::error::AppError;
use crate::application::repository::{InclusionRateLookup, InclusionRateRepository};
use crate::application::use_case::EnqueueInclusionRateUpdateUseCase;
use crate::domain::card::CardId;
use async_trait::async_trait;
use std::sync::Arc;

/// Fetches a card name's inclusion rate on EDHREC (ADR-0027).
///
/// The queue is keyed by card, the rate by English name: `pending` offers one card per name
/// without a rate, and `resolve` stores the rate under that name. A name EDHREC does not know, or
/// a technical failure, writes nothing: the name stays pending and an existing rate is kept.
pub struct InclusionRateEnricher {
    inclusion_rate_repository: Arc<dyn InclusionRateRepository>,
    edh_rec_caller: Arc<dyn EdhRecCaller>,
}

impl InclusionRateEnricher {
    pub fn new(
        inclusion_rate_repository: Arc<dyn InclusionRateRepository>,
        edh_rec_caller: Arc<dyn EdhRecCaller>,
    ) -> Self {
        Self {
            inclusion_rate_repository,
            edh_rec_caller,
        }
    }
}

#[async_trait]
impl Enricher for InclusionRateEnricher {
    type Lookup = InclusionRateLookup;
    const NAME: &'static str = "Inclusion rate";

    async fn pending(&self) -> Result<Vec<(CardId, InclusionRateLookup)>, AppError> {
        self.inclusion_rate_repository
            .get_names_without_rate()
            .await
    }

    async fn resolve(&self, card_id: &CardId, lookup: InclusionRateLookup) -> Result<(), AppError> {
        let card_name = lookup.card_name;
        match self
            .edh_rec_caller
            .get_inclusion_rate(card_name.clone())
            .await?
        {
            Some(rate) => {
                self.inclusion_rate_repository
                    .save(card_name.clone(), rate)
                    .await?;
                tracing::info!("{} ({}) -> {:?}", card_name, card_id, rate);
            }
            None => tracing::warn!(
                "EDHREC does not know '{}' ({}), left pending",
                card_name,
                card_id
            ),
        }
        Ok(())
    }
}

#[async_trait]
impl EnqueueInclusionRateUpdateUseCase for EnrichmentQueue<InclusionRateEnricher> {
    async fn enqueue_pending_updates(&self) -> Result<usize, AppError> {
        self.enqueue_pending().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::caller::MockEdhRecCaller;
    use crate::application::error::InfraError;
    use crate::application::repository::MockInclusionRateRepository;
    use crate::domain::inclusion_rate::InclusionRate;
    use crate::domain::language_code::LanguageCode;

    fn card_id() -> CardId {
        CardId::new("FDN", "87", LanguageCode::EN)
    }

    fn lookup(card_name: &str) -> InclusionRateLookup {
        InclusionRateLookup {
            card_name: card_name.to_string(),
        }
    }

    fn rate() -> InclusionRate {
        InclusionRate {
            num_decks: 10,
            potential_decks: 100,
        }
    }

    fn enricher(
        repository: MockInclusionRateRepository,
        caller: MockEdhRecCaller,
    ) -> InclusionRateEnricher {
        InclusionRateEnricher::new(Arc::new(repository), Arc::new(caller))
    }

    #[tokio::test]
    async fn pending_lists_one_card_per_name_without_a_rate() {
        let mut repository = MockInclusionRateRepository::new();
        repository
            .expect_get_names_without_rate()
            .returning(|| Box::pin(async { Ok(vec![(card_id(), lookup("Sol Ring"))]) }));

        let pending = enricher(repository, MockEdhRecCaller::new())
            .pending()
            .await
            .unwrap();

        assert_eq!(pending, vec![(card_id(), lookup("Sol Ring"))]);
    }

    #[tokio::test]
    async fn resolve_stores_the_rate_found_by_edhrec_under_the_name() {
        let mut caller = MockEdhRecCaller::new();
        caller
            .expect_get_inclusion_rate()
            .withf(|name| name == "Sol Ring")
            .returning(|_| Box::pin(async { Ok(Some(rate())) }));
        let mut repository = MockInclusionRateRepository::new();
        repository
            .expect_save()
            .withf(|name, saved| name == "Sol Ring" && *saved == rate())
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(()) }));

        enricher(repository, caller)
            .resolve(&card_id(), lookup("Sol Ring"))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn resolve_writes_nothing_when_edhrec_does_not_know_the_name() {
        let mut caller = MockEdhRecCaller::new();
        caller
            .expect_get_inclusion_rate()
            .returning(|_| Box::pin(async { Ok(None) }));
        let mut repository = MockInclusionRateRepository::new();
        repository.expect_save().times(0);

        enricher(repository, caller)
            .resolve(&card_id(), lookup("Sol Ring"))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn resolve_writes_nothing_on_an_edhrec_error() {
        let mut caller = MockEdhRecCaller::new();
        caller.expect_get_inclusion_rate().returning(|_| {
            Box::pin(async { Err(AppError::Infra(InfraError::CallError("503".to_string()))) })
        });
        let mut repository = MockInclusionRateRepository::new();
        repository.expect_save().times(0);

        let result = enricher(repository, caller)
            .resolve(&card_id(), lookup("Sol Ring"))
            .await;

        assert!(result.is_err());
    }
}
