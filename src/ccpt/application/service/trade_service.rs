use crate::application::error::AppError;
use crate::application::repository::{TradeRepository, UserRepository};
use crate::application::use_case::{
    AbandonTradeUseCase, AcceptTradeUseCase, AddTradeCardUseCase, ConfirmTradeUseCase,
    CreateTradeUseCase, GetTradeUseCase, ListTradesUseCase, RateTradeUseCase,
    RemoveTradeCardUseCase,
};
use crate::domain::card::CardId;
use crate::domain::error::FunctionalError;
use crate::domain::trade::{
    PaginatedTrades, Trade, TradeDetail, TradeId, TradeListQuery, TradePartyState, TradeStatus,
};
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

/// Reorders an (initiator, respondent) pair into (me, partner) from the caller's point of view.
fn perspective<T>(is_initiator: bool, initiator_val: T, respondent_val: T) -> (T, T) {
    if is_initiator {
        (initiator_val, respondent_val)
    } else {
        (respondent_val, initiator_val)
    }
}

/// `PENDING` → no reopening needed, `ONE_ACCEPTED` → reopen to `PENDING`, terminal statuses →
/// the trade cannot be modified at all.
fn reopen_flag_for_modification(status: &TradeStatus) -> Result<bool, AppError> {
    match status {
        TradeStatus::Pending => Ok(false),
        TradeStatus::OneAccepted => Ok(true),
        TradeStatus::FullyAccepted
        | TradeStatus::Completed
        | TradeStatus::Closed
        | TradeStatus::Abandoned => Err(FunctionalError::TradeNotModifiable.into()),
    }
}

/// Resolves `owner_username` to a `User` who must be a party to `trade`.
async fn resolve_owner(
    user_repository: &Arc<dyn UserRepository>,
    trade: &Trade,
    owner_username: &str,
) -> Result<UserId, AppError> {
    let owner = user_repository
        .find_by_username(owner_username)
        .await?
        .ok_or(FunctionalError::UserNotFound)?;

    if owner.id != trade.initiator_user_id && owner.id != trade.respondent_user_id {
        return Err(FunctionalError::WrongFormat(
            "owner_username must be a party to this trade".to_string(),
        )
        .into());
    }

    Ok(owner.id)
}

pub struct CreateTradeService {
    trade_repository: Arc<dyn TradeRepository>,
    user_repository: Arc<dyn UserRepository>,
    creation_lock: Mutex<()>,
}

impl CreateTradeService {
    pub fn new(
        trade_repository: Arc<dyn TradeRepository>,
        user_repository: Arc<dyn UserRepository>,
    ) -> Self {
        Self {
            trade_repository,
            user_repository,
            creation_lock: Mutex::new(()),
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

        let _guard = self.creation_lock.lock().await;

        match self
            .trade_repository
            .find_active_trade(&initiator_user_id, &respondent.id)
            .await?
        {
            Some((trade_id, _status)) => Ok(trade_id),
            None => {
                let id = TradeId::new();
                self.trade_repository
                    .create(id, &initiator_user_id, &respondent.id)
                    .await?;
                Ok(id)
            }
        }
    }
}

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
        card_id: CardId,
        quantity: u8,
    ) -> Result<(), AppError> {
        let trade = self
            .trade_repository
            .find_by_id(trade_id)
            .await?
            .ok_or(FunctionalError::TradeNotFound)?;
        resolve_party(&trade, &caller_id)?;

        let owner_id = resolve_owner(&self.user_repository, &trade, &owner_username).await?;
        let reopen = reopen_flag_for_modification(&trade.status)?;

        if self
            .trade_repository
            .is_card_reserved_elsewhere(trade_id, &owner_id, &card_id)
            .await?
        {
            return Err(FunctionalError::CardAlreadyReserved.into());
        }

        let owned_quantity = self
            .trade_repository
            .find_collection_entry_quantity(&owner_id, &card_id)
            .await?;
        match owned_quantity {
            Some(q) if q >= quantity as i32 => {}
            _ => return Err(FunctionalError::CardNotFound.into()),
        }

        self.trade_repository
            .merge_card_into_trade(trade_id, &card_id, &owner_id, quantity, reopen)
            .await
    }
}

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
        card_id: CardId,
    ) -> Result<(), AppError> {
        let trade = self
            .trade_repository
            .find_by_id(trade_id)
            .await?
            .ok_or(FunctionalError::TradeNotFound)?;
        resolve_party(&trade, &caller_id)?;

        let owner_id = resolve_owner(&self.user_repository, &trade, &owner_username).await?;
        let reopen = reopen_flag_for_modification(&trade.status)?;

        let removed = self
            .trade_repository
            .remove_card_from_trade(trade_id, &card_id, &owner_id, reopen)
            .await?;
        if removed {
            Ok(())
        } else {
            Err(FunctionalError::TradeCardNotFound.into())
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

        if matches!(
            trade.status,
            TradeStatus::Pending | TradeStatus::OneAccepted
        ) && !already_accepted
        {
            let cards = self.trade_repository.find_trade_cards(trade_id).await?;
            if cards.is_empty() {
                return Err(FunctionalError::TradeEmpty.into());
            }
        }

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

#[async_trait]
impl GetTradeUseCase for GetTradeService {
    async fn get_trade(
        &self,
        trade_id: TradeId,
        caller_id: UserId,
    ) -> Result<TradeDetail, AppError> {
        let trade = self
            .trade_repository
            .find_by_id(trade_id)
            .await?
            .ok_or(FunctionalError::TradeNotFound)?;
        let is_initiator = resolve_party(&trade, &caller_id)?;

        let partner_id = if is_initiator {
            &trade.respondent_user_id
        } else {
            &trade.initiator_user_id
        };
        // `trade.initiator_user_id`/`respondent_user_id` are FK-constrained to `users.id`
        // (migration 0011), so the partner always exists and always has a username
        // (`users.username` is `NOT NULL`, migration 0009).
        let partner_username = self
            .user_repository
            .find_by_id(partner_id)
            .await?
            .expect("database contains invalid trade: partner user not found")
            .username
            .expect("database contains invalid user record: missing username");

        let cards = self
            .trade_repository
            .find_trade_cards_with_details(trade_id)
            .await?;
        let (my_cards, partner_cards) = cards
            .into_iter()
            .partition(|card| card.owner_user_id == caller_id);

        let (me_accepted_at, partner_accepted_at) = perspective(
            is_initiator,
            trade.initiator_accepted_at,
            trade.respondent_accepted_at,
        );
        let (me_confirmed_at, partner_confirmed_at) = perspective(
            is_initiator,
            trade.initiator_confirmed_at,
            trade.respondent_confirmed_at,
        );
        let (me_rating, partner_rating) = perspective(
            is_initiator,
            trade.initiator_rating,
            trade.respondent_rating,
        );

        Ok(TradeDetail {
            id: trade.id,
            status: trade.status,
            partner_username,
            my_cards,
            partner_cards,
            me: TradePartyState {
                accepted: me_accepted_at.is_some(),
                confirmed: me_confirmed_at.is_some(),
                rating: me_rating,
            },
            partner: TradePartyState {
                accepted: partner_accepted_at.is_some(),
                confirmed: partner_confirmed_at.is_some(),
                rating: partner_rating,
            },
        })
    }
}

pub struct ListTradesService {
    trade_repository: Arc<dyn TradeRepository>,
}

impl ListTradesService {
    pub fn new(trade_repository: Arc<dyn TradeRepository>) -> Self {
        Self { trade_repository }
    }
}

#[async_trait]
impl ListTradesUseCase for ListTradesService {
    async fn list_trades(
        &self,
        caller_id: UserId,
        query: TradeListQuery,
    ) -> Result<PaginatedTrades, AppError> {
        self.trade_repository.list_trades(&caller_id, query).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::{MockTradeRepository, MockUserRepository};
    use crate::domain::language_code::LanguageCode;
    use crate::domain::trade::{TradeCard, TradeCardDetail};
    use crate::domain::user::User;

    fn make_initiator_id() -> UserId {
        UserId::new("user_initiator")
    }

    fn make_respondent_id() -> UserId {
        UserId::new("user_respondent")
    }

    fn make_card_id() -> CardId {
        CardId::new("FDN", "87", LanguageCode::FR, false)
    }

    fn make_respondent_user() -> User {
        User::new(
            make_respondent_id().to_string(),
            None,
            Some("respondent".to_string()),
        )
    }

    #[tokio::test]
    async fn create_trade_creates_new_trade_when_no_active_trade_exists() {
        let mut mock_trade_repository = MockTradeRepository::new();
        mock_trade_repository
            .expect_find_active_trade()
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(None) }));
        mock_trade_repository
            .expect_create()
            .times(1)
            .returning(|_, _, _| Box::pin(async { Ok(()) }));
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

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn create_trade_returns_existing_id_when_active_pending_trade_exists() {
        let existing_id = TradeId::new();
        let mut mock_trade_repository = MockTradeRepository::new();
        mock_trade_repository
            .expect_find_active_trade()
            .times(1)
            .returning(move |_, _| {
                Box::pin(async move { Ok(Some((existing_id, TradeStatus::Pending))) })
            });
        mock_trade_repository.expect_create().times(0);
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
    async fn create_trade_returns_existing_id_when_active_one_accepted_trade_exists() {
        let existing_id = TradeId::new();
        let mut mock_trade_repository = MockTradeRepository::new();
        mock_trade_repository
            .expect_find_active_trade()
            .times(1)
            .returning(move |_, _| {
                Box::pin(async move { Ok(Some((existing_id, TradeStatus::OneAccepted))) })
            });
        mock_trade_repository.expect_create().times(0);
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
    async fn create_trade_returns_existing_id_when_active_fully_accepted_trade_exists() {
        let existing_id = TradeId::new();
        let mut mock_trade_repository = MockTradeRepository::new();
        mock_trade_repository
            .expect_find_active_trade()
            .times(1)
            .returning(move |_, _| {
                Box::pin(async move { Ok(Some((existing_id, TradeStatus::FullyAccepted))) })
            });
        mock_trade_repository.expect_create().times(0);
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

    #[tokio::test]
    async fn create_trade_lock_serializes_concurrent_creations() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let created = Arc::new(AtomicBool::new(false));

        let mut mock_trade_repository = MockTradeRepository::new();
        let created_for_find = created.clone();
        mock_trade_repository
            .expect_find_active_trade()
            .times(2)
            .returning(move |_, _| {
                let found = created_for_find.load(Ordering::SeqCst);
                Box::pin(async move { Ok(found.then(|| (TradeId::new(), TradeStatus::Pending))) })
            });
        let created_for_create = created.clone();
        mock_trade_repository
            .expect_create()
            .times(1)
            .returning(move |_, _, _| {
                created_for_create.store(true, Ordering::SeqCst);
                Box::pin(async { Ok(()) })
            });

        let mut mock_user_repository = MockUserRepository::new();
        mock_user_repository
            .expect_find_by_username()
            .times(2)
            .returning(|_| Box::pin(async { Ok(Some(make_respondent_user())) }));

        let service = Arc::new(CreateTradeService::new(
            Arc::new(mock_trade_repository),
            Arc::new(mock_user_repository),
        ));

        let service_a = service.clone();
        let service_b = service.clone();
        let (result_a, result_b) = tokio::join!(
            service_a.create_trade(make_initiator_id(), "respondent".to_string()),
            service_b.create_trade(make_initiator_id(), "respondent".to_string())
        );

        assert!(result_a.is_ok());
        assert!(result_b.is_ok());
    }

    // --- AddTradeCardService ---

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
            .expect_is_card_reserved_elsewhere()
            .times(1)
            .returning(|_, _, _| Box::pin(async { Ok(false) }));
        mock_trade_repository
            .expect_find_collection_entry_quantity()
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(Some(3)) }));
        mock_trade_repository
            .expect_merge_card_into_trade()
            .times(1)
            .withf(|_, _, _, _, reopen| !*reopen)
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
            .expect_is_card_reserved_elsewhere()
            .times(1)
            .returning(|_, _, _| Box::pin(async { Ok(false) }));
        mock_trade_repository
            .expect_find_collection_entry_quantity()
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(Some(3)) }));
        mock_trade_repository
            .expect_merge_card_into_trade()
            .times(1)
            .withf(|_, _, _, _, reopen| *reopen)
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
    async fn add_card_fails_when_owner_does_not_own_enough_quantity() {
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
            .expect_is_card_reserved_elsewhere()
            .times(1)
            .returning(|_, _, _| Box::pin(async { Ok(false) }));
        mock_trade_repository
            .expect_find_collection_entry_quantity()
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(Some(0)) }));
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

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::CardNotFound))
        ));
    }

    #[tokio::test]
    async fn add_card_fails_when_card_already_reserved_elsewhere() {
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
            .expect_is_card_reserved_elsewhere()
            .times(1)
            .returning(|_, _, _| Box::pin(async { Ok(true) }));
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

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::CardAlreadyReserved))
        ));
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
                status: status.clone(),
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

    // --- RemoveTradeCardService ---

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
            .withf(|_, _, _, reopen| !*reopen)
            .returning(|_, _, _, _| Box::pin(async { Ok(true) }));
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
            .withf(|_, _, _, reopen| *reopen)
            .returning(|_, _, _, _| Box::pin(async { Ok(true) }));
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
            .returning(|_, _, _, _| Box::pin(async { Ok(false) }));
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
                status: status.clone(),
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

    fn make_trade_card() -> TradeCard {
        TradeCard {
            card_id: make_card_id(),
            owner_user_id: make_respondent_id(),
            quantity: 1,
        }
    }

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
            .expect_find_trade_cards()
            .times(1)
            .returning(|_| Box::pin(async { Ok(vec![make_trade_card()]) }));
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
            .expect_find_trade_cards()
            .times(1)
            .returning(|_| Box::pin(async { Ok(vec![make_trade_card()]) }));
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
    async fn accept_fails_with_trade_empty_when_no_cards() {
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
            .expect_find_trade_cards()
            .times(1)
            .returning(|_| Box::pin(async { Ok(vec![]) }));
        mock_repository.expect_accept().times(0);

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

    // --- GetTradeService ---

    fn make_trade_card_detail(owner: UserId) -> TradeCardDetail {
        TradeCardDetail {
            card_id: make_card_id(),
            owner_user_id: owner,
            name: "Goblin Boarders".to_string(),
            quantity: 1,
            price_guide: None,
            scryfall_id: uuid::Uuid::new_v4(),
            the_gatherer_id: None,
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

    // --- ListTradesService ---

    #[tokio::test]
    async fn list_trades_delegates_to_repository_with_caller_id_and_query() {
        let query = TradeListQuery {
            statuses: vec![TradeStatus::Pending],
            page: 0,
            page_size: 20,
        };
        let mut mock_repository = MockTradeRepository::new();
        mock_repository
            .expect_list_trades()
            .times(1)
            .withf(|caller_id, query| {
                *caller_id == make_initiator_id() && query.statuses == vec![TradeStatus::Pending]
            })
            .returning(|_, query| {
                Box::pin(async move {
                    Ok(PaginatedTrades {
                        items: vec![],
                        total: 0,
                        page: query.page,
                        page_size: query.page_size,
                    })
                })
            });

        let service = ListTradesService::new(Arc::new(mock_repository));
        let result = service
            .list_trades(make_initiator_id(), query)
            .await
            .unwrap();

        assert_eq!(result.total, 0);
    }
}
