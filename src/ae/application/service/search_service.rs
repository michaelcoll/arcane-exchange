use crate::application::error::AppError;
use crate::application::repository::CardPricesViewRepository;
use crate::application::use_case::SearchCardsUseCase;
use crate::domain::card::Card;
use crate::domain::collection::SearchQuery;
use crate::domain::pagination::{PageRequest, Paginated};
use async_trait::async_trait;
use std::sync::Arc;

/// Search results can span the whole card database, so this endpoint must stay pageable as
/// deep as the collection endpoint.
pub(crate) const SEARCH_MAX_OFFSET: u32 = 10_000;

pub struct SearchService {
    repository: Arc<dyn CardPricesViewRepository>,
}

impl SearchService {
    pub fn new(repository: Arc<dyn CardPricesViewRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl SearchCardsUseCase for SearchService {
    async fn search_cards(
        &self,
        query: SearchQuery<PageRequest>,
    ) -> Result<Paginated<Card>, AppError> {
        let query = query.paginate(SEARCH_MAX_OFFSET)?;
        self.repository.search_paginated(query).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::error::InfraError;
    use crate::application::repository::MockCardPricesViewRepository;
    use crate::domain::collection::{CollectionQuery, CollectionSortField, SortDirection};
    use crate::domain::error::FunctionalError;

    fn query_for(page: u32, page_size: u32) -> SearchQuery<PageRequest> {
        CollectionQuery {
            pagination: PageRequest { page, page_size },
            ..Default::default()
        }
        .into()
    }

    /// A repository that echoes the validated pagination it receives back as an empty page.
    fn echoing_repository() -> MockCardPricesViewRepository {
        let mut mock_repo = MockCardPricesViewRepository::new();
        mock_repo.expect_search_paginated().returning(|q| {
            Box::pin(async move {
                Ok(Paginated {
                    items: vec![],
                    total: 0,
                    pagination: q.collection_query.pagination,
                })
            })
        });
        mock_repo
    }

    #[tokio::test]
    async fn search_cards_accepts_an_offset_at_the_search_limit() {
        let service = SearchService::new(Arc::new(echoing_repository()));

        // 100 * 100 = 10 000, exactly SEARCH_MAX_OFFSET.
        let result = service.search_cards(query_for(100, 100)).await.unwrap();

        assert_eq!(result.pagination.offset(), SEARCH_MAX_OFFSET);
    }

    #[tokio::test]
    async fn search_cards_rejects_an_offset_beyond_the_search_limit_without_reaching_the_repository()
     {
        // No expectation set: mockall panics if the repository is called.
        let service = SearchService::new(Arc::new(MockCardPricesViewRepository::new()));

        let result = service.search_cards(query_for(101, 100)).await;

        match result {
            Err(AppError::Functional(FunctionalError::PaginationTooDeep {
                requested_offset: 10_100,
                max: SEARCH_MAX_OFFSET,
            })) => {}
            other => panic!("Expected PaginationTooDeep, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn search_cards_rejects_a_page_size_above_the_max() {
        let service = SearchService::new(Arc::new(MockCardPricesViewRepository::new()));

        let result = service.search_cards(query_for(0, 101)).await;

        match result {
            Err(AppError::Functional(FunctionalError::InvalidPageSize {
                requested: 101, ..
            })) => {}
            other => panic!("Expected InvalidPageSize, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn search_cards_passes_filters_and_player_username_to_the_repository() {
        let mut mock_repo = MockCardPricesViewRepository::new();
        mock_repo
            .expect_search_paginated()
            .withf(|q| {
                q.collection_query.pagination.page() == 1
                    && q.collection_query.pagination.page_size() == 10
                    && q.collection_query.sort_by == CollectionSortField::SetCode
                    && q.collection_query.sort_dir == SortDirection::Asc
                    && q.player_username == Some("alice".to_string())
            })
            .returning(|q| {
                Box::pin(async move {
                    Ok(Paginated {
                        items: vec![],
                        total: 0,
                        pagination: q.collection_query.pagination,
                    })
                })
            });

        let service = SearchService::new(Arc::new(mock_repo));
        let query = SearchQuery {
            collection_query: CollectionQuery {
                pagination: PageRequest {
                    page: 1,
                    page_size: 10,
                },
                sort_by: CollectionSortField::SetCode,
                sort_dir: SortDirection::Asc,
                ..Default::default()
            },
            player_username: Some("alice".to_string()),
        };

        let result = service.search_cards(query).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn search_cards_propagates_repository_error() {
        let mut mock_repo = MockCardPricesViewRepository::new();
        mock_repo.expect_search_paginated().returning(|_| {
            Box::pin(async {
                Err(AppError::Infra(InfraError::RepositoryError(
                    "db error".to_string(),
                )))
            })
        });

        let service = SearchService::new(Arc::new(mock_repo));
        let result = service.search_cards(SearchQuery::default()).await;
        assert!(result.is_err());
    }
}
