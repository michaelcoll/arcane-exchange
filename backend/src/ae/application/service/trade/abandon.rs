use crate::application::error::AppError;
use crate::application::repository::TradeRepository;
use crate::application::service::trade::decide_and_apply;
use crate::application::use_case::AbandonTradeUseCase;
use crate::domain::trade::TradeId;
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct AbandonTradeService {
    trade_repository: Arc<dyn TradeRepository>,
}

impl AbandonTradeService {
    pub fn new(trade_repository: Arc<dyn TradeRepository>) -> Self {
        Self { trade_repository }
    }
}

#[async_trait]
impl AbandonTradeUseCase for AbandonTradeService {
    async fn abandon(&self, trade_id: TradeId, caller_id: UserId) -> Result<(), AppError> {
        decide_and_apply(
            &self.trade_repository,
            trade_id,
            &caller_id,
            |trade, _| async move { Ok(trade.abandon()?) },
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::MockTradeRepository;
    use crate::application::service::trade::test_fixtures::{
        make_base_trade, make_initiator_id, make_respondent_id, make_stranger_id, mock_reading,
    };
    use crate::domain::error::FunctionalError;
    use crate::domain::trade::{Trade, TradeStatus};

    #[tokio::test]
    async fn abandon_succeeds_for_party() {
        let mut mock_repository = mock_reading(vec![make_base_trade()]);
        mock_repository
            .expect_apply_transition()
            .times(1)
            .withf(|transition| transition.next().status == TradeStatus::Abandoned)
            .returning(|_| Box::pin(async { Ok(true) }));

        let service = AbandonTradeService::new(Arc::new(mock_repository));
        let result = service.abandon(TradeId::new(), make_respondent_id()).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn abandon_fails_when_trade_not_found() {
        let mut mock_repository = MockTradeRepository::new();
        mock_repository
            .expect_find_by_id()
            .times(1)
            .returning(|_| Box::pin(async { Ok(None) }));

        let service = AbandonTradeService::new(Arc::new(mock_repository));
        let result = service.abandon(TradeId::new(), make_initiator_id()).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::TradeNotFound))
        ));
    }

    #[tokio::test]
    async fn abandon_fails_when_caller_is_not_a_party() {
        let trade = make_base_trade();
        let mut mock_repository = MockTradeRepository::new();
        mock_repository
            .expect_find_by_id()
            .times(1)
            .returning(move |_| {
                let trade = trade.clone();
                Box::pin(async move { Ok(Some(trade)) })
            });

        let service = AbandonTradeService::new(Arc::new(mock_repository));
        let result = service.abandon(TradeId::new(), make_stranger_id()).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::TradeAccessDenied))
        ));
    }

    #[tokio::test]
    async fn abandon_fails_with_already_finalized_once_completed_meanwhile() {
        let completed = Trade {
            status: TradeStatus::Completed,
            ..make_base_trade()
        };
        let mut mock_repository = mock_reading(vec![
            Trade {
                status: TradeStatus::FullyAccepted,
                ..make_base_trade()
            },
            completed,
        ]);
        mock_repository
            .expect_apply_transition()
            .times(1)
            .returning(|_| Box::pin(async { Ok(false) }));

        let service = AbandonTradeService::new(Arc::new(mock_repository));
        let result = service.abandon(TradeId::new(), make_initiator_id()).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::TradeAlreadyFinalized))
        ));
    }
}
