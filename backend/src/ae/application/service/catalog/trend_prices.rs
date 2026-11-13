use crate::application::error::AppError;
use crate::application::repository::CatalogRepository;
use crate::application::use_case::TrendPricesUseCase;
use crate::domain::catalog::TrendPrices;
use crate::domain::error::FunctionalError;
use crate::domain::set_name::SetCode;
use async_trait::async_trait;
use std::sync::Arc;

/// A scanned batch is read in one call: 500 cards cover the largest one.
pub(crate) const TREND_PRICES_MAX_CARDS: usize = 500;

pub struct TrendPricesService {
    repository: Arc<dyn CatalogRepository>,
}

impl TrendPricesService {
    pub fn new(repository: Arc<dyn CatalogRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl TrendPricesUseCase for TrendPricesService {
    #[tracing::instrument(skip_all, fields(cards = cards.len()))]
    async fn get_trend_prices(
        &self,
        cards: Vec<(SetCode, String)>,
    ) -> Result<Vec<TrendPrices>, AppError> {
        if cards.len() > TREND_PRICES_MAX_CARDS {
            return Err(FunctionalError::InvalidRequest(format!(
                "at most {TREND_PRICES_MAX_CARDS} cards"
            ))
            .into());
        }
        let cards: Vec<(SetCode, String)> = cards
            .into_iter()
            .map(|(set_code, number)| (set_code, without_leading_zeros(&number)))
            .collect();

        self.repository.trend_prices(&cards).await
    }
}

/// The collector number as the catalog stores it: `012` is `12`, `000` is `0`.
fn without_leading_zeros(number: &str) -> String {
    let trimmed = number.trim_start_matches('0');
    if trimmed.is_empty() && !number.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::MockCatalogRepository;

    fn card(set_code: &str, number: &str) -> (SetCode, String) {
        (SetCode::new(set_code), number.to_string())
    }

    #[tokio::test]
    async fn reads_the_prices_by_the_numbers_without_leading_zeros_in_the_order_given() {
        let expected = vec![
            TrendPrices {
                trend_cents: Some(120),
                foil_trend_cents: None,
            },
            TrendPrices::default(),
        ];
        let returned = expected.clone();
        let mut repository = MockCatalogRepository::new();
        repository
            .expect_trend_prices()
            .withf(|cards| *cards == [card("MKM", "12"), card("MKM", "0"), card("MKM", "H13")])
            .times(1)
            .returning(move |_| {
                let prices = returned.clone();
                Box::pin(async move { Ok(prices) })
            });
        let service = TrendPricesService::new(Arc::new(repository));

        let prices = service
            .get_trend_prices(vec![
                card("MKM", "012"),
                card("MKM", "000"),
                card("MKM", "H13"),
            ])
            .await
            .unwrap();

        assert_eq!(prices, expected);
    }

    #[tokio::test]
    async fn refuses_more_cards_than_the_bound_without_reading() {
        let service = TrendPricesService::new(Arc::new(MockCatalogRepository::new()));
        let cards = vec![card("MKM", "1"); TREND_PRICES_MAX_CARDS + 1];

        let error = service.get_trend_prices(cards).await.unwrap_err();

        assert_eq!(error.code(), "invalid_request");
    }

    #[tokio::test]
    async fn accepts_exactly_the_bound() {
        let mut repository = MockCatalogRepository::new();
        repository
            .expect_trend_prices()
            .times(1)
            .returning(|_| Box::pin(async { Ok(vec![]) }));
        let service = TrendPricesService::new(Arc::new(repository));
        let cards = vec![card("MKM", "1"); TREND_PRICES_MAX_CARDS];

        service.get_trend_prices(cards).await.unwrap();
    }
}
