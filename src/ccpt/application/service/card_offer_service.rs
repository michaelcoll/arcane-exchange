use crate::application::error::AppError;
use crate::application::repository::CardPricesViewRepository;
use crate::application::use_case::GetCardOffersUseCase;
use crate::domain::card::{CardId, CollectionEntry};
use crate::domain::card_offer::CardOfferSortField;
use crate::domain::error::FunctionalError;
use crate::domain::pagination::{Paginated, Pagination};
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

/// Only the first few offers for a given card are ever useful to a buyer, so pagination depth
/// stays shallow here — unlike the collection or search endpoints.
pub(crate) const CARD_OFFERS_MAX_OFFSET: u32 = 60;

pub struct CardOfferService {
    repository: Arc<dyn CardPricesViewRepository>,
}

impl CardOfferService {
    pub fn new(repository: Arc<dyn CardPricesViewRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl GetCardOffersUseCase for CardOfferService {
    async fn get_card_offers(
        &self,
        user_id: &UserId,
        card_id: CardId,
        sort_by: CardOfferSortField,
        pagination: Pagination,
    ) -> Result<Paginated<CollectionEntry>, AppError> {
        if !self.repository.exists(&card_id).await? {
            return Err(FunctionalError::CardNotFound.into());
        }

        self.repository
            .get_offers(user_id, &card_id, sort_by, pagination)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::error::InfraError;
    use crate::application::repository::MockCardPricesViewRepository;
    use crate::domain::language_code::LanguageCode;

    fn card_id() -> CardId {
        CardId::new("FDN", "1", LanguageCode::EN, false)
    }

    #[tokio::test]
    async fn returns_offers_when_card_exists() {
        let mut mock_repo = MockCardPricesViewRepository::new();
        mock_repo
            .expect_exists()
            .returning(|_| Box::pin(async { Ok(true) }));
        mock_repo
            .expect_get_offers()
            .returning(|_, _, _, pagination| {
                Box::pin(async move {
                    Ok(Paginated {
                        items: vec![],
                        total: 0,
                        pagination,
                    })
                })
            });

        let service = CardOfferService::new(Arc::new(mock_repo));
        let result = service
            .get_card_offers(
                &UserId::new("user-1"),
                card_id(),
                CardOfferSortField::SellingPrice,
                Pagination::try_new(0, 20, CARD_OFFERS_MAX_OFFSET).unwrap(),
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn returns_card_not_found_when_card_does_not_exist() {
        let mut mock_repo = MockCardPricesViewRepository::new();
        mock_repo
            .expect_exists()
            .returning(|_| Box::pin(async { Ok(false) }));
        // get_offers must never be called: no expectation set, mockall panics if it is.

        let service = CardOfferService::new(Arc::new(mock_repo));
        let result = service
            .get_card_offers(
                &UserId::new("user-1"),
                card_id(),
                CardOfferSortField::SellingPrice,
                Pagination::try_new(0, 20, CARD_OFFERS_MAX_OFFSET).unwrap(),
            )
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Functional(FunctionalError::CardNotFound) => {}
            other => panic!("Expected CardNotFound, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn propagates_exists_error() {
        let mut mock_repo = MockCardPricesViewRepository::new();
        mock_repo.expect_exists().returning(|_| {
            Box::pin(async {
                Err(AppError::Infra(InfraError::RepositoryError(
                    "db error".to_string(),
                )))
            })
        });

        let service = CardOfferService::new(Arc::new(mock_repo));
        let result = service
            .get_card_offers(
                &UserId::new("user-1"),
                card_id(),
                CardOfferSortField::SellingPrice,
                Pagination::try_new(0, 20, CARD_OFFERS_MAX_OFFSET).unwrap(),
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn propagates_get_offers_error() {
        let mut mock_repo = MockCardPricesViewRepository::new();
        mock_repo
            .expect_exists()
            .returning(|_| Box::pin(async { Ok(true) }));
        mock_repo.expect_get_offers().returning(|_, _, _, _| {
            Box::pin(async {
                Err(AppError::Infra(InfraError::RepositoryError(
                    "db error".to_string(),
                )))
            })
        });

        let service = CardOfferService::new(Arc::new(mock_repo));
        let result = service
            .get_card_offers(
                &UserId::new("user-1"),
                card_id(),
                CardOfferSortField::SellingPrice,
                Pagination::try_new(0, 20, CARD_OFFERS_MAX_OFFSET).unwrap(),
            )
            .await;

        assert!(result.is_err());
    }
}
