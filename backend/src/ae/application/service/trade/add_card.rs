use crate::application::error::AppError;
use crate::application::repository::{CardAvailability, TradeRepository, UserRepository};
use crate::application::service::trade::{find_as_party, resolve_owner};
use crate::application::use_case::AddTradeCardUseCase;
use crate::domain::card::CopyId;
use crate::domain::trade::TradeId;
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct AddTradeCardService {
    trade_repository: Arc<dyn TradeRepository>,
    user_repository: Arc<dyn UserRepository>,
}

impl AddTradeCardService {
    pub fn new(
        trade_repository: Arc<dyn TradeRepository>,
        user_repository: Arc<dyn UserRepository>,
    ) -> Self {
        Self {
            trade_repository,
            user_repository,
        }
    }
}

#[async_trait]
impl AddTradeCardUseCase for AddTradeCardService {
    async fn add_card(
        &self,
        trade_id: TradeId,
        caller_id: UserId,
        owner_username: String,
        card_id: CopyId,
        quantity: u8,
    ) -> Result<(), AppError> {
        let (trade, _) = find_as_party(&self.trade_repository, trade_id, &caller_id).await?;
        let owner_id = resolve_owner(&self.user_repository, &trade, &owner_username).await?;
        let transition = trade.modify()?;

        // The caller disposes freely of their own side of the trade; a card put up on the other
        // party's behalf must be one that party actually offers to trade — see
        // `.agents/database-schema.instructions.md` on `v_tradable_entry`.
        let availability = if owner_id == caller_id {
            CardAvailability::Owned
        } else {
            CardAvailability::Offered
        };

        self.trade_repository
            .merge_card_into_trade(&transition, &card_id, &owner_id, quantity, availability)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::{MockTradeRepository, MockUserRepository};
    use crate::application::service::trade::test_fixtures::{
        make_base_trade, make_card_id, make_initiator_id, make_respondent_id, make_respondent_user,
        make_stranger_id,
    };
    use crate::domain::error::FunctionalError;
    use crate::domain::trade::{Trade, TradeStatus};
    use crate::domain::user::User;

    fn make_initiator_user() -> User {
        User::new(
            make_initiator_id().to_string(),
            None,
            Some("initiator".to_string()),
            None,
        )
    }

    #[tokio::test]
    async fn add_card_succeeds_on_pending_trade_without_reopening() {
        let trade = make_base_trade();
        let mut mock_trade_repository = MockTradeRepository::new();
        mock_trade_repository
            .expect_find_by_id()
            .times(1)
            .returning(move |_| {
                let trade = trade.clone();
                Box::pin(async move { Ok(Some(trade)) })
            });
        mock_trade_repository
            .expect_merge_card_into_trade()
            .times(1)
            .withf(|transition, _, _, _, _| {
                transition.from().status == TradeStatus::Pending
                    && transition.next().status == TradeStatus::Pending
            })
            .returning(|_, _, _, _, _| Box::pin(async { Ok(()) }));
        let mut mock_user_repository = MockUserRepository::new();
        mock_user_repository
            .expect_find_by_username()
            .times(1)
            .returning(|_| Box::pin(async { Ok(Some(make_respondent_user())) }));

        let service = AddTradeCardService::new(
            Arc::new(mock_trade_repository),
            Arc::new(mock_user_repository),
        );
        let result = service
            .add_card(
                TradeId::new(),
                make_initiator_id(),
                "respondent".to_string(),
                make_card_id(),
                1,
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn add_card_reopens_one_accepted_trade() {
        let trade = Trade {
            status: TradeStatus::OneAccepted,
            ..make_base_trade()
        };
        let mut mock_trade_repository = MockTradeRepository::new();
        mock_trade_repository
            .expect_find_by_id()
            .times(1)
            .returning(move |_| {
                let trade = trade.clone();
                Box::pin(async move { Ok(Some(trade)) })
            });
        mock_trade_repository
            .expect_merge_card_into_trade()
            .times(1)
            .withf(|transition, _, _, _, _| {
                transition.from().status == TradeStatus::OneAccepted
                    && transition.next().status == TradeStatus::Pending
            })
            .returning(|_, _, _, _, _| Box::pin(async { Ok(()) }));
        let mut mock_user_repository = MockUserRepository::new();
        mock_user_repository
            .expect_find_by_username()
            .times(1)
            .returning(|_| Box::pin(async { Ok(Some(make_respondent_user())) }));

        let service = AddTradeCardService::new(
            Arc::new(mock_trade_repository),
            Arc::new(mock_user_repository),
        );
        let result = service
            .add_card(
                TradeId::new(),
                make_initiator_id(),
                "respondent".to_string(),
                make_card_id(),
                1,
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn add_card_fails_when_trade_not_found() {
        let mut mock_trade_repository = MockTradeRepository::new();
        mock_trade_repository
            .expect_find_by_id()
            .times(1)
            .returning(|_| Box::pin(async { Ok(None) }));

        let service = AddTradeCardService::new(
            Arc::new(mock_trade_repository),
            Arc::new(MockUserRepository::new()),
        );
        let result = service
            .add_card(
                TradeId::new(),
                make_initiator_id(),
                "respondent".to_string(),
                make_card_id(),
                1,
            )
            .await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::TradeNotFound))
        ));
    }

    #[tokio::test]
    async fn add_card_fails_when_caller_is_not_a_party() {
        let trade = make_base_trade();
        let mut mock_trade_repository = MockTradeRepository::new();
        mock_trade_repository
            .expect_find_by_id()
            .times(1)
            .returning(move |_| {
                let trade = trade.clone();
                Box::pin(async move { Ok(Some(trade)) })
            });

        let service = AddTradeCardService::new(
            Arc::new(mock_trade_repository),
            Arc::new(MockUserRepository::new()),
        );
        let result = service
            .add_card(
                TradeId::new(),
                make_stranger_id(),
                "respondent".to_string(),
                make_card_id(),
                1,
            )
            .await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::TradeAccessDenied))
        ));
    }

    #[tokio::test]
    async fn add_card_fails_when_owner_username_unknown() {
        let trade = make_base_trade();
        let mut mock_trade_repository = MockTradeRepository::new();
        mock_trade_repository
            .expect_find_by_id()
            .times(1)
            .returning(move |_| {
                let trade = trade.clone();
                Box::pin(async move { Ok(Some(trade)) })
            });
        let mut mock_user_repository = MockUserRepository::new();
        mock_user_repository
            .expect_find_by_username()
            .times(1)
            .returning(|_| Box::pin(async { Ok(None) }));

        let service = AddTradeCardService::new(
            Arc::new(mock_trade_repository),
            Arc::new(mock_user_repository),
        );
        let result = service
            .add_card(
                TradeId::new(),
                make_initiator_id(),
                "unknown".to_string(),
                make_card_id(),
                1,
            )
            .await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::UserNotFound))
        ));
    }

    #[tokio::test]
    async fn add_card_fails_when_owner_is_not_a_party() {
        let trade = make_base_trade();
        let mut mock_trade_repository = MockTradeRepository::new();
        mock_trade_repository
            .expect_find_by_id()
            .times(1)
            .returning(move |_| {
                let trade = trade.clone();
                Box::pin(async move { Ok(Some(trade)) })
            });
        let mut mock_user_repository = MockUserRepository::new();
        mock_user_repository
            .expect_find_by_username()
            .times(1)
            .returning(|_| {
                Box::pin(async {
                    Ok(Some(User::new(
                        make_stranger_id().to_string(),
                        None,
                        Some("stranger".to_string()),
                        None,
                    )))
                })
            });

        let service = AddTradeCardService::new(
            Arc::new(mock_trade_repository),
            Arc::new(mock_user_repository),
        );
        let result = service
            .add_card(
                TradeId::new(),
                make_initiator_id(),
                "stranger".to_string(),
                make_card_id(),
                1,
            )
            .await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::WrongFormat(_)))
        ));
    }

    #[tokio::test]
    async fn add_card_bounds_the_other_partys_card_by_their_offered_quantity() {
        let trade = make_base_trade();
        let mut mock_trade_repository = MockTradeRepository::new();
        mock_trade_repository
            .expect_find_by_id()
            .times(1)
            .returning(move |_| {
                let trade = trade.clone();
                Box::pin(async move { Ok(Some(trade)) })
            });
        mock_trade_repository
            .expect_merge_card_into_trade()
            .times(1)
            .withf(|_, _, owner, _, availability| {
                *owner == make_respondent_id() && *availability == CardAvailability::Offered
            })
            .returning(|_, _, _, _, _| Box::pin(async { Ok(()) }));
        let mut mock_user_repository = MockUserRepository::new();
        mock_user_repository
            .expect_find_by_username()
            .times(1)
            .returning(|_| Box::pin(async { Ok(Some(make_respondent_user())) }));

        let service = AddTradeCardService::new(
            Arc::new(mock_trade_repository),
            Arc::new(mock_user_repository),
        );
        let result = service
            .add_card(
                TradeId::new(),
                make_initiator_id(),
                "respondent".to_string(),
                make_card_id(),
                2,
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn add_card_uses_owned_quantity_when_owner_is_the_caller() {
        // Adding a card to one's own side of the trade is never subject to the tradability
        // rule — only how much the caller actually owns.
        let trade = make_base_trade();
        let mut mock_trade_repository = MockTradeRepository::new();
        mock_trade_repository
            .expect_find_by_id()
            .times(1)
            .returning(move |_| {
                let trade = trade.clone();
                Box::pin(async move { Ok(Some(trade)) })
            });
        mock_trade_repository
            .expect_merge_card_into_trade()
            .times(1)
            .withf(|_, _, owner, _, availability| {
                *owner == make_initiator_id() && *availability == CardAvailability::Owned
            })
            .returning(|_, _, _, _, _| Box::pin(async { Ok(()) }));
        let mut mock_user_repository = MockUserRepository::new();
        mock_user_repository
            .expect_find_by_username()
            .times(1)
            .returning(|_| Box::pin(async { Ok(Some(make_initiator_user())) }));

        let service = AddTradeCardService::new(
            Arc::new(mock_trade_repository),
            Arc::new(mock_user_repository),
        );
        let result = service
            .add_card(
                TradeId::new(),
                make_initiator_id(),
                "initiator".to_string(),
                make_card_id(),
                1,
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn add_card_fails_with_trade_not_modifiable_on_terminal_statuses() {
        for status in [
            TradeStatus::FullyAccepted,
            TradeStatus::Completed,
            TradeStatus::Closed,
            TradeStatus::Abandoned,
        ] {
            let trade = Trade {
                status,
                ..make_base_trade()
            };
            let mut mock_trade_repository = MockTradeRepository::new();
            mock_trade_repository
                .expect_find_by_id()
                .times(1)
                .returning(move |_| {
                    let trade = trade.clone();
                    Box::pin(async move { Ok(Some(trade)) })
                });
            let mut mock_user_repository = MockUserRepository::new();
            mock_user_repository
                .expect_find_by_username()
                .times(1)
                .returning(|_| Box::pin(async { Ok(Some(make_respondent_user())) }));

            let service = AddTradeCardService::new(
                Arc::new(mock_trade_repository),
                Arc::new(mock_user_repository),
            );
            let result = service
                .add_card(
                    TradeId::new(),
                    make_initiator_id(),
                    "respondent".to_string(),
                    make_card_id(),
                    1,
                )
                .await;

            assert!(
                matches!(
                    result,
                    Err(AppError::Functional(FunctionalError::TradeNotModifiable))
                ),
                "status {status:?} should not be modifiable"
            );
        }
    }
}
