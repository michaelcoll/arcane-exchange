use crate::application::error::AppError;
use crate::application::repository::CatalogRepository;
use crate::application::use_case::SearchCatalogUseCase;
use crate::domain::catalog::CatalogSearchItem;
use crate::domain::pagination::{PageRequest, Paginated};
use crate::domain::set_name::SetCode;
use async_trait::async_trait;
use std::sync::Arc;

/// A player picks an impression among the first results of a name: they refine the query rather
/// than page through the whole catalog.
pub(crate) const CATALOG_SEARCH_MAX_OFFSET: u32 = 1_000;

pub struct SearchCatalogService {
    repository: Arc<dyn CatalogRepository>,
}

impl SearchCatalogService {
    pub fn new(repository: Arc<dyn CatalogRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl SearchCatalogUseCase for SearchCatalogService {
    #[tracing::instrument(skip(self))]
    async fn search_catalog(
        &self,
        query: Option<String>,
        set_code: Option<SetCode>,
        page: PageRequest,
    ) -> Result<Paginated<CatalogSearchItem>, AppError> {
        let pagination = page.paginate(CATALOG_SEARCH_MAX_OFFSET)?;
        let query = query.as_deref().map(str::trim).filter(|q| !q.is_empty());

        self.repository
            .search(query, set_code.as_ref(), pagination)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::MockCatalogRepository;
    use crate::domain::pagination::Pagination;

    fn empty_page(pagination: Pagination) -> Paginated<CatalogSearchItem> {
        Paginated {
            items: vec![],
            total: 0,
            pagination,
        }
    }

    #[tokio::test]
    async fn searches_with_the_trimmed_query_the_set_and_the_page() {
        let mut repository = MockCatalogRepository::new();
        repository
            .expect_search()
            .withf(|name, set_code, pagination| {
                *name == Some("lightning")
                    && set_code.is_some_and(|code| code.to_string() == "LEA")
                    && pagination.page() == 2
                    && pagination.limit() == 10
            })
            .times(1)
            .returning(|_, _, pagination| Box::pin(async move { Ok(empty_page(pagination)) }));
        let service = SearchCatalogService::new(Arc::new(repository));

        let result = service
            .search_catalog(
                Some("  lightning ".to_string()),
                Some(SetCode::new("LEA")),
                PageRequest {
                    page: 2,
                    page_size: 10,
                },
            )
            .await
            .unwrap();

        assert_eq!(result.pagination.page(), 2);
    }

    #[tokio::test]
    async fn a_blank_query_searches_without_a_name() {
        let mut repository = MockCatalogRepository::new();
        repository
            .expect_search()
            .withf(|name, _, _| name.is_none())
            .times(1)
            .returning(|_, _, pagination| Box::pin(async move { Ok(empty_page(pagination)) }));
        let service = SearchCatalogService::new(Arc::new(repository));

        service
            .search_catalog(Some("   ".to_string()), None, PageRequest::default())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn refuses_a_page_too_deep_without_searching() {
        let service = SearchCatalogService::new(Arc::new(MockCatalogRepository::new()));

        let error = service
            .search_catalog(
                None,
                None,
                PageRequest {
                    page: 51,
                    page_size: 20,
                },
            )
            .await
            .unwrap_err();

        assert_eq!(error.code(), "pagination_too_deep");
    }

    #[tokio::test]
    async fn refuses_a_page_size_out_of_bounds() {
        let service = SearchCatalogService::new(Arc::new(MockCatalogRepository::new()));

        let error = service
            .search_catalog(
                None,
                None,
                PageRequest {
                    page: 0,
                    page_size: 101,
                },
            )
            .await
            .unwrap_err();

        assert_eq!(error.code(), "invalid_page_size");
    }
}
