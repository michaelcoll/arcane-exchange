use crate::application::error::AppError;
use crate::application::repository::TradeRepository;
use crate::application::use_case::ListTradesUseCase;
use crate::domain::pagination::{PageRequest, Paginated};
use crate::domain::trade::{TradeListQuery, TradeSummary};
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

/// Trade history is bounded by how many trades a user can realistically accumulate, so it
/// needs less depth than the collection or search endpoints, but more than card offers.
pub(crate) const TRADES_MAX_OFFSET: u32 = 2_000;

pub struct ListTradesService {
    trade_repository: Arc<dyn TradeRepository>,
}

impl ListTradesService {
    pub fn new(trade_repository: Arc<dyn TradeRepository>) -> Self {
        Self { trade_repository }
    }
}

#[async_trait]
impl ListTradesUseCase for ListTradesService {
    async fn list_trades(
        &self,
        caller_id: UserId,
        query: TradeListQuery<PageRequest>,
    ) -> Result<Paginated<TradeSummary>, AppError> {
        let query = query.paginate(TRADES_MAX_OFFSET)?;
        self.trade_repository.list_trades(&caller_id, query).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::MockTradeRepository;
    use crate::application::service::trade::test_fixtures::make_initiator_id;
    use crate::domain::error::FunctionalError;
    use crate::domain::trade::TradeStatus;

    fn trade_list_query_for(page: u32, page_size: u32) -> TradeListQuery<PageRequest> {
        TradeListQuery {
            statuses: vec![TradeStatus::Pending],
            pagination: PageRequest { page, page_size },
        }
    }

    #[tokio::test]
    async fn list_trades_accepts_an_offset_at_the_trades_limit_and_passes_caller_and_statuses() {
        let mut mock_repository = MockTradeRepository::new();
        mock_repository
            .expect_list_trades()
            .times(1)
            .withf(|caller_id, query| {
                *caller_id == make_initiator_id() && query.statuses == vec![TradeStatus::Pending]
            })
            .returning(|_, query| {
                Box::pin(async move {
                    Ok(Paginated {
                        items: vec![],
                        total: 0,
                        pagination: query.pagination,
                    })
                })
            });

        let service = ListTradesService::new(Arc::new(mock_repository));
        // 20 * 100 = 2 000, exactly TRADES_MAX_OFFSET.
        let result = service
            .list_trades(make_initiator_id(), trade_list_query_for(20, 100))
            .await
            .unwrap();

        assert_eq!(result.pagination.offset(), TRADES_MAX_OFFSET);
    }

    #[tokio::test]
    async fn list_trades_rejects_an_offset_beyond_the_trades_limit_without_reaching_the_repository()
    {
        // No expectation set: mockall panics if the repository is called.
        let service = ListTradesService::new(Arc::new(MockTradeRepository::new()));

        let result = service
            .list_trades(make_initiator_id(), trade_list_query_for(21, 100))
            .await;

        match result {
            Err(AppError::Functional(FunctionalError::PaginationTooDeep {
                requested_offset: 2_100,
                max: TRADES_MAX_OFFSET,
            })) => {}
            other => panic!("Expected PaginationTooDeep, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn list_trades_rejects_a_page_size_above_the_max() {
        let service = ListTradesService::new(Arc::new(MockTradeRepository::new()));

        let result = service
            .list_trades(make_initiator_id(), trade_list_query_for(0, 101))
            .await;

        match result {
            Err(AppError::Functional(FunctionalError::InvalidPageSize {
                requested: 101, ..
            })) => {}
            other => panic!("Expected InvalidPageSize, got {:?}", other),
        }
    }
}
