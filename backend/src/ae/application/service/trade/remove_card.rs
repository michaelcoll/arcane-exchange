use crate::application::error::AppError;
use crate::application::repository::{TradeRepository, UserRepository};
use crate::application::service::trade::{find_as_party, resolve_owner};
use crate::application::use_case::RemoveTradeCardUseCase;
use crate::domain::card::CopyId;
use crate::domain::error::FunctionalError;
use crate::domain::trade::TradeId;
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct RemoveTradeCardService {
    trade_repository: Arc<dyn TradeRepository>,
    user_repository: Arc<dyn UserRepository>,
}

impl RemoveTradeCardService {
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
impl RemoveTradeCardUseCase for RemoveTradeCardService {
    async fn remove_card(
        &self,
        trade_id: TradeId,
        caller_id: UserId,
        owner_username: String,
        card_id: CopyId,
    ) -> Result<(), AppError> {
        let (trade, _) = find_as_party(&self.trade_repository, trade_id, &caller_id).await?;
        let owner_id = resolve_owner(&self.user_repository, &trade, &owner_username).await?;
        let transition = trade.modify()?;

        let removed = self
            .trade_repository
            .remove_card_from_trade(&transition, &card_id, &owner_id)
            .await?;
        if removed {
            Ok(())
        } else {
            Err(FunctionalError::TradeCardNotFound.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::{MockTradeRepository, MockUserRepository};
    use crate::application::service::trade::test_fixtures::{
        make_base_trade, make_card_id, make_initiator_id, make_respondent_user, make_stranger_id,
    };
    use crate::domain::trade::{Trade, TradeStatus};
    use crate::domain::user::User;

    #[tokio::test]
    async fn remove_card_succeeds_on_pending_trade_without_reopening() {
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
            .expect_remove_card_from_trade()
            .times(1)
            .withf(|transition, _, _| transition.from().status == TradeStatus::Pending)
            .returning(|_, _, _| Box::pin(async { Ok(true) }));
        let mut mock_user_repository = MockUserRepository::new();
        mock_user_repository
            .expect_find_by_username()
            .times(1)
            .returning(|_| Box::pin(async { Ok(Some(make_respondent_user())) }));

        let service = RemoveTradeCardService::new(
            Arc::new(mock_trade_repository),
            Arc::new(mock_user_repository),
        );
        let result = service
            .remove_card(
                TradeId::new(),
                make_initiator_id(),
                "respondent".to_string(),
                make_card_id(),
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn remove_card_reopens_one_accepted_trade() {
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
            .expect_remove_card_from_trade()
            .times(1)
            .withf(|transition, _, _| {
                transition.from().status == TradeStatus::OneAccepted
                    && transition.next().status == TradeStatus::Pending
            })
            .returning(|_, _, _| Box::pin(async { Ok(true) }));
        let mut mock_user_repository = MockUserRepository::new();
        mock_user_repository
            .expect_find_by_username()
            .times(1)
            .returning(|_| Box::pin(async { Ok(Some(make_respondent_user())) }));

        let service = RemoveTradeCardService::new(
            Arc::new(mock_trade_repository),
            Arc::new(mock_user_repository),
        );
        let result = service
            .remove_card(
                TradeId::new(),
                make_initiator_id(),
                "respondent".to_string(),
                make_card_id(),
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn remove_card_fails_with_trade_card_not_found_when_repository_returns_false() {
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
            .expect_remove_card_from_trade()
            .times(1)
            .returning(|_, _, _| Box::pin(async { Ok(false) }));
        let mut mock_user_repository = MockUserRepository::new();
        mock_user_repository
            .expect_find_by_username()
            .times(1)
            .returning(|_| Box::pin(async { Ok(Some(make_respondent_user())) }));

        let service = RemoveTradeCardService::new(
            Arc::new(mock_trade_repository),
            Arc::new(mock_user_repository),
        );
        let result = service
            .remove_card(
                TradeId::new(),
                make_initiator_id(),
                "respondent".to_string(),
                make_card_id(),
            )
            .await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::TradeCardNotFound))
        ));
    }

    #[tokio::test]
    async fn remove_card_fails_when_trade_not_found() {
        let mut mock_trade_repository = MockTradeRepository::new();
        mock_trade_repository
            .expect_find_by_id()
            .times(1)
            .returning(|_| Box::pin(async { Ok(None) }));

        let service = RemoveTradeCardService::new(
            Arc::new(mock_trade_repository),
            Arc::new(MockUserRepository::new()),
        );
        let result = service
            .remove_card(
                TradeId::new(),
                make_initiator_id(),
                "respondent".to_string(),
                make_card_id(),
            )
            .await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::TradeNotFound))
        ));
    }

    #[tokio::test]
    async fn remove_card_fails_when_caller_is_not_a_party() {
        let trade = make_base_trade();
        let mut mock_trade_repository = MockTradeRepository::new();
        mock_trade_repository
            .expect_find_by_id()
            .times(1)
            .returning(move |_| {
                let trade = trade.clone();
                Box::pin(async move { Ok(Some(trade)) })
            });

        let service = RemoveTradeCardService::new(
            Arc::new(mock_trade_repository),
            Arc::new(MockUserRepository::new()),
        );
        let result = service
            .remove_card(
                TradeId::new(),
                make_stranger_id(),
                "respondent".to_string(),
                make_card_id(),
            )
            .await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::TradeAccessDenied))
        ));
    }

    #[tokio::test]
    async fn remove_card_fails_when_owner_username_unknown() {
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

        let service = RemoveTradeCardService::new(
            Arc::new(mock_trade_repository),
            Arc::new(mock_user_repository),
        );
        let result = service
            .remove_card(
                TradeId::new(),
                make_initiator_id(),
                "unknown".to_string(),
                make_card_id(),
            )
            .await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::UserNotFound))
        ));
    }

    #[tokio::test]
    async fn remove_card_fails_when_owner_is_not_a_party() {
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

        let service = RemoveTradeCardService::new(
            Arc::new(mock_trade_repository),
            Arc::new(mock_user_repository),
        );
        let result = service
            .remove_card(
                TradeId::new(),
                make_initiator_id(),
                "stranger".to_string(),
                make_card_id(),
            )
            .await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::WrongFormat(_)))
        ));
    }

    #[tokio::test]
    async fn remove_card_fails_with_trade_not_modifiable_on_terminal_statuses() {
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

            let service = RemoveTradeCardService::new(
                Arc::new(mock_trade_repository),
                Arc::new(mock_user_repository),
            );
            let result = service
                .remove_card(
                    TradeId::new(),
                    make_initiator_id(),
                    "respondent".to_string(),
                    make_card_id(),
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
