use crate::application::caller::CardMarketCaller;
use crate::application::error::AppError;
use crate::application::repository::{CardMarketPriceRepository, CardPricesViewRepository};
use crate::application::use_case::{CardCollectionPriceCalculationUseCase, ImportPriceUseCase};
use async_trait::async_trait;
use std::sync::Arc;

pub struct ImportPriceService {
    cardmarket_caller: Arc<dyn CardMarketCaller>,
    cardmarket_repository: Arc<dyn CardMarketPriceRepository>,
    card_prices_view_repository: Arc<dyn CardPricesViewRepository>,
    price_calculation: Arc<dyn CardCollectionPriceCalculationUseCase>,
}

impl ImportPriceService {
    pub fn new(
        cardmarket_caller: Arc<dyn CardMarketCaller>,
        cardmarket_repository: Arc<dyn CardMarketPriceRepository>,
        card_prices_view_repository: Arc<dyn CardPricesViewRepository>,
        price_calculation: Arc<dyn CardCollectionPriceCalculationUseCase>,
    ) -> Self {
        Self {
            cardmarket_caller,
            cardmarket_repository,
            card_prices_view_repository,
            price_calculation,
        }
    }
}

#[async_trait]
impl ImportPriceUseCase for ImportPriceService {
    #[tracing::instrument(skip(self))]
    async fn import_prices_for_current_date(&self) -> Result<(), AppError> {
        let (date, price_guides) = self.cardmarket_caller.get_price_guides().await?;
        let price_count = price_guides.len();
        self.cardmarket_repository.save(date, price_guides).await?;
        self.card_prices_view_repository.refresh().await?;
        self.price_calculation.calculate_total_price().await?;

        tracing::info!(price_count, %date, "Prices imported successfully");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::caller::MockCardMarketCaller;
    use crate::application::error::InfraError;
    use crate::application::repository::{
        MockCardMarketPriceRepository, MockCardPricesViewRepository,
    };
    use crate::application::use_case::MockCardCollectionPriceCalculationUseCase;
    use crate::domain::price::{FullPriceGuide, PriceGuide};
    use chrono::NaiveDate;
    use mockall::Sequence;

    fn price_date() -> NaiveDate {
        NaiveDate::from_ymd_opt(2025, 12, 25).unwrap()
    }

    fn price_guide() -> FullPriceGuide {
        FullPriceGuide {
            id_product: 42,
            normal: PriceGuide::new(100, 150, 130),
            foil: PriceGuide::new(200, 250, 230),
        }
    }

    fn failure(step: &str) -> AppError {
        AppError::Infra(InfraError::CallError(format!("{step} failed")))
    }

    fn caller_returning_one_price_guide() -> MockCardMarketCaller {
        let mut caller = MockCardMarketCaller::new();
        caller
            .expect_get_price_guides()
            .times(1)
            .returning(|| Box::pin(async { Ok((price_date(), vec![price_guide()])) }));
        caller
    }

    fn service(
        caller: MockCardMarketCaller,
        price_repository: MockCardMarketPriceRepository,
        view_repository: MockCardPricesViewRepository,
        price_calculation: MockCardCollectionPriceCalculationUseCase,
    ) -> ImportPriceService {
        ImportPriceService::new(
            Arc::new(caller),
            Arc::new(price_repository),
            Arc::new(view_repository),
            Arc::new(price_calculation),
        )
    }

    #[tokio::test]
    async fn saves_the_price_guides_then_refreshes_the_views_then_recalculates_the_collection_value()
     {
        let mut sequence = Sequence::new();
        let mut price_repository = MockCardMarketPriceRepository::new();
        price_repository
            .expect_save()
            .withf(|date, price_guides| {
                *date == price_date() && price_guides.len() == 1 && price_guides[0].id_product == 42
            })
            .times(1)
            .in_sequence(&mut sequence)
            .returning(|_, _| Box::pin(async { Ok(()) }));
        let mut view_repository = MockCardPricesViewRepository::new();
        view_repository
            .expect_refresh()
            .times(1)
            .in_sequence(&mut sequence)
            .returning(|| Box::pin(async { Ok(()) }));
        let mut price_calculation = MockCardCollectionPriceCalculationUseCase::new();
        price_calculation
            .expect_calculate_total_price()
            .times(1)
            .in_sequence(&mut sequence)
            .returning(|| Box::pin(async { Ok(()) }));
        let service = service(
            caller_returning_one_price_guide(),
            price_repository,
            view_repository,
            price_calculation,
        );

        let result = service.import_prices_for_current_date().await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn stops_before_saving_when_the_price_guides_cannot_be_fetched() {
        let mut caller = MockCardMarketCaller::new();
        caller
            .expect_get_price_guides()
            .times(1)
            .returning(|| Box::pin(async { Err(failure("fetch")) }));
        let mut price_repository = MockCardMarketPriceRepository::new();
        price_repository.expect_save().times(0);
        let mut view_repository = MockCardPricesViewRepository::new();
        view_repository.expect_refresh().times(0);
        let mut price_calculation = MockCardCollectionPriceCalculationUseCase::new();
        price_calculation.expect_calculate_total_price().times(0);
        let service = service(caller, price_repository, view_repository, price_calculation);

        let result = service.import_prices_for_current_date().await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn stops_before_refreshing_the_views_when_the_save_fails() {
        let mut price_repository = MockCardMarketPriceRepository::new();
        price_repository
            .expect_save()
            .times(1)
            .returning(|_, _| Box::pin(async { Err(failure("save")) }));
        let mut view_repository = MockCardPricesViewRepository::new();
        view_repository.expect_refresh().times(0);
        let mut price_calculation = MockCardCollectionPriceCalculationUseCase::new();
        price_calculation.expect_calculate_total_price().times(0);
        let service = service(
            caller_returning_one_price_guide(),
            price_repository,
            view_repository,
            price_calculation,
        );

        let result = service.import_prices_for_current_date().await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn stops_before_recalculating_the_collection_value_when_the_refresh_fails() {
        let mut price_repository = MockCardMarketPriceRepository::new();
        price_repository
            .expect_save()
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(()) }));
        let mut view_repository = MockCardPricesViewRepository::new();
        view_repository
            .expect_refresh()
            .times(1)
            .returning(|| Box::pin(async { Err(failure("refresh")) }));
        let mut price_calculation = MockCardCollectionPriceCalculationUseCase::new();
        price_calculation.expect_calculate_total_price().times(0);
        let service = service(
            caller_returning_one_price_guide(),
            price_repository,
            view_repository,
            price_calculation,
        );

        let result = service.import_prices_for_current_date().await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn fails_when_the_collection_value_cannot_be_recalculated() {
        let mut price_repository = MockCardMarketPriceRepository::new();
        price_repository
            .expect_save()
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(()) }));
        let mut view_repository = MockCardPricesViewRepository::new();
        view_repository
            .expect_refresh()
            .times(1)
            .returning(|| Box::pin(async { Ok(()) }));
        let mut price_calculation = MockCardCollectionPriceCalculationUseCase::new();
        price_calculation
            .expect_calculate_total_price()
            .times(1)
            .returning(|| Box::pin(async { Err(failure("calculation")) }));
        let service = service(
            caller_returning_one_price_guide(),
            price_repository,
            view_repository,
            price_calculation,
        );

        let result = service.import_prices_for_current_date().await;

        assert!(result.is_err());
    }
}
