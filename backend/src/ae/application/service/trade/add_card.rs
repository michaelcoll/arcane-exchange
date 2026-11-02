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
    /// Ensures `card_id` figures on `owner_username`'s side of the trade, at one copy: a card
    /// already there is left untouched (ADR-0021).
    async fn add_card(
        &self,
        trade_id: TradeId,
        caller_id: UserId,
        owner_username: String,
        card_id: CopyId,
    ) -> Result<(), AppError> {
        let (trade, _) = find_as_party(&self.trade_repository, trade_id, &caller_id).await?;
        let owner_id = resolve_owner(&self.user_repository, &trade, &owner_username).await?;

        let already_present = self
            .trade_repository
            .find_trade_cards(trade_id)
            .await?
            .iter()
            .any(|card| card.card_id == card_id && card.owner_user_id == owner_id);
        let Some(transition) = trade.add_card(already_present)? else {
            return Ok(());
        };

        // The caller disposes freely of their own side of the trade; a card put up on the other
        // party's behalf must be one that party actually offers to trade — see
        // `.agents/database-schema.instructions.md` on `v_tradable_entry`.
        let availability = if owner_id == caller_id {
            CardAvailability::Owned
        } else {
            CardAvailability::Offered
        };

        // Presence is read above without a lock: the repository checks it again under the
        // trade's lock, so a concurrent addition of the same card still gives a single copy.
        self.trade_repository
            .add_card_to_trade(&transition, &card_id, &owner_id, availability)
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
    use crate::domain::trade::{Trade, TradeCard, TradeStatus};
    use crate::domain::user::User;

    fn make_initiator_user() -> User {
        User::new(
            make_initiator_id().to_string(),
            None,
            Some("initiator".to_string()),
            None,
        )
    }

    /// The respondent already put `make_card_id()` on their side of the trade.
    fn respondent_card() -> TradeCard {
        TradeCard {
            card_id: make_card_id(),
            owner_user_id: make_respondent_id(),
            quantity: 1,
        }
    }

    /// A trade repository returning `trade`, holding `cards`.
    fn trade_repository_with(trade: Trade, cards: Vec<TradeCard>) -> MockTradeRepository {
        let mut mock_trade_repository = MockTradeRepository::new();
        mock_trade_repository
            .expect_find_by_id()
            .times(1)
            .returning(move |_| {
                let trade = trade.clone();
                Box::pin(async move { Ok(Some(trade)) })
            });
        mock_trade_repository
            .expect_find_trade_cards()
            .returning(move |_| {
                let cards = cards.clone();
                Box::pin(async move { Ok(cards) })
            });
        mock_trade_repository
    }

    fn user_repository_with(user: User) -> MockUserRepository {
        let mut mock_user_repository = MockUserRepository::new();
        mock_user_repository
            .expect_find_by_username()
            .times(1)
            .returning(move |_| {
                let user = user.clone();
                Box::pin(async move { Ok(Some(user)) })
            });
        mock_user_repository
    }

    /// Initiator adds `make_card_id()` to the side of `owner_username`.
    async fn add_card_owned_by(
        owner_username: &str,
        trade_repository: MockTradeRepository,
        user_repository: MockUserRepository,
    ) -> Result<(), AppError> {
        AddTradeCardService::new(Arc::new(trade_repository), Arc::new(user_repository))
            .add_card(
                TradeId::new(),
                make_initiator_id(),
                owner_username.to_string(),
                make_card_id(),
            )
            .await
    }

    /// Initiator adds `make_card_id()` to the respondent's side.
    async fn add_respondent_card(
        trade_repository: MockTradeRepository,
        user_repository: MockUserRepository,
    ) -> Result<(), AppError> {
        add_card_owned_by("respondent", trade_repository, user_repository).await
    }

    fn expect_write(
        mock_trade_repository: &mut MockTradeRepository,
        from: TradeStatus,
        next: TradeStatus,
    ) {
        mock_trade_repository
            .expect_add_card_to_trade()
            .times(1)
            .withf(move |transition, _, _, _| {
                transition.from().status == from && transition.next().status == next
            })
            .returning(|_, _, _, _| Box::pin(async { Ok(()) }));
    }

    #[tokio::test]
    async fn add_card_succeeds_on_pending_trade_without_reopening() {
        let mut trade_repository = trade_repository_with(make_base_trade(), vec![]);
        expect_write(
            &mut trade_repository,
            TradeStatus::Pending,
            TradeStatus::Pending,
        );

        let result = add_respondent_card(
            trade_repository,
            user_repository_with(make_respondent_user()),
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
        let mut trade_repository = trade_repository_with(trade, vec![]);
        expect_write(
            &mut trade_repository,
            TradeStatus::OneAccepted,
            TradeStatus::Pending,
        );

        let result = add_respondent_card(
            trade_repository,
            user_repository_with(make_respondent_user()),
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn add_card_writes_nothing_when_the_card_is_already_present() {
        // No `add_card_to_trade` expectation: any write would fail the test.
        for status in [
            TradeStatus::Pending,
            TradeStatus::OneAccepted,
            TradeStatus::FullyAccepted,
        ] {
            let trade = Trade {
                status,
                ..make_base_trade()
            };
            let trade_repository = trade_repository_with(trade, vec![respondent_card()]);

            let result = add_respondent_card(
                trade_repository,
                user_repository_with(make_respondent_user()),
            )
            .await;

            assert!(result.is_ok(), "status {status:?}");
        }
    }

    #[tokio::test]
    async fn add_card_on_the_other_side_is_not_a_present_card() {
        // Same card, but it is on the initiator's side: the respondent's side still lacks it.
        let initiator_card = TradeCard {
            owner_user_id: make_initiator_id(),
            ..respondent_card()
        };
        let mut trade_repository = trade_repository_with(make_base_trade(), vec![initiator_card]);
        expect_write(
            &mut trade_repository,
            TradeStatus::Pending,
            TradeStatus::Pending,
        );

        let result = add_respondent_card(
            trade_repository,
            user_repository_with(make_respondent_user()),
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

        let result = add_respondent_card(mock_trade_repository, MockUserRepository::new()).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::TradeNotFound))
        ));
    }

    #[tokio::test]
    async fn add_card_fails_when_caller_is_not_a_party() {
        let mut mock_trade_repository = MockTradeRepository::new();
        mock_trade_repository
            .expect_find_by_id()
            .times(1)
            .returning(|_| {
                let trade = make_base_trade();
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
            )
            .await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::TradeAccessDenied))
        ));
    }

    #[tokio::test]
    async fn add_card_fails_when_owner_username_unknown() {
        let mut mock_trade_repository = MockTradeRepository::new();
        mock_trade_repository
            .expect_find_by_id()
            .times(1)
            .returning(|_| {
                let trade = make_base_trade();
                Box::pin(async move { Ok(Some(trade)) })
            });
        let mut mock_user_repository = MockUserRepository::new();
        mock_user_repository
            .expect_find_by_username()
            .times(1)
            .returning(|_| Box::pin(async { Ok(None) }));

        let result =
            add_card_owned_by("unknown", mock_trade_repository, mock_user_repository).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::UserNotFound))
        ));
    }

    #[tokio::test]
    async fn add_card_fails_when_owner_is_not_a_party() {
        let mut mock_trade_repository = MockTradeRepository::new();
        mock_trade_repository
            .expect_find_by_id()
            .times(1)
            .returning(|_| {
                let trade = make_base_trade();
                Box::pin(async move { Ok(Some(trade)) })
            });
        let stranger = User::new(
            make_stranger_id().to_string(),
            None,
            Some("stranger".to_string()),
            None,
        );

        let result = add_card_owned_by(
            "stranger",
            mock_trade_repository,
            user_repository_with(stranger),
        )
        .await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::WrongFormat(_)))
        ));
    }

    #[tokio::test]
    async fn add_card_bounds_the_other_partys_card_by_their_offered_quantity() {
        let mut trade_repository = trade_repository_with(make_base_trade(), vec![]);
        trade_repository
            .expect_add_card_to_trade()
            .times(1)
            .withf(|_, _, owner, availability| {
                *owner == make_respondent_id() && *availability == CardAvailability::Offered
            })
            .returning(|_, _, _, _| Box::pin(async { Ok(()) }));

        let result = add_respondent_card(
            trade_repository,
            user_repository_with(make_respondent_user()),
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn add_card_uses_owned_quantity_when_owner_is_the_caller() {
        // Adding a card to one's own side of the trade is never subject to the tradability
        // rule — only how much the caller actually owns.
        let mut trade_repository = trade_repository_with(make_base_trade(), vec![]);
        trade_repository
            .expect_add_card_to_trade()
            .times(1)
            .withf(|_, _, owner, availability| {
                *owner == make_initiator_id() && *availability == CardAvailability::Owned
            })
            .returning(|_, _, _, _| Box::pin(async { Ok(()) }));

        let service = AddTradeCardService::new(
            Arc::new(trade_repository),
            Arc::new(user_repository_with(make_initiator_user())),
        );
        let result = service
            .add_card(
                TradeId::new(),
                make_initiator_id(),
                "initiator".to_string(),
                make_card_id(),
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn add_card_refuses_an_absent_card_once_the_trade_is_fully_accepted_or_over() {
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
            let result = add_respondent_card(
                trade_repository_with(trade, vec![]),
                user_repository_with(make_respondent_user()),
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

    #[tokio::test]
    async fn add_card_refuses_a_present_card_once_the_trade_is_over() {
        for status in [
            TradeStatus::Completed,
            TradeStatus::Closed,
            TradeStatus::Abandoned,
        ] {
            let trade = Trade {
                status,
                ..make_base_trade()
            };
            let result = add_respondent_card(
                trade_repository_with(trade, vec![respondent_card()]),
                user_repository_with(make_respondent_user()),
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
