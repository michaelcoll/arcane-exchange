use crate::application::error::AppError;
use crate::application::repository::PurchaseRepository;
use crate::application::use_case::ListPurchasesUseCase;
use crate::domain::pagination::{PageRequest, Paginated};
use crate::domain::purchase::Purchase;
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

/// A player records a few purchases a week at most: far less depth than the collection needs.
pub(crate) const PURCHASES_MAX_OFFSET: u32 = 2_000;

pub struct ListPurchasesService {
    repository: Arc<dyn PurchaseRepository>,
}

impl ListPurchasesService {
    pub fn new(repository: Arc<dyn PurchaseRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl ListPurchasesUseCase for ListPurchasesService {
    async fn list_purchases(
        &self,
        user_id: UserId,
        page: PageRequest,
    ) -> Result<Paginated<Purchase>, AppError> {
        let pagination = page.paginate(PURCHASES_MAX_OFFSET)?;

        self.repository.list(&user_id, pagination).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::MockPurchaseRepository;
    use crate::application::service::purchase::test_fixtures::booster;
    use crate::domain::error::FunctionalError;

    #[tokio::test]
    async fn list_purchases_returns_the_page_of_the_player() {
        let mut mock_repository = MockPurchaseRepository::new();
        mock_repository
            .expect_list()
            .withf(|user, pagination| {
                user.as_str() == "user_1" && pagination.page() == 1 && pagination.limit() == 10
            })
            .times(1)
            .returning(|_, pagination| {
                Box::pin(async move {
                    Ok(Paginated {
                        items: vec![booster(None)],
                        total: 11,
                        pagination,
                    })
                })
            });

        let service = ListPurchasesService::new(Arc::new(mock_repository));
        let page = service
            .list_purchases(
                UserId::new("user_1"),
                PageRequest {
                    page: 1,
                    page_size: 10,
                },
            )
            .await
            .unwrap();

        assert_eq!(page.items, vec![booster(None)]);
        assert_eq!(page.total, 11);
    }

    #[tokio::test]
    async fn list_purchases_refuses_a_page_beyond_the_limit_without_touching_the_repository() {
        let service = ListPurchasesService::new(Arc::new(MockPurchaseRepository::new()));

        // 21 * 100 = 2 100, over PURCHASES_MAX_OFFSET.
        let result = service
            .list_purchases(
                UserId::new("user_1"),
                PageRequest {
                    page: 21,
                    page_size: 100,
                },
            )
            .await;

        match result.unwrap_err() {
            AppError::Functional(FunctionalError::PaginationTooDeep { .. }) => {}
            other => panic!("Expected PaginationTooDeep, got {other:?}"),
        }
    }
}
