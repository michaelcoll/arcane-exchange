use crate::application::error::AppError;
use crate::application::repository::SearchRepository;
use crate::application::use_case::SearchCardsUseCase;
use crate::domain::pagination::{PageRequest, Paginated};
use crate::domain::search::{SearchCard, SearchQuery};
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

/// Search results can span the whole card database, so this endpoint must stay pageable as
/// deep as the collection endpoint.
pub(crate) const SEARCH_MAX_OFFSET: u32 = 10_000;

pub struct SearchCardsService {
    repository: Arc<dyn SearchRepository>,
}

impl SearchCardsService {
    pub fn new(repository: Arc<dyn SearchRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl SearchCardsUseCase for SearchCardsService {
    async fn search_cards(
        &self,
        user_id: &UserId,
        query: SearchQuery<PageRequest>,
    ) -> Result<Paginated<SearchCard>, AppError> {
        let query = query.paginate(SEARCH_MAX_OFFSET)?;
        self.repository.search_paginated(user_id, query).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::error::InfraError;
    use crate::application::repository::MockSearchRepository;
    use crate::domain::collection::SortDirection;
    use crate::domain::error::FunctionalError;
    use crate::domain::search::{Offer, Possession};

    fn user() -> UserId {
        UserId::new("userA")
    }

    fn query_for(page: u32, page_size: u32) -> SearchQuery<PageRequest> {
        SearchQuery {
            pagination: PageRequest { page, page_size },
            ..Default::default()
        }
    }

    /// A repository that echoes the validated pagination it receives back as an empty page.
    fn echoing_repository() -> MockSearchRepository {
        let mut mock_repo = MockSearchRepository::new();
        mock_repo.expect_search_paginated().returning(|_, q| {
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
    async fn search_cards_accepts_an_offset_at_the_search_limit() {
        let service = SearchCardsService::new(Arc::new(echoing_repository()));

        // 100 * 100 = 10 000, exactly SEARCH_MAX_OFFSET.
        let result = service
            .search_cards(&user(), query_for(100, 100))
            .await
            .unwrap();

        assert_eq!(result.pagination.offset(), SEARCH_MAX_OFFSET);
    }

    #[tokio::test]
    async fn search_cards_rejects_an_offset_beyond_the_search_limit_without_reaching_the_repository()
     {
        // No expectation set: mockall panics if the repository is called.
        let service = SearchCardsService::new(Arc::new(MockSearchRepository::new()));

        let result = service.search_cards(&user(), query_for(101, 100)).await;

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
        let service = SearchCardsService::new(Arc::new(MockSearchRepository::new()));

        let result = service.search_cards(&user(), query_for(0, 101)).await;

        match result {
            Err(AppError::Functional(FunctionalError::InvalidPageSize {
                requested: 101, ..
            })) => {}
            other => panic!("Expected InvalidPageSize, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn search_cards_passes_the_filters_and_the_offer_to_the_repository() {
        let mut mock_repo = MockSearchRepository::new();
        mock_repo
            .expect_search_paginated()
            .withf(|user_id, q| {
                *user_id == UserId::new("userA")
                    && q.pagination.page() == 1
                    && q.pagination.page_size() == 10
                    && q.sort_dir == SortDirection::Asc
                    && q.possession == Possession::NotOwned
                    && q.offer == Offer::OfferedBy("alice".to_string())
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

        let service = SearchCardsService::new(Arc::new(mock_repo));
        let query = SearchQuery {
            pagination: PageRequest {
                page: 1,
                page_size: 10,
            },
            sort_dir: SortDirection::Asc,
            possession: Possession::NotOwned,
            offer: Offer::OfferedBy("alice".to_string()),
            ..Default::default()
        };

        let result = service.search_cards(&user(), query).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn search_cards_propagates_repository_error() {
        let mut mock_repo = MockSearchRepository::new();
        mock_repo.expect_search_paginated().returning(|_, _| {
            Box::pin(async {
                Err(AppError::Infra(InfraError::RepositoryError(
                    "db error".to_string(),
                )))
            })
        });

        let service = SearchCardsService::new(Arc::new(mock_repo));
        let result = service.search_cards(&user(), SearchQuery::default()).await;
        assert!(result.is_err());
    }
}
