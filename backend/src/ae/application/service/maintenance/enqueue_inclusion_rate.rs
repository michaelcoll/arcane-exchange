use crate::application::caller::EdhRecCaller;
use crate::application::enrichment_queue::{Enricher, EnrichmentQueue};
use crate::application::error::AppError;
use crate::application::repository::{InclusionRateLookup, InclusionRateRepository};
use crate::application::use_case::EnqueueInclusionRateUpdateUseCase;
use crate::domain::card::CardId;
use crate::domain::inclusion_rate::InclusionRate;
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

    /// One card per name present in at least one collection, rate or not.
    async fn in_collections(&self) -> Result<Vec<(CardId, InclusionRateLookup)>, AppError> {
        self.inclusion_rate_repository
            .get_names_in_collections()
            .await
    }
}

/// The log line of a resolved name: `Sol Ring -> 0,3 %`, or `Sol Ring -> —` when no deck could
/// play the card.
fn resolved_line(card_name: &str, rate: &InclusionRate) -> String {
    match rate.percentage() {
        Some(percentage) => {
            let percentage = format!("{percentage:.1}").replace('.', ",");
            format!("{card_name} -> {percentage} %")
        }
        None => format!("{card_name} -> —"),
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
                tracing::info!("{}", resolved_line(&card_name, &rate));
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

    async fn enqueue_collected_names(&self) -> Result<usize, AppError> {
        let names = self.enricher().in_collections().await?;
        self.enqueue(names)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::caller::MockEdhRecCaller;
    use crate::application::error::InfraError;
    use crate::application::repository::{
        MockCardPricesViewRepository, MockInclusionRateRepository,
    };
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

    #[test]
    fn the_log_line_gives_the_percentage_with_one_decimal_and_a_comma() {
        let rate = InclusionRate {
            num_decks: 2445,
            potential_decks: 900_894,
        };

        assert_eq!(
            resolved_line("Screeching Soulbreaker", &rate),
            "Screeching Soulbreaker -> 0,3 %"
        );
    }

    #[test]
    fn the_log_line_has_a_dash_when_no_deck_could_play_the_card() {
        let rate = InclusionRate {
            num_decks: 0,
            potential_decks: 0,
        };

        assert_eq!(resolved_line("Sol Ring", &rate), "Sol Ring -> —");
    }

    #[test]
    fn the_log_line_keeps_the_dots_of_the_card_name() {
        let rate = InclusionRate {
            num_decks: 1,
            potential_decks: 4,
        };

        assert_eq!(
            resolved_line("Dr. Julius Jumblemorph", &rate),
            "Dr. Julius Jumblemorph -> 25,0 %"
        );
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

    #[tokio::test]
    async fn collection_updates_queue_one_card_per_name_in_a_collection_rate_or_not() {
        let mut repository = MockInclusionRateRepository::new();
        repository.expect_get_names_in_collections().returning(|| {
            Box::pin(async {
                Ok(vec![
                    (card_id(), lookup("Sol Ring")),
                    (
                        CardId::new("FDN", "3", LanguageCode::EN),
                        lookup("Counterspell"),
                    ),
                ])
            })
        });
        repository.expect_get_names_without_rate().times(0);
        let (queue, _worker) = EnrichmentQueue::new(
            enricher(repository, MockEdhRecCaller::new()),
            Arc::new(MockCardPricesViewRepository::new()),
        );

        assert_eq!(queue.enqueue_collected_names().await.unwrap(), 2);
        // Already in flight: not queued twice.
        assert_eq!(queue.enqueue_collected_names().await.unwrap(), 0);
    }
}
