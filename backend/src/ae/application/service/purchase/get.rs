use crate::application::error::AppError;
use crate::application::repository::PurchaseRepository;
use crate::application::use_case::GetPurchaseUseCase;
use crate::domain::purchase::{PurchaseDetail, PurchaseId};
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct GetPurchaseService {
    repository: Arc<dyn PurchaseRepository>,
}

impl GetPurchaseService {
    pub fn new(repository: Arc<dyn PurchaseRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl GetPurchaseUseCase for GetPurchaseService {
    async fn get_purchase(
        &self,
        user_id: UserId,
        id: PurchaseId,
    ) -> Result<PurchaseDetail, AppError> {
        let purchase = self.repository.find(&user_id, id).await?;
        let content = self.repository.content(&user_id, id).await?;

        Ok(PurchaseDetail { purchase, content })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::MockPurchaseRepository;
    use crate::application::service::purchase::test_fixtures::{booster, purchase_id};
    use crate::domain::card::CardId;
    use crate::domain::error::FunctionalError;
    use crate::domain::language_code::LanguageCode;
    use crate::domain::purchase::PurchaseContentLine;

    fn line() -> PurchaseContentLine {
        PurchaseContentLine {
            card_id: CardId::new("FDN", "1", LanguageCode::EN),
            name: "Card".to_string(),
            image: None,
            foil: true,
            quantity: 2,
            trend_cents: Some(120),
        }
    }

    #[tokio::test]
    async fn get_purchase_returns_the_purchase_of_the_player_with_its_content() {
        let mut mock_repository = MockPurchaseRepository::new();
        mock_repository
            .expect_find()
            .withf(|user, id| user.as_str() == "user_1" && *id == purchase_id())
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(booster(None)) }));
        mock_repository
            .expect_content()
            .withf(|user, id| user.as_str() == "user_1" && *id == purchase_id())
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(vec![line()]) }));

        let service = GetPurchaseService::new(Arc::new(mock_repository));
        let detail = service
            .get_purchase(UserId::new("user_1"), purchase_id())
            .await
            .unwrap();

        assert_eq!(detail.purchase, booster(None));
        assert_eq!(detail.content, vec![line()]);
    }

    #[tokio::test]
    async fn get_purchase_propagates_purchase_not_found_and_reads_no_content() {
        let mut mock_repository = MockPurchaseRepository::new();
        mock_repository
            .expect_find()
            .times(1)
            .returning(|_, _| Box::pin(async { Err(FunctionalError::PurchaseNotFound.into()) }));
        mock_repository.expect_content().never();

        let service = GetPurchaseService::new(Arc::new(mock_repository));
        let result = service
            .get_purchase(UserId::new("user_1"), purchase_id())
            .await;

        match result.unwrap_err() {
            AppError::Functional(FunctionalError::PurchaseNotFound) => {}
            other => panic!("Expected PurchaseNotFound, got {other:?}"),
        }
    }
}
