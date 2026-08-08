use crate::application::error::AppError;
use crate::application::repository::TradeRepository;
use crate::application::use_case::{
    AbandonTradeUseCase, AcceptTradeUseCase, ConfirmTradeUseCase, CreateTradeUseCase,
    RateTradeUseCase,
};
use crate::domain::card::CardId;
use crate::domain::error::FunctionalError;
use crate::domain::trade::{Trade, TradeId, TradeStatus};
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Determines whether `caller_id` is the initiator (`true`) or the respondent (`false`) of
/// `trade`. Fails when the caller is neither.
fn resolve_party(trade: &Trade, caller_id: &UserId) -> Result<bool, AppError> {
    if trade.initiator_user_id == *caller_id {
        Ok(true)
    } else if trade.respondent_user_id == *caller_id {
        Ok(false)
    } else {
        Err(FunctionalError::TradeAccessDenied.into())
    }
}

pub struct CreateTradeService {
    trade_repository: Arc<dyn TradeRepository>,
    creation_lock: Mutex<()>,
}

impl CreateTradeService {
    pub fn new(trade_repository: Arc<dyn TradeRepository>) -> Self {
        Self {
            trade_repository,
            creation_lock: Mutex::new(()),
        }
    }
}

#[async_trait]
impl CreateTradeUseCase for CreateTradeService {
    async fn create_trade(
        &self,
        initiator_user_id: UserId,
        respondent_user_id: UserId,
        card_id: CardId,
        quantity: u8,
    ) -> Result<TradeId, AppError> {
        let _guard = self.creation_lock.lock().await;

        let owned_quantity = self
            .trade_repository
            .find_collection_entry_quantity(&respondent_user_id, &card_id)
            .await?;
        match owned_quantity {
            Some(q) if q >= quantity as i32 => {}
            _ => return Err(FunctionalError::CardNotFound.into()),
        }

        if respondent_user_id == initiator_user_id {
            return Err(FunctionalError::SelfTrade.into());
        }

        let active_trade = self
            .trade_repository
            .find_active_trade(&initiator_user_id, &respondent_user_id)
            .await?;

        match active_trade {
            None => {
                let id = TradeId::new();
                self.trade_repository
                    .create(
                        id,
                        &initiator_user_id,
                        &respondent_user_id,
                        &card_id,
                        quantity,
                    )
                    .await?;
                Ok(id)
            }
            Some((trade_id, TradeStatus::Pending)) => {
                self.trade_repository
                    .merge_card_into_trade(trade_id, &card_id, &respondent_user_id, quantity, false)
                    .await?;
                Ok(trade_id)
            }
            Some((trade_id, TradeStatus::OneAccepted)) => {
                self.trade_repository
                    .merge_card_into_trade(trade_id, &card_id, &respondent_user_id, quantity, true)
                    .await?;
                Ok(trade_id)
            }
            Some((_, TradeStatus::FullyAccepted)) => {
                Err(FunctionalError::TradeNotModifiable.into())
            }
            Some((_, TradeStatus::Completed | TradeStatus::Closed | TradeStatus::Abandoned)) => {
                unreachable!(
                    "find_active_trade only returns PENDING, ONE_ACCEPTED or FULLY_ACCEPTED trades"
                )
            }
        }
    }
}

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
        let trade = self
            .trade_repository
            .find_by_id(trade_id)
            .await?
            .ok_or(FunctionalError::TradeNotFound)?;
        let is_initiator = resolve_party(&trade, &caller_id)?;

        let already_accepted = if is_initiator {
            trade.initiator_accepted_at.is_some()
        } else {
            trade.respondent_accepted_at.is_some()
        };
        let error = match trade.status {
            TradeStatus::Pending | TradeStatus::OneAccepted if already_accepted => {
                FunctionalError::TradeAlreadyAccepted
            }
            TradeStatus::Pending
            | TradeStatus::OneAccepted
            | TradeStatus::FullyAccepted
            | TradeStatus::Completed
            | TradeStatus::Closed
            | TradeStatus::Abandoned => FunctionalError::TradeNotAcceptable,
        };

        match self.trade_repository.accept(trade_id, is_initiator).await? {
            Some(_) => Ok(()),
            None => Err(error.into()),
        }
    }
}

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
        let trade = self
            .trade_repository
            .find_by_id(trade_id)
            .await?
            .ok_or(FunctionalError::TradeNotFound)?;
        resolve_party(&trade, &caller_id)?;

        if self.trade_repository.abandon(trade_id).await? {
            Ok(())
        } else {
            Err(FunctionalError::TradeAlreadyFinalized.into())
        }
    }
}

pub struct ConfirmTradeService {
    trade_repository: Arc<dyn TradeRepository>,
}

impl ConfirmTradeService {
    pub fn new(trade_repository: Arc<dyn TradeRepository>) -> Self {
        Self { trade_repository }
    }
}

#[async_trait]
impl ConfirmTradeUseCase for ConfirmTradeService {
    async fn confirm(&self, trade_id: TradeId, caller_id: UserId) -> Result<(), AppError> {
        let trade = self
            .trade_repository
            .find_by_id(trade_id)
            .await?
            .ok_or(FunctionalError::TradeNotFound)?;
        let is_initiator = resolve_party(&trade, &caller_id)?;

        let already_confirmed = if is_initiator {
            trade.initiator_confirmed_at.is_some()
        } else {
            trade.respondent_confirmed_at.is_some()
        };
        let error = match trade.status {
            TradeStatus::FullyAccepted if already_confirmed => {
                FunctionalError::TradeAlreadyConfirmed
            }
            TradeStatus::Pending
            | TradeStatus::OneAccepted
            | TradeStatus::FullyAccepted
            | TradeStatus::Completed
            | TradeStatus::Closed
            | TradeStatus::Abandoned => FunctionalError::TradeNotFullyAccepted,
        };

        match self
            .trade_repository
            .confirm(trade_id, is_initiator)
            .await?
        {
            Some(_) => Ok(()),
            None => Err(error.into()),
        }
    }
}

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
        let trade = self
            .trade_repository
            .find_by_id(trade_id)
            .await?
            .ok_or(FunctionalError::TradeNotFound)?;
        let is_initiator = resolve_party(&trade, &caller_id)?;

        let already_rated = if is_initiator {
            trade.initiator_rating.is_some()
        } else {
            trade.respondent_rating.is_some()
        };
        let error = match trade.status {
            TradeStatus::Completed if already_rated => FunctionalError::TradeAlreadyRated,
            TradeStatus::Pending
            | TradeStatus::OneAccepted
            | TradeStatus::FullyAccepted
            | TradeStatus::Completed
            | TradeStatus::Closed
            | TradeStatus::Abandoned => FunctionalError::TradeNotCompleted,
        };

        match self
            .trade_repository
            .rate(trade_id, is_initiator, rating)
            .await?
        {
            Some(_) => Ok(()),
            None => Err(error.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::MockTradeRepository;
    use crate::domain::language_code::LanguageCode;

    fn make_initiator_id() -> UserId {
        UserId::new("user_initiator")
    }

    fn make_respondent_id() -> UserId {
        UserId::new("user_respondent")
    }

    fn make_card_id() -> CardId {
        CardId::new("FDN", "87", LanguageCode::FR, false)
    }

    #[tokio::test]
    async fn create_trade_creates_new_trade_when_no_active_trade_exists() {
        let mut mock_repository = MockTradeRepository::new();
        mock_repository
            .expect_find_collection_entry_quantity()
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(Some(3)) }));
        mock_repository
            .expect_find_active_trade()
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(None) }));
        mock_repository
            .expect_create()
            .times(1)
            .returning(|_, _, _, _, _| Box::pin(async { Ok(()) }));

        let service = CreateTradeService::new(Arc::new(mock_repository));
        let result = service
            .create_trade(make_initiator_id(), make_respondent_id(), make_card_id(), 1)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn create_trade_fails_when_respondent_unknown_or_does_not_own_card() {
        let mut mock_repository = MockTradeRepository::new();
        mock_repository
            .expect_find_collection_entry_quantity()
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(None) }));

        let service = CreateTradeService::new(Arc::new(mock_repository));
        let result = service
            .create_trade(make_initiator_id(), make_respondent_id(), make_card_id(), 1)
            .await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::CardNotFound))
        ));
    }

    #[tokio::test]
    async fn create_trade_fails_when_respondent_owns_zero() {
        let mut mock_repository = MockTradeRepository::new();
        mock_repository
            .expect_find_collection_entry_quantity()
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(Some(0)) }));

        let service = CreateTradeService::new(Arc::new(mock_repository));
        let result = service
            .create_trade(make_initiator_id(), make_respondent_id(), make_card_id(), 1)
            .await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::CardNotFound))
        ));
    }

    #[tokio::test]
    async fn create_trade_fails_when_owned_quantity_is_insufficient() {
        let mut mock_repository = MockTradeRepository::new();
        mock_repository
            .expect_find_collection_entry_quantity()
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(Some(2)) }));

        let service = CreateTradeService::new(Arc::new(mock_repository));
        let result = service
            .create_trade(make_initiator_id(), make_respondent_id(), make_card_id(), 3)
            .await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::CardNotFound))
        ));
    }

    #[tokio::test]
    async fn create_trade_fails_on_self_targeting() {
        let mut mock_repository = MockTradeRepository::new();
        mock_repository
            .expect_find_collection_entry_quantity()
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(Some(1)) }));

        let initiator_id = make_initiator_id();
        let service = CreateTradeService::new(Arc::new(mock_repository));
        let result = service
            .create_trade(initiator_id.clone(), initiator_id, make_card_id(), 1)
            .await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::SelfTrade))
        ));
    }

    #[tokio::test]
    async fn create_trade_merges_into_pending_trade_without_reopening() {
        let mut mock_repository = MockTradeRepository::new();
        mock_repository
            .expect_find_collection_entry_quantity()
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(Some(1)) }));

        let existing_id = TradeId::new();
        mock_repository
            .expect_find_active_trade()
            .times(1)
            .returning(move |_, _| {
                Box::pin(async move { Ok(Some((existing_id, TradeStatus::Pending))) })
            });
        mock_repository
            .expect_merge_card_into_trade()
            .times(1)
            .withf(move |trade_id, _, _, _, reopen| *trade_id == existing_id && !*reopen)
            .returning(|_, _, _, _, _| Box::pin(async { Ok(()) }));

        let service = CreateTradeService::new(Arc::new(mock_repository));
        let result = service
            .create_trade(make_initiator_id(), make_respondent_id(), make_card_id(), 1)
            .await;

        assert_eq!(result.unwrap(), existing_id);
    }

    #[tokio::test]
    async fn create_trade_merges_into_one_accepted_trade_and_reopens_it() {
        let mut mock_repository = MockTradeRepository::new();
        mock_repository
            .expect_find_collection_entry_quantity()
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(Some(1)) }));

        let existing_id = TradeId::new();
        mock_repository
            .expect_find_active_trade()
            .times(1)
            .returning(move |_, _| {
                Box::pin(async move { Ok(Some((existing_id, TradeStatus::OneAccepted))) })
            });
        mock_repository
            .expect_merge_card_into_trade()
            .times(1)
            .withf(move |trade_id, _, _, _, reopen| *trade_id == existing_id && *reopen)
            .returning(|_, _, _, _, _| Box::pin(async { Ok(()) }));

        let service = CreateTradeService::new(Arc::new(mock_repository));
        let result = service
            .create_trade(make_initiator_id(), make_respondent_id(), make_card_id(), 1)
            .await;

        assert_eq!(result.unwrap(), existing_id);
    }

    #[tokio::test]
    async fn create_trade_fails_when_active_trade_is_fully_accepted() {
        let mut mock_repository = MockTradeRepository::new();
        mock_repository
            .expect_find_collection_entry_quantity()
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(Some(1)) }));
        mock_repository
            .expect_find_active_trade()
            .times(1)
            .returning(|_, _| {
                Box::pin(async { Ok(Some((TradeId::new(), TradeStatus::FullyAccepted))) })
            });

        let service = CreateTradeService::new(Arc::new(mock_repository));
        let result = service
            .create_trade(make_initiator_id(), make_respondent_id(), make_card_id(), 1)
            .await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::TradeNotModifiable))
        ));
    }

    #[tokio::test]
    async fn create_trade_lock_serializes_concurrent_creations() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let created = Arc::new(AtomicBool::new(false));

        let mut mock_repository = MockTradeRepository::new();
        mock_repository
            .expect_find_collection_entry_quantity()
            .times(2)
            .returning(|_, _| Box::pin(async { Ok(Some(1)) }));

        let created_for_find = created.clone();
        mock_repository
            .expect_find_active_trade()
            .times(2)
            .returning(move |_, _| {
                let found = created_for_find.load(Ordering::SeqCst);
                Box::pin(async move { Ok(found.then(|| (TradeId::new(), TradeStatus::Pending))) })
            });

        let created_for_create = created.clone();
        mock_repository
            .expect_create()
            .times(1)
            .returning(move |_, _, _, _, _| {
                created_for_create.store(true, Ordering::SeqCst);
                Box::pin(async { Ok(()) })
            });

        mock_repository
            .expect_merge_card_into_trade()
            .times(1)
            .returning(|_, _, _, _, _| Box::pin(async { Ok(()) }));

        let service = Arc::new(CreateTradeService::new(Arc::new(mock_repository)));

        let service_a = service.clone();
        let service_b = service.clone();
        let (result_a, result_b) = tokio::join!(
            service_a.create_trade(make_initiator_id(), make_respondent_id(), make_card_id(), 1),
            service_b.create_trade(make_initiator_id(), make_respondent_id(), make_card_id(), 1)
        );

        assert!(result_a.is_ok());
        assert!(result_b.is_ok());
    }

    fn make_base_trade() -> Trade {
        Trade {
            id: TradeId::new(),
            initiator_user_id: make_initiator_id(),
            respondent_user_id: make_respondent_id(),
            status: TradeStatus::Pending,
            initiator_amount_due: None,
            respondent_amount_due: None,
            initiator_accepted_at: None,
            respondent_accepted_at: None,
            initiator_confirmed_at: None,
            respondent_confirmed_at: None,
            initiator_rating: None,
            respondent_rating: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    fn make_stranger_id() -> UserId {
        UserId::new("user_stranger")
    }

    // --- AcceptTradeService ---

    #[tokio::test]
    async fn accept_succeeds_for_initiator_on_pending_trade() {
        let trade = make_base_trade();
        let mut mock_repository = MockTradeRepository::new();
        mock_repository
            .expect_find_by_id()
            .times(1)
            .returning(move |_| {
                let trade = trade.clone();
                Box::pin(async move { Ok(Some(trade)) })
            });
        mock_repository
            .expect_accept()
            .times(1)
            .withf(|_, is_initiator| *is_initiator)
            .returning(|_, _| Box::pin(async { Ok(Some(TradeStatus::OneAccepted)) }));

        let service = AcceptTradeService::new(Arc::new(mock_repository));
        let result = service.accept(TradeId::new(), make_initiator_id()).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn accept_succeeds_for_respondent_on_pending_trade() {
        let trade = make_base_trade();
        let mut mock_repository = MockTradeRepository::new();
        mock_repository
            .expect_find_by_id()
            .times(1)
            .returning(move |_| {
                let trade = trade.clone();
                Box::pin(async move { Ok(Some(trade)) })
            });
        mock_repository
            .expect_accept()
            .times(1)
            .withf(|_, is_initiator| !*is_initiator)
            .returning(|_, _| Box::pin(async { Ok(Some(TradeStatus::OneAccepted)) }));

        let service = AcceptTradeService::new(Arc::new(mock_repository));
        let result = service.accept(TradeId::new(), make_respondent_id()).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn accept_fails_when_trade_not_found() {
        let mut mock_repository = MockTradeRepository::new();
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
        let trade = make_base_trade();
        let mut mock_repository = MockTradeRepository::new();
        mock_repository
            .expect_find_by_id()
            .times(1)
            .returning(move |_| {
                let trade = trade.clone();
                Box::pin(async move { Ok(Some(trade)) })
            });

        let service = AcceptTradeService::new(Arc::new(mock_repository));
        let result = service.accept(TradeId::new(), make_stranger_id()).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::TradeAccessDenied))
        ));
    }

    #[tokio::test]
    async fn accept_fails_with_already_accepted_when_caller_already_accepted() {
        let trade = Trade {
            status: TradeStatus::OneAccepted,
            initiator_accepted_at: Some(chrono::Utc::now()),
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
        mock_repository
            .expect_accept()
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(None) }));

        let service = AcceptTradeService::new(Arc::new(mock_repository));
        let result = service.accept(TradeId::new(), make_initiator_id()).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::TradeAlreadyAccepted))
        ));
    }

    #[tokio::test]
    async fn accept_fails_with_not_acceptable_when_status_is_terminal() {
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
        mock_repository
            .expect_accept()
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(None) }));

        let service = AcceptTradeService::new(Arc::new(mock_repository));
        let result = service.accept(TradeId::new(), make_initiator_id()).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::TradeNotAcceptable))
        ));
    }

    // --- AbandonTradeService ---

    #[tokio::test]
    async fn abandon_succeeds_for_party() {
        let trade = make_base_trade();
        let mut mock_repository = MockTradeRepository::new();
        mock_repository
            .expect_find_by_id()
            .times(1)
            .returning(move |_| {
                let trade = trade.clone();
                Box::pin(async move { Ok(Some(trade)) })
            });
        mock_repository
            .expect_abandon()
            .times(1)
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
    async fn abandon_fails_with_already_finalized_when_repo_returns_false() {
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
        mock_repository
            .expect_abandon()
            .times(1)
            .returning(|_| Box::pin(async { Ok(false) }));

        let service = AbandonTradeService::new(Arc::new(mock_repository));
        let result = service.abandon(TradeId::new(), make_initiator_id()).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::TradeAlreadyFinalized))
        ));
    }

    // --- ConfirmTradeService ---

    #[tokio::test]
    async fn confirm_succeeds_for_initiator_on_fully_accepted_trade() {
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
        mock_repository
            .expect_confirm()
            .times(1)
            .withf(|_, is_initiator| *is_initiator)
            .returning(|_, _| Box::pin(async { Ok(Some(TradeStatus::FullyAccepted)) }));

        let service = ConfirmTradeService::new(Arc::new(mock_repository));
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

        let service = ConfirmTradeService::new(Arc::new(mock_repository));
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

        let service = ConfirmTradeService::new(Arc::new(mock_repository));
        let result = service.confirm(TradeId::new(), make_stranger_id()).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::TradeAccessDenied))
        ));
    }

    #[tokio::test]
    async fn confirm_fails_with_already_confirmed_when_caller_already_confirmed() {
        let trade = Trade {
            status: TradeStatus::FullyAccepted,
            initiator_confirmed_at: Some(chrono::Utc::now()),
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
        mock_repository
            .expect_confirm()
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(None) }));

        let service = ConfirmTradeService::new(Arc::new(mock_repository));
        let result = service.confirm(TradeId::new(), make_initiator_id()).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::TradeAlreadyConfirmed))
        ));
    }

    #[tokio::test]
    async fn confirm_fails_with_not_fully_accepted_when_status_is_not_fully_accepted() {
        let trade = make_base_trade();
        let mut mock_repository = MockTradeRepository::new();
        mock_repository
            .expect_find_by_id()
            .times(1)
            .returning(move |_| {
                let trade = trade.clone();
                Box::pin(async move { Ok(Some(trade)) })
            });
        mock_repository
            .expect_confirm()
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(None) }));

        let service = ConfirmTradeService::new(Arc::new(mock_repository));
        let result = service.confirm(TradeId::new(), make_initiator_id()).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::TradeNotFullyAccepted))
        ));
    }

    // --- RateTradeService ---

    #[tokio::test]
    async fn rate_succeeds_for_initiator_on_completed_trade() {
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
        mock_repository
            .expect_rate()
            .times(1)
            .withf(|_, is_initiator, rating| *is_initiator && *rating == 5)
            .returning(|_, _, _| Box::pin(async { Ok(Some(TradeStatus::Completed)) }));

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
    async fn rate_fails_with_already_rated_when_caller_already_rated() {
        let trade = Trade {
            status: TradeStatus::Completed,
            initiator_rating: Some(4),
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
        mock_repository
            .expect_rate()
            .times(1)
            .returning(|_, _, _| Box::pin(async { Ok(None) }));

        let service = RateTradeService::new(Arc::new(mock_repository));
        let result = service.rate(TradeId::new(), make_initiator_id(), 2).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::TradeAlreadyRated))
        ));
    }

    #[tokio::test]
    async fn rate_fails_with_not_completed_when_status_is_not_completed() {
        let trade = make_base_trade();
        let mut mock_repository = MockTradeRepository::new();
        mock_repository
            .expect_find_by_id()
            .times(1)
            .returning(move |_| {
                let trade = trade.clone();
                Box::pin(async move { Ok(Some(trade)) })
            });
        mock_repository
            .expect_rate()
            .times(1)
            .returning(|_, _, _| Box::pin(async { Ok(None) }));

        let service = RateTradeService::new(Arc::new(mock_repository));
        let result = service.rate(TradeId::new(), make_initiator_id(), 5).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::TradeNotCompleted))
        ));
    }
}
