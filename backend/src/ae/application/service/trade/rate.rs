use crate::application::error::AppError;
use crate::application::repository::TradeRepository;
use crate::application::service::trade::decide_and_apply;
use crate::application::use_case::RateTradeUseCase;
use crate::domain::trade::TradeId;
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct RateTradeService {
    trade_repository: Arc<dyn TradeRepository>,
}

impl RateTradeService {
    pub fn new(trade_repository: Arc<dyn TradeRepository>) -> Self {
        Self { trade_repository }
    }
}

#[async_trait]
impl RateTradeUseCase for RateTradeService {
    async fn rate(&self, trade_id: TradeId, caller_id: UserId, rating: u8) -> Result<(), AppError> {
        decide_and_apply(
            &self.trade_repository,
            trade_id,
            &caller_id,
            |trade, party| async move { Ok(trade.rate(party, rating)?) },
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::MockTradeRepository;
    use crate::application::service::trade::test_fixtures::{
        make_base_trade, make_initiator_id, make_stranger_id, mock_reading,
    };
    use crate::domain::error::FunctionalError;
    use crate::domain::trade::{Trade, TradeStatus};

    #[tokio::test]
    async fn rate_succeeds_for_initiator_on_completed_trade() {
        let mut mock_repository = mock_reading(vec![Trade {
            status: TradeStatus::Completed,
            ..make_base_trade()
        }]);
        mock_repository
            .expect_apply_transition()
            .times(1)
            .withf(|transition| {
                transition.next().initiator_rating == Some(5)
                    && transition.next().respondent_rating.is_none()
            })
            .returning(|_| Box::pin(async { Ok(true) }));

        let service = RateTradeService::new(Arc::new(mock_repository));
        let result = service.rate(TradeId::new(), make_initiator_id(), 5).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn rate_fails_when_trade_not_found() {
        let mut mock_repository = MockTradeRepository::new();
        mock_repository
            .expect_find_by_id()
            .times(1)
            .returning(|_| Box::pin(async { Ok(None) }));

        let service = RateTradeService::new(Arc::new(mock_repository));
        let result = service.rate(TradeId::new(), make_initiator_id(), 5).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::TradeNotFound))
        ));
    }

    #[tokio::test]
    async fn rate_fails_when_caller_is_not_a_party() {
        let trade = Trade {
            status: TradeStatus::Completed,
            ..make_base_trade()
        };
        let mut mock_repository = MockTradeRepository::new();
        mock_repository
            .expect_find_by_id()
            .times(1)
            .returning(move |_| {
                let trade = trade.clone();
                Box::pin(async move { Ok(Some(trade)) })
            });

        let service = RateTradeService::new(Arc::new(mock_repository));
        let result = service.rate(TradeId::new(), make_stranger_id(), 5).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::TradeAccessDenied))
        ));
    }

    #[tokio::test]
    async fn rate_fails_with_not_completed_when_status_is_not_completed() {
        let mut mock_repository = mock_reading(vec![make_base_trade()]);
        mock_repository.expect_apply_transition().times(0);

        let service = RateTradeService::new(Arc::new(mock_repository));
        let result = service.rate(TradeId::new(), make_initiator_id(), 5).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::TradeNotCompleted))
        ));
    }
}
