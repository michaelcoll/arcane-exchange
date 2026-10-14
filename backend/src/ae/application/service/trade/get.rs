use crate::application::error::{AppError, InfraError};
use crate::application::repository::{TradeRepository, UserRepository};
use crate::application::service::trade::find_as_party;
use crate::application::use_case::GetTradeUseCase;
use crate::domain::trade::{TradeDetail, TradeId};
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct GetTradeService {
    trade_repository: Arc<dyn TradeRepository>,
    user_repository: Arc<dyn UserRepository>,
}

impl GetTradeService {
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

/// A trade the database constraints should have made impossible.
fn invalid_trade(trade_id: TradeId, problem: &str) -> InfraError {
    InfraError::RepositoryError(format!("invalid trade {trade_id} from database: {problem}"))
}

#[async_trait]
impl GetTradeUseCase for GetTradeService {
    async fn get_trade(
        &self,
        trade_id: TradeId,
        caller_id: UserId,
    ) -> Result<TradeDetail, AppError> {
        let (trade, me) = find_as_party(&self.trade_repository, trade_id, &caller_id).await?;
        let partner = me.other();

        // `trade.initiator_user_id`/`respondent_user_id` are FK-constrained to `users.id`
        // (migration 0011) and `users.username` is `NOT NULL` (migration 0009), so a missing
        // partner or username is corrupt data: it surfaces as an `InfraError` (500), not a panic.
        let partner_username = self
            .user_repository
            .find_by_id(trade.user_id(partner))
            .await?
            .ok_or_else(|| invalid_trade(trade_id, "partner user not found"))?
            .username
            .ok_or_else(|| invalid_trade(trade_id, "partner has no username"))?;

        let cards = self
            .trade_repository
            .find_trade_cards_with_details(trade_id)
            .await?;
        let (my_cards, partner_cards) = cards
            .into_iter()
            .partition(|card| card.owner_user_id == caller_id);

        Ok(TradeDetail {
            id: trade.id,
            status: trade.status,
            partner_username,
            my_cards,
            partner_cards,
            me: trade.party_state(me),
            partner: trade.party_state(partner),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::{MockTradeRepository, MockUserRepository};
    use crate::application::service::trade::test_fixtures::{
        make_base_trade, make_card_id, make_initiator_id, make_respondent_id, make_stranger_id,
    };
    use crate::domain::error::FunctionalError;
    use crate::domain::trade::{Trade, TradeCardDetail};
    use crate::domain::user::User;

    fn make_trade_card_detail(owner: UserId) -> TradeCardDetail {
        TradeCardDetail {
            card_id: make_card_id(),
            owner_user_id: owner,
            name: "Goblin Boarders".to_string(),
            quantity: 1,
            price_guide: None,
            scryfall_id: uuid::Uuid::new_v4(),
            image: None,
        }
    }

    #[tokio::test]
    async fn get_trade_returns_detail_split_by_owner() {
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
            .expect_find_trade_cards_with_details()
            .times(1)
            .returning(|_| {
                Box::pin(async {
                    Ok(vec![
                        make_trade_card_detail(make_initiator_id()),
                        make_trade_card_detail(make_respondent_id()),
                    ])
                })
            });
        let mut mock_user_repository = MockUserRepository::new();
        mock_user_repository
            .expect_find_by_id()
            .times(1)
            .withf(|id| *id == make_respondent_id())
            .returning(|_| {
                Box::pin(async {
                    Ok(Some(User::new(
                        make_respondent_id(),
                        None,
                        Some("bob".to_string()),
                        None,
                    )))
                })
            });

        let service = GetTradeService::new(
            Arc::new(mock_trade_repository),
            Arc::new(mock_user_repository),
        );
        let detail = service
            .get_trade(TradeId::new(), make_initiator_id())
            .await
            .unwrap();

        assert_eq!(detail.partner_username, "bob");
        assert_eq!(detail.my_cards.len(), 1);
        assert_eq!(detail.my_cards[0].owner_user_id, make_initiator_id());
        assert_eq!(detail.partner_cards.len(), 1);
        assert_eq!(detail.partner_cards[0].owner_user_id, make_respondent_id());
    }

    #[tokio::test]
    async fn get_trade_fails_when_trade_not_found() {
        let mut mock_trade_repository = MockTradeRepository::new();
        mock_trade_repository
            .expect_find_by_id()
            .times(1)
            .returning(|_| Box::pin(async { Ok(None) }));

        let service = GetTradeService::new(
            Arc::new(mock_trade_repository),
            Arc::new(MockUserRepository::new()),
        );
        let result = service.get_trade(TradeId::new(), make_initiator_id()).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::TradeNotFound))
        ));
    }

    #[tokio::test]
    async fn get_trade_fails_when_caller_is_not_a_party() {
        let trade = make_base_trade();
        let mut mock_trade_repository = MockTradeRepository::new();
        mock_trade_repository
            .expect_find_by_id()
            .times(1)
            .returning(move |_| {
                let trade = trade.clone();
                Box::pin(async move { Ok(Some(trade)) })
            });

        let service = GetTradeService::new(
            Arc::new(mock_trade_repository),
            Arc::new(MockUserRepository::new()),
        );
        let result = service.get_trade(TradeId::new(), make_stranger_id()).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::TradeAccessDenied))
        ));
    }

    fn get_trade_service_with_partner(partner: Option<User>) -> GetTradeService {
        let trade = make_base_trade();
        let mut mock_trade_repository = MockTradeRepository::new();
        mock_trade_repository
            .expect_find_by_id()
            .returning(move |_| {
                let trade = trade.clone();
                Box::pin(async move { Ok(Some(trade)) })
            });
        let mut mock_user_repository = MockUserRepository::new();
        mock_user_repository
            .expect_find_by_id()
            .returning(move |_| {
                let partner = partner.clone();
                Box::pin(async move { Ok(partner) })
            });

        GetTradeService::new(
            Arc::new(mock_trade_repository),
            Arc::new(mock_user_repository),
        )
    }

    #[tokio::test]
    async fn get_trade_fails_with_infra_error_when_partner_user_is_missing() {
        let service = get_trade_service_with_partner(None);

        let result = service.get_trade(TradeId::new(), make_initiator_id()).await;

        assert!(matches!(
            result,
            Err(AppError::Infra(InfraError::RepositoryError(_)))
        ));
    }

    #[tokio::test]
    async fn get_trade_fails_with_infra_error_when_partner_username_is_missing() {
        let partner = User::new(make_respondent_id(), None, None, None);
        let service = get_trade_service_with_partner(Some(partner));

        let result = service.get_trade(TradeId::new(), make_initiator_id()).await;

        assert!(matches!(
            result,
            Err(AppError::Infra(InfraError::RepositoryError(_)))
        ));
    }

    #[tokio::test]
    async fn get_trade_me_and_partner_state_reflect_caller_perspective() {
        let trade = Trade {
            initiator_accepted_at: Some(chrono::Utc::now()),
            respondent_accepted_at: None,
            ..make_base_trade()
        };

        let mut mock_trade_repository_as_initiator = MockTradeRepository::new();
        let trade_for_initiator = trade.clone();
        mock_trade_repository_as_initiator
            .expect_find_by_id()
            .times(1)
            .returning(move |_| {
                let trade = trade_for_initiator.clone();
                Box::pin(async move { Ok(Some(trade)) })
            });
        mock_trade_repository_as_initiator
            .expect_find_trade_cards_with_details()
            .times(1)
            .returning(|_| Box::pin(async { Ok(vec![]) }));
        let mut mock_user_repository_as_initiator = MockUserRepository::new();
        mock_user_repository_as_initiator
            .expect_find_by_id()
            .times(1)
            .returning(|_| {
                Box::pin(async {
                    Ok(Some(User::new(
                        make_respondent_id(),
                        None,
                        Some("bob".to_string()),
                        None,
                    )))
                })
            });
        let service_as_initiator = GetTradeService::new(
            Arc::new(mock_trade_repository_as_initiator),
            Arc::new(mock_user_repository_as_initiator),
        );
        let detail_as_initiator = service_as_initiator
            .get_trade(TradeId::new(), make_initiator_id())
            .await
            .unwrap();
        assert!(detail_as_initiator.me.accepted);
        assert!(!detail_as_initiator.partner.accepted);

        let mut mock_trade_repository_as_respondent = MockTradeRepository::new();
        let trade_for_respondent = trade.clone();
        mock_trade_repository_as_respondent
            .expect_find_by_id()
            .times(1)
            .returning(move |_| {
                let trade = trade_for_respondent.clone();
                Box::pin(async move { Ok(Some(trade)) })
            });
        mock_trade_repository_as_respondent
            .expect_find_trade_cards_with_details()
            .times(1)
            .returning(|_| Box::pin(async { Ok(vec![]) }));
        let mut mock_user_repository_as_respondent = MockUserRepository::new();
        mock_user_repository_as_respondent
            .expect_find_by_id()
            .times(1)
            .returning(|_| {
                Box::pin(async {
                    Ok(Some(User::new(
                        make_initiator_id(),
                        None,
                        Some("alice".to_string()),
                        None,
                    )))
                })
            });
        let service_as_respondent = GetTradeService::new(
            Arc::new(mock_trade_repository_as_respondent),
            Arc::new(mock_user_repository_as_respondent),
        );
        let detail_as_respondent = service_as_respondent
            .get_trade(TradeId::new(), make_respondent_id())
            .await
            .unwrap();
        assert!(!detail_as_respondent.me.accepted);
        assert!(detail_as_respondent.partner.accepted);
    }
}
