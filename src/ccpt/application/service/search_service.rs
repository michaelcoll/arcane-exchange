use crate::application::error::AppError;
use crate::application::repository::CardPricesViewRepository;
use crate::application::use_case::SearchCardsUseCase;
use crate::domain::collection::{CollectionQuery, PaginatedCollection};
use async_trait::async_trait;
use std::sync::Arc;

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
    async fn search_cards(&self, query: CollectionQuery) -> Result<PaginatedCollection, AppError> {
        self.repository.search_paginated(query).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::error::InfraError;
    use crate::application::repository::MockCardPricesViewRepository;
    use crate::domain::collection::{CollectionSortField, SortDirection};

    #[tokio::test]
    async fn search_cards_delegates_to_repository_with_correct_args() {
        let mut mock_repo = MockCardPricesViewRepository::new();
        let expected_query = CollectionQuery {
            page: 1,
            page_size: 10,
            sort_by: CollectionSortField::SetCode,
            sort_dir: SortDirection::Asc,
            search_query: None,
            rarity: Vec::new(),
            sets: Vec::new(),
            price_min: None,
            price_max: None,
        };
        let expected_result = PaginatedCollection {
            items: vec![],
            total: 0,
            page: 1,
            page_size: 10,
        };
        let result_clone = expected_result.clone();

        mock_repo
            .expect_search_paginated()
            .withf(|q| {
                q.page == 1
                    && q.page_size == 10
                    && q.sort_by == CollectionSortField::SetCode
                    && q.sort_dir == SortDirection::Asc
            })
            .returning(move |_| {
                let r = result_clone.clone();
                Box::pin(async move { Ok(r) })
            });

        let service = SearchService::new(Arc::new(mock_repo));
        let result = service.search_cards(expected_query).await;
        assert!(result.is_ok());
        let paginated = result.unwrap();
        assert_eq!(paginated.page, 1);
        assert_eq!(paginated.page_size, 10);
        assert_eq!(paginated.total, 0);
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
        let result = service.search_cards(CollectionQuery::default()).await;
        assert!(result.is_err());
    }
}
