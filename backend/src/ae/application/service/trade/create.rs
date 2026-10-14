use crate::application::error::AppError;
use crate::application::repository::{TradeRepository, UserRepository};
use crate::application::use_case::CreateTradeUseCase;
use crate::domain::error::FunctionalError;
use crate::domain::trade::TradeId;
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct CreateTradeService {
    trade_repository: Arc<dyn TradeRepository>,
    user_repository: Arc<dyn UserRepository>,
}

impl CreateTradeService {
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
impl CreateTradeUseCase for CreateTradeService {
    async fn create_trade(
        &self,
        initiator_user_id: UserId,
        respondent_username: String,
    ) -> Result<TradeId, AppError> {
        let respondent = self
            .user_repository
            .find_by_username(&respondent_username)
            .await?
            .ok_or(FunctionalError::UserNotFound)?;

        if respondent.id == initiator_user_id {
            return Err(FunctionalError::SelfTrade.into());
        }

        self.trade_repository
            .create_or_find_active(&initiator_user_id, &respondent.id)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::{MockTradeRepository, MockUserRepository};
    use crate::application::service::trade::test_fixtures::{
        make_initiator_id, make_respondent_id, make_respondent_user,
    };
    use crate::domain::user::User;

    #[tokio::test]
    async fn create_trade_returns_the_active_trade_of_the_pair() {
        let existing_id = TradeId::new();
        let mut mock_trade_repository = MockTradeRepository::new();
        mock_trade_repository
            .expect_create_or_find_active()
            .times(1)
            .withf(|initiator, respondent| {
                *initiator == make_initiator_id() && *respondent == make_respondent_id()
            })
            .returning(move |_, _| Box::pin(async move { Ok(existing_id) }));
        let mut mock_user_repository = MockUserRepository::new();
        mock_user_repository
            .expect_find_by_username()
            .times(1)
            .returning(|_| Box::pin(async { Ok(Some(make_respondent_user())) }));

        let service = CreateTradeService::new(
            Arc::new(mock_trade_repository),
            Arc::new(mock_user_repository),
        );
        let result = service
            .create_trade(make_initiator_id(), "respondent".to_string())
            .await;

        assert_eq!(result.unwrap(), existing_id);
    }

    #[tokio::test]
    async fn create_trade_fails_when_respondent_username_unknown() {
        let mock_trade_repository = MockTradeRepository::new();
        let mut mock_user_repository = MockUserRepository::new();
        mock_user_repository
            .expect_find_by_username()
            .times(1)
            .returning(|_| Box::pin(async { Ok(None) }));

        let service = CreateTradeService::new(
            Arc::new(mock_trade_repository),
            Arc::new(mock_user_repository),
        );
        let result = service
            .create_trade(make_initiator_id(), "unknown".to_string())
            .await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::UserNotFound))
        ));
    }

    #[tokio::test]
    async fn create_trade_fails_on_self_targeting() {
        let initiator_id = make_initiator_id();
        let mock_trade_repository = MockTradeRepository::new();
        let mut mock_user_repository = MockUserRepository::new();
        let initiator_id_for_mock = initiator_id.clone();
        mock_user_repository
            .expect_find_by_username()
            .times(1)
            .returning(move |_| {
                let user = User::new(
                    initiator_id_for_mock.to_string(),
                    None,
                    Some("initiator".to_string()),
                    None,
                );
                Box::pin(async move { Ok(Some(user)) })
            });

        let service = CreateTradeService::new(
            Arc::new(mock_trade_repository),
            Arc::new(mock_user_repository),
        );
        let result = service
            .create_trade(initiator_id, "initiator".to_string())
            .await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::SelfTrade))
        ));
    }
}
