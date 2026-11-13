use crate::application::error::AppError;
use crate::application::repository::PurchaseRepository;
use crate::application::use_case::UpdatePurchaseUseCase;
use crate::domain::purchase::{
    Purchase, PurchaseAmount, PurchaseChanges, PurchaseEditRequest, PurchaseId,
    normalize_sealed_product_name,
};
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct UpdatePurchaseService {
    repository: Arc<dyn PurchaseRepository>,
}

impl UpdatePurchaseService {
    pub fn new(repository: Arc<dyn PurchaseRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl UpdatePurchaseUseCase for UpdatePurchaseService {
    async fn update_purchase(
        &self,
        user_id: UserId,
        id: PurchaseId,
        edit: PurchaseEditRequest,
    ) -> Result<Purchase, AppError> {
        let changes = PurchaseChanges {
            amount: edit.amount_cents.map(PurchaseAmount::try_new).transpose()?,
            purchased_on: edit.purchased_on,
            product_type: edit.product_type,
            set_code: edit.set_code,
            name: edit
                .name
                .map(|name| normalize_sealed_product_name(Some(&name))),
        };

        self.repository.update(&user_id, id, changes).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::MockPurchaseRepository;
    use crate::application::service::purchase::test_fixtures::{booster, day, purchase_id};
    use crate::domain::error::FunctionalError;

    #[tokio::test]
    async fn update_purchase_replaces_only_what_is_given() {
        let mut mock_repository = MockPurchaseRepository::new();
        mock_repository
            .expect_update()
            .withf(|user, id, changes| {
                user.as_str() == "user_1"
                    && *id == purchase_id()
                    && *changes
                        == PurchaseChanges {
                            amount: Some(PurchaseAmount::try_new(650).unwrap()),
                            purchased_on: Some(day()),
                            ..Default::default()
                        }
            })
            .times(1)
            .returning(|_, _, _| Box::pin(async { Ok(booster(None)) }));

        let service = UpdatePurchaseService::new(Arc::new(mock_repository));
        service
            .update_purchase(
                UserId::new("user_1"),
                purchase_id(),
                PurchaseEditRequest {
                    amount_cents: Some(650),
                    purchased_on: Some(day()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn update_purchase_clears_the_name_when_it_is_empty_or_blank() {
        for name in ["", "   "] {
            let mut mock_repository = MockPurchaseRepository::new();
            mock_repository
                .expect_update()
                .withf(|_, _, changes| changes.name == Some(None))
                .times(1)
                .returning(|_, _, _| Box::pin(async { Ok(booster(None)) }));

            let service = UpdatePurchaseService::new(Arc::new(mock_repository));
            service
                .update_purchase(
                    UserId::new("user_1"),
                    purchase_id(),
                    PurchaseEditRequest {
                        name: Some(name.to_string()),
                        ..Default::default()
                    },
                )
                .await
                .unwrap();
        }
    }

    #[tokio::test]
    async fn update_purchase_sets_a_trimmed_name() {
        let mut mock_repository = MockPurchaseRepository::new();
        mock_repository
            .expect_update()
            .withf(|_, _, changes| changes.name == Some(Some("Deck Rouge".to_string())))
            .times(1)
            .returning(|_, _, _| Box::pin(async { Ok(booster(Some("Deck Rouge"))) }));

        let service = UpdatePurchaseService::new(Arc::new(mock_repository));
        service
            .update_purchase(
                UserId::new("user_1"),
                purchase_id(),
                PurchaseEditRequest {
                    name: Some(" Deck Rouge ".to_string()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn update_purchase_keeps_the_name_when_absent() {
        let mut mock_repository = MockPurchaseRepository::new();
        mock_repository
            .expect_update()
            .withf(|_, _, changes| changes.name.is_none())
            .times(1)
            .returning(|_, _, _| Box::pin(async { Ok(booster(None)) }));

        let service = UpdatePurchaseService::new(Arc::new(mock_repository));
        service
            .update_purchase(
                UserId::new("user_1"),
                purchase_id(),
                PurchaseEditRequest::default(),
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn update_purchase_refuses_a_non_positive_amount_without_touching_the_repository() {
        let service = UpdatePurchaseService::new(Arc::new(MockPurchaseRepository::new()));

        let result = service
            .update_purchase(
                UserId::new("user_1"),
                purchase_id(),
                PurchaseEditRequest {
                    amount_cents: Some(0),
                    ..Default::default()
                },
            )
            .await;

        match result.unwrap_err() {
            AppError::Functional(FunctionalError::InvalidPurchasePrice) => {}
            other => panic!("Expected InvalidPurchasePrice, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn update_purchase_propagates_purchase_not_found() {
        let mut mock_repository = MockPurchaseRepository::new();
        mock_repository
            .expect_update()
            .times(1)
            .returning(|_, _, _| Box::pin(async { Err(FunctionalError::PurchaseNotFound.into()) }));

        let service = UpdatePurchaseService::new(Arc::new(mock_repository));
        let result = service
            .update_purchase(
                UserId::new("user_1"),
                purchase_id(),
                PurchaseEditRequest::default(),
            )
            .await;

        match result.unwrap_err() {
            AppError::Functional(FunctionalError::PurchaseNotFound) => {}
            other => panic!("Expected PurchaseNotFound, got {other:?}"),
        }
    }
}
