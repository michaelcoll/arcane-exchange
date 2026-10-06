use crate::application::error::AppError;
use crate::application::repository::CardPricesViewRepository;
use crate::application::use_case::GetCollectionUseCase;
use crate::domain::card::Card;
use crate::domain::collection::CollectionQuery;
use crate::domain::pagination::{PageRequest, Paginated};
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
        query: CollectionQuery<PageRequest>,
    ) -> Result<Paginated<Card>, AppError> {
        let query = query.paginate(COLLECTION_MAX_OFFSET)?;
        self.repository.get_paginated(user_id, query).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::error::InfraError;
    use crate::application::repository::MockCardPricesViewRepository;
    use crate::domain::collection::{CollectionSortField, SortDirection};
    use crate::domain::error::FunctionalError;

    fn query_for(page: u32, page_size: u32) -> CollectionQuery<PageRequest> {
        CollectionQuery {
            pagination: PageRequest { page, page_size },
            ..Default::default()
        }
    }

    /// A repository that echoes the validated pagination it receives back as an empty page.
    fn echoing_repository() -> MockCardPricesViewRepository {
        let mut mock_repo = MockCardPricesViewRepository::new();
        mock_repo.expect_get_paginated().returning(|_, q| {
            Box::pin(async move {
                Ok(Paginated {
                    items: vec![],
                    total: 0,
                    pagination: q.pagination,
                })
            })
        });
        mock_repo
    }

    #[tokio::test]
    async fn get_collection_accepts_an_offset_at_the_collection_limit() {
        let service = CollectionService::new(Arc::new(echoing_repository()));

        // 100 * 100 = 10 000, exactly COLLECTION_MAX_OFFSET.
        let result = service
            .get_collection(&UserId::new("user-1"), query_for(100, 100))
            .await
            .unwrap();

        assert_eq!(result.pagination.offset(), COLLECTION_MAX_OFFSET);
    }

    #[tokio::test]
    async fn get_collection_rejects_an_offset_beyond_the_collection_limit_without_reaching_the_repository()
     {
        // No expectation set: mockall panics if the repository is called.
        let service = CollectionService::new(Arc::new(MockCardPricesViewRepository::new()));

        let result = service
            .get_collection(&UserId::new("user-1"), query_for(101, 100))
            .await;

        match result {
            Err(AppError::Functional(FunctionalError::PaginationTooDeep {
                requested_offset: 10_100,
                max: COLLECTION_MAX_OFFSET,
            })) => {}
            other => panic!("Expected PaginationTooDeep, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn get_collection_rejects_a_page_size_above_the_max() {
        let service = CollectionService::new(Arc::new(MockCardPricesViewRepository::new()));

        let result = service
            .get_collection(&UserId::new("user-1"), query_for(0, 101))
            .await;

        match result {
            Err(AppError::Functional(FunctionalError::InvalidPageSize {
                requested: 101, ..
            })) => {}
            other => panic!("Expected InvalidPageSize, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn get_collection_passes_the_caller_and_filters_to_the_repository() {
        let mut mock_repo = MockCardPricesViewRepository::new();
        mock_repo
            .expect_get_paginated()
            .withf(|uid, q| {
                uid == &UserId::new("user-1")
                    && q.pagination.page() == 1
                    && q.pagination.page_size() == 10
                    && q.sort_by == CollectionSortField::SetCode
                    && q.sort_dir == SortDirection::Asc
            })
            .returning(|_, q| {
                Box::pin(async move {
                    Ok(Paginated {
                        items: vec![],
                        total: 0,
                        pagination: q.pagination,
                    })
                })
            });

        let service = CollectionService::new(Arc::new(mock_repo));
        let result = service
            .get_collection(
                &UserId::new("user-1"),
                CollectionQuery {
                    sort_by: CollectionSortField::SetCode,
                    sort_dir: SortDirection::Asc,
                    ..query_for(1, 10)
                },
            )
            .await;

        assert!(result.is_ok());
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
