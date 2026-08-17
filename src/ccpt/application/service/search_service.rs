use crate::application::error::AppError;
use crate::application::repository::CardPricesViewRepository;
use crate::application::use_case::SearchCardsUseCase;
use crate::domain::card::Card;
use crate::domain::collection::SearchQuery;
use crate::domain::pagination::Paginated;
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
    async fn search_cards(&self, query: SearchQuery) -> Result<Paginated<Card>, AppError> {
        self.repository.search_paginated(query).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::error::InfraError;
    use crate::application::repository::MockCardPricesViewRepository;
    use crate::domain::collection::{CollectionQuery, CollectionSortField, SortDirection};
    use crate::domain::pagination::Pagination;

    #[tokio::test]
    async fn search_cards_delegates_to_repository_with_correct_args() {
        let mut mock_repo = MockCardPricesViewRepository::new();
        let expected_query = SearchQuery {
            collection_query: CollectionQuery {
                pagination: Pagination::try_new(1, 10, SEARCH_MAX_OFFSET).unwrap(),
                sort_by: CollectionSortField::SetCode,
                sort_dir: SortDirection::Asc,
                search_query: None,
                rarity: Vec::new(),
                sets: Vec::new(),
                price_min: None,
                price_max: None,
            },
            player_username: None,
        };
        let expected_result = Paginated {
            items: vec![],
            total: 0,
            pagination: Pagination::try_new(1, 10, SEARCH_MAX_OFFSET).unwrap(),
        };
        let result_clone = expected_result.clone();

        mock_repo
            .expect_search_paginated()
            .withf(|q| {
                q.collection_query.pagination.page() == 1
                    && q.collection_query.pagination.page_size() == 10
                    && q.collection_query.sort_by == CollectionSortField::SetCode
                    && q.collection_query.sort_dir == SortDirection::Asc
            })
            .returning(move |_| {
                let r = result_clone.clone();
                Box::pin(async move { Ok(r) })
            });

        let service = SearchService::new(Arc::new(mock_repo));
        let result = service.search_cards(expected_query).await;
        assert!(result.is_ok());
        let paginated = result.unwrap();
        assert_eq!(paginated.pagination.page(), 1);
        assert_eq!(paginated.pagination.page_size(), 10);
        assert_eq!(paginated.total, 0);
    }

    #[tokio::test]
    async fn search_cards_delegates_player_username_to_repository() {
        let mut mock_repo = MockCardPricesViewRepository::new();
        let expected_query = SearchQuery {
            collection_query: CollectionQuery::default(),
            player_username: Some("alice".to_string()),
        };
        let expected_result = Paginated {
            items: vec![],
            total: 0,
            pagination: Pagination::default(),
        };
        let result_clone = expected_result.clone();

        mock_repo
            .expect_search_paginated()
            .withf(|q| q.player_username == Some("alice".to_string()))
            .returning(move |_| {
                let r = result_clone.clone();
                Box::pin(async move { Ok(r) })
            });

        let service = SearchService::new(Arc::new(mock_repo));
        let result = service.search_cards(expected_query).await;
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
