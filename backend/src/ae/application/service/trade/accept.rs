use crate::application::error::AppError;
use crate::application::repository::TradeRepository;
use crate::application::service::trade::decide_and_apply;
use crate::application::use_case::AcceptTradeUseCase;
use crate::domain::trade::TradeId;
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct AcceptTradeService {
    trade_repository: Arc<dyn TradeRepository>,
}

impl AcceptTradeService {
    pub fn new(trade_repository: Arc<dyn TradeRepository>) -> Self {
        Self { trade_repository }
    }
}

#[async_trait]
impl AcceptTradeUseCase for AcceptTradeService {
    async fn accept(&self, trade_id: TradeId, caller_id: UserId) -> Result<(), AppError> {
        // Cards are read on every attempt, once the caller is known to be a party. The guard
        // covers the trade row only: the last card removed from a `PENDING` trade between this
        // read and the write leaves the trade accepted though empty.
        let trade_repository = &self.trade_repository;
        decide_and_apply(
            trade_repository,
            trade_id,
            &caller_id,
            |trade, party| async move {
                let cards = trade_repository.find_trade_cards(trade.id).await?;
                Ok(trade.accept(party, &cards)?)
            },
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::MockTradeRepository;
    use crate::application::service::trade::TRANSITION_ATTEMPTS;
    use crate::application::service::trade::test_fixtures::{
        make_base_trade, make_card_id, make_initiator_id, make_respondent_id, make_stranger_id,
        mock_reading,
    };
    use crate::domain::error::FunctionalError;
    use crate::domain::trade::{Trade, TradeCard, TradeStatus};

    fn make_trade_card() -> TradeCard {
        TradeCard {
            card_id: make_card_id(),
            owner_user_id: make_respondent_id(),
            quantity: 1,
        }
    }

    fn with_cards(mut mock_repository: MockTradeRepository) -> MockTradeRepository {
        mock_repository
            .expect_find_trade_cards()
            .returning(|_| Box::pin(async { Ok(vec![make_trade_card()]) }));
        mock_repository
    }

    #[tokio::test]
    async fn accept_applies_the_transition_decided_by_the_trade() {
        let mut mock_repository = with_cards(mock_reading(vec![make_base_trade()]));
        mock_repository
            .expect_apply_transition()
            .times(1)
            .withf(|transition| {
                transition.from().status == TradeStatus::Pending
                    && transition.next().status == TradeStatus::OneAccepted
                    && transition.next().respondent_accepted_at.is_some()
            })
            .returning(|_| Box::pin(async { Ok(true) }));

        let service = AcceptTradeService::new(Arc::new(mock_repository));
        let result = service.accept(TradeId::new(), make_respondent_id()).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn accept_returns_the_refusal_of_the_trade_without_writing() {
        let mut mock_repository = mock_reading(vec![make_base_trade()]);
        mock_repository
            .expect_find_trade_cards()
            .returning(|_| Box::pin(async { Ok(vec![]) }));
        mock_repository.expect_apply_transition().times(0);

        let service = AcceptTradeService::new(Arc::new(mock_repository));
        let result = service.accept(TradeId::new(), make_initiator_id()).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::TradeEmpty))
        ));
    }

    #[tokio::test]
    async fn accept_fails_when_trade_not_found() {
        let mut mock_repository = MockTradeRepository::new();
        mock_repository
            .expect_find_trade_cards()
            .returning(|_| Box::pin(async { Ok(vec![]) }));
        mock_repository
            .expect_find_by_id()
            .times(1)
            .returning(|_| Box::pin(async { Ok(None) }));

        let service = AcceptTradeService::new(Arc::new(mock_repository));
        let result = service.accept(TradeId::new(), make_initiator_id()).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::TradeNotFound))
        ));
    }

    #[tokio::test]
    async fn accept_fails_when_caller_is_not_a_party() {
        let mut mock_repository = with_cards(mock_reading(vec![make_base_trade()]));
        mock_repository.expect_apply_transition().times(0);

        let service = AcceptTradeService::new(Arc::new(mock_repository));
        let result = service.accept(TradeId::new(), make_stranger_id()).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::TradeAccessDenied))
        ));
    }

    #[tokio::test]
    async fn accept_decides_again_on_fresh_state_after_a_concurrent_change() {
        // A double submission: both requests read `PENDING`, the other one commits first.
        let already_accepted = Trade {
            status: TradeStatus::OneAccepted,
            initiator_accepted_at: Some(chrono::Utc::now()),
            ..make_base_trade()
        };
        let mut mock_repository =
            with_cards(mock_reading(vec![make_base_trade(), already_accepted]));
        mock_repository
            .expect_apply_transition()
            .times(1)
            .returning(|_| Box::pin(async { Ok(false) }));

        let service = AcceptTradeService::new(Arc::new(mock_repository));
        let result = service.accept(TradeId::new(), make_initiator_id()).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::TradeAlreadyAccepted))
        ));
    }

    #[tokio::test]
    async fn accept_gives_up_when_the_trade_keeps_changing() {
        let mut mock_repository = with_cards(mock_reading(vec![
            make_base_trade(),
            make_base_trade(),
            make_base_trade(),
        ]));
        mock_repository
            .expect_apply_transition()
            .times(TRANSITION_ATTEMPTS)
            .returning(|_| Box::pin(async { Ok(false) }));

        let service = AcceptTradeService::new(Arc::new(mock_repository));
        let result = service.accept(TradeId::new(), make_initiator_id()).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(
                FunctionalError::TradeConcurrentlyModified
            ))
        ));
    }
}
