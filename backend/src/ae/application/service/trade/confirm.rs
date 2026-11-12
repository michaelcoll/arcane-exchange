use crate::application::error::AppError;
use crate::application::repository::{CardPricesRefreshRequester, TradeRepository};
use crate::application::service::trade::decide_and_apply_transition;
use crate::application::use_case::ConfirmTradeUseCase;
use crate::domain::trade::TradeId;
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct ConfirmTradeService {
    trade_repository: Arc<dyn TradeRepository>,
    card_prices_refresh: Arc<dyn CardPricesRefreshRequester>,
}

impl ConfirmTradeService {
    pub fn new(
        trade_repository: Arc<dyn TradeRepository>,
        card_prices_refresh: Arc<dyn CardPricesRefreshRequester>,
    ) -> Self {
        Self {
            trade_repository,
            card_prices_refresh,
        }
    }
}

#[async_trait]
impl ConfirmTradeUseCase for ConfirmTradeService {
    async fn confirm(&self, trade_id: TradeId, caller_id: UserId) -> Result<(), AppError> {
        let transition = decide_and_apply_transition(
            &self.trade_repository,
            trade_id,
            &caller_id,
            |trade, party| async move { Ok(trade.confirm(party)?) },
        )
        .await?;

        // The Transfer moved cards between Collections: the views built on card prices follow.
        if transition.completes_trade() {
            self.card_prices_refresh.request_card_prices_refresh();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::{MockCardPricesRefreshRequester, MockTradeRepository};
    use crate::application::service::trade::test_fixtures::{
        make_base_trade, make_initiator_id, make_stranger_id, mock_reading,
    };
    use crate::domain::error::FunctionalError;
    use crate::domain::trade::{Trade, TradeStatus};
    use chrono::Utc;

    /// A service whose refresh requester expects to be asked exactly `refreshes` times.
    fn service(trades: MockTradeRepository, refreshes: usize) -> ConfirmTradeService {
        let mut refresh = MockCardPricesRefreshRequester::new();
        refresh
            .expect_request_card_prices_refresh()
            .times(refreshes)
            .return_const(());
        ConfirmTradeService::new(Arc::new(trades), Arc::new(refresh))
    }

    #[tokio::test]
    async fn confirm_completing_the_trade_requests_a_card_prices_refresh() {
        let mut mock_repository = mock_reading(vec![Trade {
            status: TradeStatus::FullyAccepted,
            respondent_confirmed_at: Some(Utc::now()),
            ..make_base_trade()
        }]);
        mock_repository
            .expect_apply_transition()
            .times(1)
            .withf(|transition| transition.completes_trade())
            .returning(|_| Box::pin(async { Ok(true) }));

        let service = service(mock_repository, 1);
        let result = service.confirm(TradeId::new(), make_initiator_id()).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn confirm_does_not_request_a_refresh_when_the_write_is_refused_every_time() {
        let mut mock_repository = mock_reading(vec![
            Trade {
                status: TradeStatus::FullyAccepted,
                respondent_confirmed_at: Some(Utc::now()),
                ..make_base_trade()
            };
            3
        ]);
        mock_repository
            .expect_apply_transition()
            .times(3)
            .returning(|_| Box::pin(async { Ok(false) }));

        let service = service(mock_repository, 0);
        let result = service.confirm(TradeId::new(), make_initiator_id()).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(
                FunctionalError::TradeConcurrentlyModified
            ))
        ));
    }

    #[tokio::test]
    async fn confirm_succeeds_for_initiator_on_fully_accepted_trade() {
        let mut mock_repository = mock_reading(vec![Trade {
            status: TradeStatus::FullyAccepted,
            ..make_base_trade()
        }]);
        mock_repository
            .expect_apply_transition()
            .times(1)
            .withf(|transition| {
                transition.next().initiator_confirmed_at.is_some()
                    && transition.next().respondent_confirmed_at.is_none()
            })
            .returning(|_| Box::pin(async { Ok(true) }));

        let service = service(mock_repository, 0);
        let result = service.confirm(TradeId::new(), make_initiator_id()).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn confirm_fails_when_trade_not_found() {
        let mut mock_repository = MockTradeRepository::new();
        mock_repository
            .expect_find_by_id()
            .times(1)
            .returning(|_| Box::pin(async { Ok(None) }));

        let service = service(mock_repository, 0);
        let result = service.confirm(TradeId::new(), make_initiator_id()).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::TradeNotFound))
        ));
    }

    #[tokio::test]
    async fn confirm_fails_when_caller_is_not_a_party() {
        let trade = Trade {
            status: TradeStatus::FullyAccepted,
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

        let service = service(mock_repository, 0);
        let result = service.confirm(TradeId::new(), make_stranger_id()).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::TradeAccessDenied))
        ));
    }

    #[tokio::test]
    async fn confirm_fails_with_not_fully_accepted_when_status_is_not_fully_accepted() {
        let mut mock_repository = mock_reading(vec![make_base_trade()]);
        mock_repository.expect_apply_transition().times(0);

        let service = service(mock_repository, 0);
        let result = service.confirm(TradeId::new(), make_initiator_id()).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::TradeNotFullyAccepted))
        ));
    }
}
