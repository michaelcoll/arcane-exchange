use crate::application::error::AppError;
use crate::application::repository::PurchaseRepository;
use crate::application::use_case::CreateSealedProductUseCase;
use crate::domain::purchase::{
    NewSealedProduct, Purchase, PurchaseAmount, SealedProductRequest, normalize_sealed_product_name,
};
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct CreateSealedProductService {
    repository: Arc<dyn PurchaseRepository>,
}

impl CreateSealedProductService {
    pub fn new(repository: Arc<dyn PurchaseRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl CreateSealedProductUseCase for CreateSealedProductService {
    async fn create_sealed_product(
        &self,
        user_id: UserId,
        request: SealedProductRequest,
    ) -> Result<Purchase, AppError> {
        let product = NewSealedProduct {
            product_type: request.product_type,
            set_code: request.set_code,
            name: normalize_sealed_product_name(request.name.as_deref()),
            amount: PurchaseAmount::try_new(request.amount_cents)?,
            purchased_on: request.purchased_on,
        };

        self.repository
            .create_sealed_product(&user_id, product)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::MockPurchaseRepository;
    use crate::application::service::purchase::test_fixtures::{booster, day};
    use crate::domain::error::FunctionalError;
    use crate::domain::purchase::SealedProductType;
    use crate::domain::set_name::SetCode;

    fn request(name: Option<&str>, amount_cents: i32) -> SealedProductRequest {
        SealedProductRequest {
            product_type: SealedProductType::Display,
            set_code: SetCode::new("FDN"),
            name: name.map(str::to_string),
            amount_cents,
            purchased_on: day(),
        }
    }

    #[tokio::test]
    async fn create_sealed_product_records_it_for_the_player() {
        let mut mock_repository = MockPurchaseRepository::new();
        mock_repository
            .expect_create_sealed_product()
            .withf(|user, product| {
                user.as_str() == "user_1"
                    && product.product_type == SealedProductType::Display
                    && product.amount.cents() == 12_000
                    && product.purchased_on == day()
            })
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(booster(None)) }));

        let service = CreateSealedProductService::new(Arc::new(mock_repository));
        let purchase = service
            .create_sealed_product(UserId::new("user_1"), request(None, 12_000))
            .await
            .unwrap();

        assert_eq!(purchase, booster(None));
    }

    #[tokio::test]
    async fn create_sealed_product_stores_a_blank_name_as_absent() {
        let mut mock_repository = MockPurchaseRepository::new();
        mock_repository
            .expect_create_sealed_product()
            .withf(|_, product| product.name.is_none())
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(booster(None)) }));

        let service = CreateSealedProductService::new(Arc::new(mock_repository));
        service
            .create_sealed_product(UserId::new("user_1"), request(Some("  \t"), 499))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn create_sealed_product_trims_the_name() {
        let mut mock_repository = MockPurchaseRepository::new();
        mock_repository
            .expect_create_sealed_product()
            .withf(|_, product| product.name.as_deref() == Some("Deck Bleu"))
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(booster(Some("Deck Bleu"))) }));

        let service = CreateSealedProductService::new(Arc::new(mock_repository));
        service
            .create_sealed_product(UserId::new("user_1"), request(Some(" Deck Bleu "), 499))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn create_sealed_product_refuses_a_non_positive_amount_without_touching_the_repository() {
        let service = CreateSealedProductService::new(Arc::new(MockPurchaseRepository::new()));

        for cents in [0, -499] {
            let result = service
                .create_sealed_product(UserId::new("user_1"), request(None, cents))
                .await;

            match result.unwrap_err() {
                AppError::Functional(FunctionalError::InvalidPurchasePrice) => {}
                other => panic!("Expected InvalidPurchasePrice, got {other:?}"),
            }
        }
    }

    #[tokio::test]
    async fn create_sealed_product_propagates_an_unknown_set() {
        let mut mock_repository = MockPurchaseRepository::new();
        mock_repository
            .expect_create_sealed_product()
            .times(1)
            .returning(|_, _| Box::pin(async { Err(FunctionalError::SetNotFound.into()) }));

        let service = CreateSealedProductService::new(Arc::new(mock_repository));
        let result = service
            .create_sealed_product(UserId::new("user_1"), request(None, 499))
            .await;

        match result.unwrap_err() {
            AppError::Functional(FunctionalError::SetNotFound) => {}
            other => panic!("Expected SetNotFound, got {other:?}"),
        }
    }
}
