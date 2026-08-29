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
