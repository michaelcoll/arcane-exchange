use crate::application::error::AppError;
use crate::application::repository::PurchaseRepository;
use crate::application::use_case::UpdatePurchaseUseCase;
use crate::domain::error::FunctionalError;
use crate::domain::purchase::{
    Purchase, PurchaseAmount, PurchaseChanges, PurchaseEditRequest, PurchaseId, PurchaseKind,
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
        let sealed_product_only =
            edit.product_type.is_some() || edit.set_code.is_some() || edit.name.is_some();
        if sealed_product_only {
            let purchase = self.repository.find(&user_id, id).await?;
            if matches!(purchase.kind, PurchaseKind::SingleCard { .. }) {
                return Err(FunctionalError::InvalidRequest(
                    "a single card purchase has no type, set or name".to_string(),
                )
                .into());
            }
        }

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
    use crate::application::service::purchase::test_fixtures::{
        booster, day, purchase_id, single_card,
    };
    use crate::domain::purchase::SealedProductType;
    use crate::domain::set_name::SetCode;

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
                .expect_find()
                .returning(|_, _| Box::pin(async { Ok(booster(None)) }));
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
            .expect_find()
            .returning(|_, _| Box::pin(async { Ok(booster(None)) }));
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
    async fn update_purchase_refuses_a_type_a_set_or_a_name_on_a_single_card() {
        let edits = [
            PurchaseEditRequest {
                product_type: Some(SealedProductType::Display),
                ..Default::default()
            },
            PurchaseEditRequest {
                set_code: Some(SetCode::new("DSK")),
                ..Default::default()
            },
            PurchaseEditRequest {
                name: Some(String::new()),
                ..Default::default()
            },
        ];
        for edit in edits {
            let mut mock_repository = MockPurchaseRepository::new();
            mock_repository
                .expect_find()
                .returning(|_, _| Box::pin(async { Ok(single_card()) }));
            mock_repository.expect_update().never();
            let service = UpdatePurchaseService::new(Arc::new(mock_repository));

            let error = service
                .update_purchase(UserId::new("user_1"), purchase_id(), edit)
                .await
                .unwrap_err();

            assert_eq!(error.code(), "invalid_request");
        }
    }

    #[tokio::test]
    async fn update_purchase_changes_the_amount_and_the_date_of_a_single_card() {
        let mut mock_repository = MockPurchaseRepository::new();
        mock_repository
            .expect_update()
            .withf(|_, _, changes| {
                changes.amount == Some(PurchaseAmount::try_new(650).unwrap())
                    && changes.purchased_on == Some(day())
            })
            .times(1)
            .returning(|_, _, _| Box::pin(async { Ok(single_card()) }));
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
