use crate::application::error::AppError;
use crate::application::repository::CardPricesViewRepository;
use crate::application::use_case::GetCollectionUseCase;
use crate::domain::card::Card;
use crate::domain::collection::CollectionQuery;
use crate::domain::pagination::Paginated;
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

/// A user must be able to page through their entire collection, which can run into the
/// thousands of cards — much deeper than the other paginated endpoints.
pub(crate) const COLLECTION_MAX_OFFSET: u32 = 10_000;

pub struct CollectionService {
    repository: Arc<dyn CardPricesViewRepository>,
}

impl CollectionService {
    pub fn new(repository: Arc<dyn CardPricesViewRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl GetCollectionUseCase for CollectionService {
    async fn get_collection(
        &self,
        user_id: &UserId,
        query: CollectionQuery,
    ) -> Result<Paginated<Card>, AppError> {
        self.repository.get_paginated(user_id, query).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::error::InfraError;
    use crate::application::repository::MockCardPricesViewRepository;
    use crate::domain::collection::{CollectionSortField, SortDirection};
    use crate::domain::pagination::Pagination;

    #[tokio::test]
    async fn get_collection_delegates_to_repository_with_correct_args() {
        let mut mock_repo = MockCardPricesViewRepository::new();
        let expected_query = CollectionQuery {
            pagination: Pagination::try_new(1, 10, COLLECTION_MAX_OFFSET).unwrap(),
            sort_by: CollectionSortField::SetCode,
            sort_dir: SortDirection::Asc,
            search_query: None,
            rarity: Vec::new(),
            sets: Vec::new(),
            price_min: None,
            price_max: None,
        };
        let expected_result = Paginated {
            items: vec![],
            total: 0,
            pagination: Pagination::try_new(1, 10, COLLECTION_MAX_OFFSET).unwrap(),
        };
        let result_clone = expected_result.clone();

        mock_repo
            .expect_get_paginated()
            .withf(|uid, q| {
                uid == &UserId::new("user-1")
                    && q.pagination.page() == 1
                    && q.pagination.page_size() == 10
                    && q.sort_by == CollectionSortField::SetCode
                    && q.sort_dir == SortDirection::Asc
            })
            .returning(move |_, _| {
                let r = result_clone.clone();
                Box::pin(async move { Ok(r) })
            });

        let service = CollectionService::new(Arc::new(mock_repo));
        let result = service
            .get_collection(&UserId::new("user-1"), expected_query)
            .await;
        assert!(result.is_ok());
        let paginated = result.unwrap();
        assert_eq!(paginated.pagination.page(), 1);
        assert_eq!(paginated.pagination.page_size(), 10);
        assert_eq!(paginated.total, 0);
    }

    #[tokio::test]
    async fn get_collection_propagates_repository_error() {
        let mut mock_repo = MockCardPricesViewRepository::new();
        mock_repo.expect_get_paginated().returning(|_, _| {
            Box::pin(async {
                Err(AppError::Infra(InfraError::RepositoryError(
                    "db error".to_string(),
                )))
            })
        });

        let service = CollectionService::new(Arc::new(mock_repo));
        let result = service
            .get_collection(&UserId::new("user-1"), CollectionQuery::default())
            .await;
        assert!(result.is_err());
    }
}
