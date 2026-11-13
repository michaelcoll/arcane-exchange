use crate::application::error::AppError;
use crate::application::repository::{CardPricesRefreshRequester, CollectionEntryRepository};
use crate::application::use_case::{CreateCardUseCase, ValidateScanBatchUseCase};
use crate::domain::binder::BinderId;
use crate::domain::collection_entry::validate_quantity;
use crate::domain::error::FunctionalError;
use crate::domain::purchase::PurchaseId;
use crate::domain::scan_batch::{ScanBatchCopies, ScanBatchId, ScanBatchLine, ScanBatchOutcome};
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct ValidateScanBatchService {
    create_card: Arc<dyn CreateCardUseCase>,
    repository: Arc<dyn CollectionEntryRepository>,
    card_prices_refresh: Arc<dyn CardPricesRefreshRequester>,
}

impl ValidateScanBatchService {
    pub fn new(
        create_card: Arc<dyn CreateCardUseCase>,
        repository: Arc<dyn CollectionEntryRepository>,
        card_prices_refresh: Arc<dyn CardPricesRefreshRequester>,
    ) -> Self {
        Self {
            create_card,
            repository,
            card_prices_refresh,
        }
    }
}

#[async_trait]
impl ValidateScanBatchUseCase for ValidateScanBatchService {
    #[tracing::instrument(skip(self, lines), fields(lines = lines.len()))]
    async fn validate_scan_batch(
        &self,
        user_id: &UserId,
        batch_id: ScanBatchId,
        binder_id: Option<BinderId>,
        sealed_product_id: Option<PurchaseId>,
        lines: Vec<ScanBatchLine>,
    ) -> Result<ScanBatchOutcome, AppError> {
        // A batch already recorded answers what its first send answered, whatever this send holds.
        if let Some(recorded) = self.repository.find_scan_batch(user_id, batch_id).await? {
            return Ok(recorded);
        }
        if lines.is_empty() {
            return Err(FunctionalError::InvalidRequest(
                "a scan batch has at least one line".to_string(),
            )
            .into());
        }
        // Every quantity is checked before any Card is created.
        let quantities = lines
            .iter()
            .map(|line| validate_quantity(line.quantity))
            .collect::<Result<Vec<_>, _>>()?;
        // The destination is read before any Card is created: a refused batch writes nothing.
        self.repository
            .ensure_scan_batch_destination(user_id, binder_id, sealed_product_id)
            .await?;

        let mut copies = Vec::with_capacity(lines.len());
        let mut missing = Vec::new();
        for (line, quantity) in lines.into_iter().zip(quantities) {
            match self.create_card.create_card(line.card_id.clone()).await {
                Ok(card_id) => copies.push(ScanBatchCopies {
                    card_id,
                    foil: line.foil,
                    quantity,
                }),
                Err(AppError::Functional(FunctionalError::CardNotInCatalog)) => {
                    missing.push(line.card_id);
                }
                Err(error) => return Err(error),
            }
        }
        if !missing.is_empty() {
            return Err(FunctionalError::CardsNotInCatalog(missing).into());
        }

        let outcome = self
            .repository
            .add_scan_batch(user_id, batch_id, binder_id, sealed_product_id, &copies)
            .await?;
        if !outcome.already_validated {
            self.card_prices_refresh.request_card_prices_refresh();
        }
        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::MockCollectionEntryRepository;
    use crate::application::service::collection::test_fixtures::{creating_card, refresh};
    use crate::application::use_case::MockCreateCardUseCase;
    use crate::domain::card::CardId;
    use crate::domain::language_code::LanguageCode;
    use uuid::Uuid;

    fn batch_id() -> ScanBatchId {
        ScanBatchId(Uuid::from_u128(3))
    }

    fn line(collector_number: &str, foil: bool, quantity: i32) -> ScanBatchLine {
        ScanBatchLine {
            card_id: CardId::new("MKM", collector_number, LanguageCode::FR),
            foil,
            quantity,
        }
    }

    /// A repository that knows no batch and accepts any destination.
    fn repository() -> MockCollectionEntryRepository {
        let mut repository = MockCollectionEntryRepository::new();
        repository
            .expect_find_scan_batch()
            .returning(|_, _| Box::pin(async { Ok(None) }));
        repository
            .expect_ensure_scan_batch_destination()
            .returning(|_, _, _| Box::pin(async { Ok(()) }));
        repository
    }

    fn written(copies_added: u32) -> ScanBatchOutcome {
        ScanBatchOutcome {
            copies_added,
            already_validated: false,
        }
    }

    fn service(
        create_card: MockCreateCardUseCase,
        repository: MockCollectionEntryRepository,
        refresh: crate::application::repository::MockCardPricesRefreshRequester,
    ) -> ValidateScanBatchService {
        ValidateScanBatchService::new(
            Arc::new(create_card),
            Arc::new(repository),
            Arc::new(refresh),
        )
    }

    #[tokio::test]
    async fn creates_each_card_writes_the_batch_once_and_requests_a_single_refresh() {
        let binder_id = BinderId(Uuid::from_u128(5));
        let sealed_product_id = PurchaseId(Uuid::from_u128(7));
        let mut repository = repository();
        repository
            .expect_add_scan_batch()
            .withf(move |user_id, id, binder, sealed_product, lines| {
                user_id.as_str() == "user_1"
                    && *id == batch_id()
                    && *binder == Some(binder_id)
                    && *sealed_product == Some(sealed_product_id)
                    // The ids the card creation returned, not the ones the batch named.
                    && lines
                        == [
                            ScanBatchCopies {
                                card_id: CardId::new("MKM", "12", LanguageCode::FR),
                                foil: false,
                                quantity: 2,
                            },
                            ScanBatchCopies {
                                card_id: CardId::new("MKM", "13", LanguageCode::FR),
                                foil: true,
                                quantity: 1,
                            },
                        ]
            })
            .times(1)
            .returning(|_, _, _, _, _| Box::pin(async { Ok(written(3)) }));
        let service = service(creating_card(2), repository, refresh(1));

        let outcome = service
            .validate_scan_batch(
                &UserId::new("user_1"),
                batch_id(),
                Some(binder_id),
                Some(sealed_product_id),
                vec![line("012", false, 2), line("13", true, 1)],
            )
            .await
            .unwrap();

        assert_eq!(
            outcome,
            ScanBatchOutcome {
                copies_added: 3,
                already_validated: false
            }
        );
    }

    #[tokio::test]
    async fn a_batch_without_a_sealed_product_has_no_purchase_to_record() {
        // The repository takes no price and no Purchase: without a sealed product it is handed
        // none, so the batch records no Purchase.
        let mut repository = repository();
        repository
            .expect_add_scan_batch()
            .withf(|_, _, binder, sealed_product, _| binder.is_none() && sealed_product.is_none())
            .times(1)
            .returning(|_, _, _, _, _| Box::pin(async { Ok(written(1)) }));
        let service = service(creating_card(1), repository, refresh(1));

        let result = service
            .validate_scan_batch(
                &UserId::new("user_1"),
                batch_id(),
                None,
                None,
                vec![line("12", false, 1)],
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn a_batch_already_recorded_answers_the_copies_of_its_first_send_and_checks_nothing_else()
    {
        let mut repository = MockCollectionEntryRepository::new();
        repository.expect_find_scan_batch().returning(|_, _| {
            Box::pin(async {
                Ok(Some(ScanBatchOutcome {
                    copies_added: 4,
                    already_validated: true,
                }))
            })
        });
        repository.expect_ensure_scan_batch_destination().never();
        repository.expect_add_scan_batch().never();
        // Nothing is created and no refresh is requested, whatever this send holds.
        let service = service(creating_card(0), repository, refresh(0));

        let outcome = service
            .validate_scan_batch(
                &UserId::new("user_1"),
                batch_id(),
                Some(BinderId(Uuid::from_u128(5))),
                None,
                vec![line("12", false, 999)],
            )
            .await
            .unwrap();

        assert_eq!(
            outcome,
            ScanBatchOutcome {
                copies_added: 4,
                already_validated: true
            }
        );
    }

    #[tokio::test]
    async fn a_batch_recorded_by_a_concurrent_send_answers_the_same_and_requests_no_refresh() {
        let mut repository = repository();
        repository
            .expect_add_scan_batch()
            .returning(|_, _, _, _, _| {
                Box::pin(async {
                    Ok(ScanBatchOutcome {
                        copies_added: 4,
                        already_validated: true,
                    })
                })
            });
        let service = service(creating_card(1), repository, refresh(0));

        let outcome = service
            .validate_scan_batch(
                &UserId::new("user_1"),
                batch_id(),
                None,
                None,
                vec![line("12", false, 9)],
            )
            .await
            .unwrap();

        assert_eq!(
            outcome,
            ScanBatchOutcome {
                copies_added: 4,
                already_validated: true
            }
        );
    }

    #[tokio::test]
    async fn a_destination_the_player_does_not_have_is_refused_before_any_card_is_created() {
        for error in [
            FunctionalError::BinderNotFound,
            FunctionalError::PurchaseNotFound,
        ] {
            let code = error.code();
            let mut repository = MockCollectionEntryRepository::new();
            repository
                .expect_find_scan_batch()
                .returning(|_, _| Box::pin(async { Ok(None) }));
            repository
                .expect_ensure_scan_batch_destination()
                .returning(move |_, _, _| {
                    let error = error.clone();
                    Box::pin(async move { Err(error.into()) })
                });
            repository.expect_add_scan_batch().never();
            // `creating_card(0)`: a single card created fails the test.
            let service = service(creating_card(0), repository, refresh(0));

            let refused = service
                .validate_scan_batch(
                    &UserId::new("user_1"),
                    batch_id(),
                    Some(BinderId(Uuid::from_u128(5))),
                    Some(PurchaseId(Uuid::from_u128(7))),
                    vec![line("12", false, 1)],
                )
                .await
                .unwrap_err();

            assert_eq!(refused.code(), code);
        }
    }

    #[tokio::test]
    async fn refuses_an_empty_batch() {
        let mut repository = repository();
        repository.expect_add_scan_batch().never();
        let service = service(creating_card(0), repository, refresh(0));

        let error = service
            .validate_scan_batch(&UserId::new("user_1"), batch_id(), None, None, vec![])
            .await
            .unwrap_err();

        assert_eq!(error.code(), "invalid_request");
    }

    #[tokio::test]
    async fn refuses_a_quantity_out_of_bounds_without_creating_any_card() {
        for quantity in [-1, 0, 256] {
            let mut repository = repository();
            repository.expect_add_scan_batch().never();
            let service = service(creating_card(0), repository, refresh(0));

            let error = service
                .validate_scan_batch(
                    &UserId::new("user_1"),
                    batch_id(),
                    None,
                    None,
                    vec![line("12", false, 1), line("13", false, quantity)],
                )
                .await
                .unwrap_err();

            assert_eq!(error.code(), "invalid_quantity");
        }
    }

    #[tokio::test]
    async fn names_every_card_the_catalog_does_not_have_and_writes_nothing() {
        let mut create_card = MockCreateCardUseCase::new();
        create_card.expect_create_card().returning(|card_id| {
            Box::pin(async move {
                if card_id.collector_number == "12" {
                    Ok(card_id)
                } else {
                    Err(FunctionalError::CardNotInCatalog.into())
                }
            })
        });
        let mut repository = repository();
        repository.expect_add_scan_batch().never();
        let service = service(create_card, repository, refresh(0));

        let error = service
            .validate_scan_batch(
                &UserId::new("user_1"),
                batch_id(),
                None,
                None,
                vec![
                    line("12", false, 1),
                    line("998", false, 1),
                    line("999", true, 1),
                ],
            )
            .await
            .unwrap_err();

        assert_eq!(error.code(), "card_not_in_catalog");
        assert!(matches!(
            error,
            AppError::Functional(FunctionalError::CardsNotInCatalog(cards))
                if cards == [
                    CardId::new("MKM", "998", LanguageCode::FR),
                    CardId::new("MKM", "999", LanguageCode::FR),
                ]
        ));
    }

    #[tokio::test]
    async fn a_refused_batch_keeps_its_error_and_requests_no_refresh() {
        for error in [
            FunctionalError::BinderNotFound,
            FunctionalError::PurchaseNotFound,
        ] {
            let code = error.code();
            let mut repository = repository();
            repository
                .expect_add_scan_batch()
                .returning(move |_, _, _, _, _| {
                    let error = error.clone();
                    Box::pin(async move { Err(error.into()) })
                });
            let service = service(creating_card(1), repository, refresh(0));

            let refused = service
                .validate_scan_batch(
                    &UserId::new("user_1"),
                    batch_id(),
                    None,
                    None,
                    vec![line("12", false, 1)],
                )
                .await
                .unwrap_err();

            assert_eq!(refused.code(), code);
        }
    }
}
