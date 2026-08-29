use crate::application::date_range::resolve_date_range;
use crate::application::error::AppError;
use crate::application::repository::{CardMarketPriceRepository, CardRepository};
use crate::application::use_case::GetCardPriceHistoryUseCase;
use crate::domain::error::FunctionalError;
use crate::domain::price::PriceHistoryEntry;
use async_trait::async_trait;
use chrono::NaiveDate;
use std::sync::Arc;

pub struct CardPriceHistoryService {
    card_repository: Arc<dyn CardRepository>,
    cardmarket_price_repository: Arc<dyn CardMarketPriceRepository>,
}

impl CardPriceHistoryService {
    pub fn new(
        card_repository: Arc<dyn CardRepository>,
        cardmarket_price_repository: Arc<dyn CardMarketPriceRepository>,
    ) -> Self {
        Self {
            card_repository,
            cardmarket_price_repository,
        }
    }
}

#[async_trait]
impl GetCardPriceHistoryUseCase for CardPriceHistoryService {
    async fn get_card_price_history(
        &self,
        scryfall_id: uuid::Uuid,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Result<Vec<PriceHistoryEntry>, AppError> {
        let (start_date, end_date) = resolve_date_range(start_date, end_date)?;

        let Some((cardmarket_id, foil)) = self
            .card_repository
            .find_by_scryfall_id(scryfall_id)
            .await?
        else {
            return Err(FunctionalError::CardNotFound.into());
        };

        let Some(cardmarket_id) = cardmarket_id else {
            return Ok(vec![]);
        };

        self.cardmarket_price_repository
            .find_by_id_and_date_range(cardmarket_id, foil, start_date, end_date)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::error::InfraError;
    use crate::application::repository::{MockCardMarketPriceRepository, MockCardRepository};
    use crate::domain::price::{Price, PriceGuide};
    use chrono::NaiveDate;
    use uuid::Uuid;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[tokio::test]
    async fn returns_history_from_repository_when_card_found() {
        let scryfall_id = Uuid::new_v4();

        let mut mock_card_repo = MockCardRepository::new();
        mock_card_repo
            .expect_find_by_scryfall_id()
            .withf(move |id| *id == scryfall_id)
            .returning(|_| Box::pin(async { Ok(Some((Some(42), false))) }));

        let mut mock_price_repo = MockCardMarketPriceRepository::new();
        mock_price_repo
            .expect_find_by_id_and_date_range()
            .withf(|id_product, foil, start, end| {
                *id_product == 42
                    && !*foil
                    && *start == date(2025, 1, 1)
                    && *end == date(2025, 1, 31)
            })
            .returning(|_, _, _, _| {
                Box::pin(async {
                    Ok(vec![PriceHistoryEntry {
                        date: date(2025, 1, 15),
                        price_guide: PriceGuide {
                            low: Price { value: Some(100) },
                            trend: Price { value: Some(150) },
                            avg: Price { value: Some(130) },
                        },
                    }])
                })
            });

        let service =
            CardPriceHistoryService::new(Arc::new(mock_card_repo), Arc::new(mock_price_repo));

        let result = service
            .get_card_price_history(scryfall_id, Some(date(2025, 1, 1)), Some(date(2025, 1, 31)))
            .await;

        assert!(result.is_ok());
        let entries = result.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].price_guide.low.value, Some(100));
    }

    #[tokio::test]
    async fn returns_card_not_found_when_scryfall_id_unknown() {
        let mut mock_card_repo = MockCardRepository::new();
        mock_card_repo
            .expect_find_by_scryfall_id()
            .returning(|_| Box::pin(async { Ok(None) }));

        let mock_price_repo = MockCardMarketPriceRepository::new();

        let service =
            CardPriceHistoryService::new(Arc::new(mock_card_repo), Arc::new(mock_price_repo));

        let result = service
            .get_card_price_history(
                Uuid::new_v4(),
                Some(date(2025, 1, 1)),
                Some(date(2025, 1, 31)),
            )
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Functional(FunctionalError::CardNotFound) => {}
            other => panic!("Expected CardNotFound, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn returns_empty_vec_when_card_has_no_cardmarket_id() {
        let mut mock_card_repo = MockCardRepository::new();
        mock_card_repo
            .expect_find_by_scryfall_id()
            .returning(|_| Box::pin(async { Ok(Some((None, false))) }));

        // find_by_id_and_date_range must never be called: no expectation set on the mock,
        // mockall panics if it is called unexpectedly.
        let mock_price_repo = MockCardMarketPriceRepository::new();

        let service =
            CardPriceHistoryService::new(Arc::new(mock_card_repo), Arc::new(mock_price_repo));

        let result = service
            .get_card_price_history(
                Uuid::new_v4(),
                Some(date(2025, 1, 1)),
                Some(date(2025, 1, 31)),
            )
            .await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn returns_error_when_start_after_end() {
        let mock_card_repo = MockCardRepository::new();
        let mock_price_repo = MockCardMarketPriceRepository::new();

        let service =
            CardPriceHistoryService::new(Arc::new(mock_card_repo), Arc::new(mock_price_repo));

        let result = service
            .get_card_price_history(
                Uuid::new_v4(),
                Some(date(2025, 2, 1)),
                Some(date(2025, 1, 1)),
            )
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Functional(FunctionalError::WrongFormat(msg)) => {
                assert_eq!(msg, "start_date must be before or equal to end_date");
            }
            other => panic!("Expected WrongFormat, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn defaults_to_last_30_days_when_no_dates_provided() {
        use chrono::{Days, Utc};

        let today = Utc::now().date_naive();
        let expected_start = today - Days::new(30);

        let mut mock_card_repo = MockCardRepository::new();
        mock_card_repo
            .expect_find_by_scryfall_id()
            .returning(|_| Box::pin(async { Ok(Some((Some(1), false))) }));

        let mut mock_price_repo = MockCardMarketPriceRepository::new();
        mock_price_repo
            .expect_find_by_id_and_date_range()
            .withf(move |_, _, start, end| *start == expected_start && *end == today)
            .returning(|_, _, _, _| Box::pin(async { Ok(vec![]) }));

        let service =
            CardPriceHistoryService::new(Arc::new(mock_card_repo), Arc::new(mock_price_repo));

        let result = service
            .get_card_price_history(Uuid::new_v4(), None, None)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn propagates_repository_error() {
        let mut mock_card_repo = MockCardRepository::new();
        mock_card_repo
            .expect_find_by_scryfall_id()
            .returning(|_| Box::pin(async { Ok(Some((Some(1), false))) }));

        let mut mock_price_repo = MockCardMarketPriceRepository::new();
        mock_price_repo
            .expect_find_by_id_and_date_range()
            .returning(|_, _, _, _| {
                Box::pin(async {
                    Err(AppError::Infra(InfraError::RepositoryError(
                        "db error".to_string(),
                    )))
                })
            });

        let service =
            CardPriceHistoryService::new(Arc::new(mock_card_repo), Arc::new(mock_price_repo));

        let result = service
            .get_card_price_history(
                Uuid::new_v4(),
                Some(date(2025, 1, 1)),
                Some(date(2025, 1, 31)),
            )
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Infra(InfraError::RepositoryError(msg)) => assert_eq!(msg, "db error"),
            other => panic!("Expected RepositoryError, got {:?}", other),
        }
    }
}
