use crate::application::error::AppError;
use crate::application::repository::{CardAvailability, TradeRepository, UserRepository};
use crate::application::use_case::{
    AbandonTradeUseCase, AcceptTradeUseCase, AddTradeCardUseCase, ConfirmTradeUseCase,
    CreateTradeUseCase, GetTradeUseCase, ListTradesUseCase, RateTradeUseCase,
    RemoveTradeCardUseCase,
};
use crate::domain::card::CopyId;
use crate::domain::error::FunctionalError;
use crate::domain::pagination::Paginated;
use crate::domain::trade::{
    Party, Trade, TradeDetail, TradeId, TradeListQuery, TradeSummary, TradeTransition,
};
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

/// Trade history is bounded by how many trades a user can realistically accumulate, so it
/// needs less depth than the collection or search endpoints, but more than card offers.
pub(crate) const TRADES_MAX_OFFSET: u32 = 2_000;

/// An attempt only fails when another request changed the trade between its read and its write;
/// the next one decides again on the fresh state, which most often yields the exact error (e.g.
/// `TradeAlreadyAccepted` after a double submission).
const TRANSITION_ATTEMPTS: usize = 3;

/// Loads `trade_id` and resolves which of its parties `caller_id` is.
async fn find_as_party(
    trade_repository: &Arc<dyn TradeRepository>,
    trade_id: TradeId,
    caller_id: &UserId,
) -> Result<(Trade, Party), AppError> {
    let trade = trade_repository
        .find_by_id(trade_id)
        .await?
        .ok_or(FunctionalError::TradeNotFound)?;
    let party = trade.party_of(caller_id)?;
    Ok((trade, party))
}

/// Lets `decide` take the caller's transition on the current state of the trade, then persists
/// it — deciding again on fresh state whenever the trade changed in between.
async fn decide_and_apply<Decision>(
    trade_repository: &Arc<dyn TradeRepository>,
    trade_id: TradeId,
    caller_id: &UserId,
    decide: impl Fn(Trade, Party) -> Decision,
) -> Result<(), AppError>
where
    Decision: Future<Output = Result<TradeTransition, AppError>>,
{
    for _ in 0..TRANSITION_ATTEMPTS {
        let (trade, party) = find_as_party(trade_repository, trade_id, caller_id).await?;
        let transition = decide(trade, party).await?;
        if trade_repository.apply_transition(&transition).await? {
            return Ok(());
        }
    }

    Err(FunctionalError::TradeConcurrentlyModified.into())
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

    trade.party_of(&owner.id).map_err(|_| {
        FunctionalError::WrongFormat("owner_username must be a party to this trade".to_string())
    })?;

    Ok(owner.id)
}

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
        // read and the write leaves the trade accepted though empty (same as before the domain
        // held these rules).
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
        decide_and_apply(
            &self.trade_repository,
            trade_id,
            &caller_id,
            |trade, party| async move { Ok(trade.confirm(party)?) },
        )
        .await
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
        decide_and_apply(
            &self.trade_repository,
            trade_id,
            &caller_id,
            |trade, party| async move { Ok(trade.rate(party, rating)?) },
        )
        .await
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
        let (trade, me) = find_as_party(&self.trade_repository, trade_id, &caller_id).await?;
        let partner = me.other();

        // `trade.initiator_user_id`/`respondent_user_id` are FK-constrained to `users.id`
        // (migration 0011), so the partner always exists and always has a username
        // (`users.username` is `NOT NULL`, migration 0009).
        let partner_username = self
            .user_repository
            .find_by_id(trade.user_id(partner))
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
    ) -> Result<Paginated<TradeSummary>, AppError> {
        self.trade_repository.list_trades(&caller_id, query).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::{MockTradeRepository, MockUserRepository};
    use crate::domain::language_code::LanguageCode;
    use crate::domain::pagination::Pagination;
    use crate::domain::trade::{TradeCard, TradeCardDetail, TradeStatus};
    use crate::domain::user::User;

    fn make_initiator_id() -> UserId {
        UserId::new("user_initiator")
    }

    fn make_respondent_id() -> UserId {
        UserId::new("user_respondent")
    }

    fn make_card_id() -> CopyId {
        CopyId::new("FDN", "87", LanguageCode::FR, false)
    }

    fn make_respondent_user() -> User {
        User::new(
            make_respondent_id().to_string(),
            None,
            Some("respondent".to_string()),
            None,
        )
    }

    fn make_initiator_user() -> User {
        User::new(
            make_initiator_id().to_string(),
            None,
            Some("initiator".to_string()),
            None,
        )
    }

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

    /// A repository whose `find_by_id` returns each of `reads` in turn, one per call.
    fn mock_reading(reads: Vec<Trade>) -> MockTradeRepository {
        let mut mock_repository = MockTradeRepository::new();
        let mut reads = reads.into_iter();
        mock_repository.expect_find_by_id().returning(move |_| {
            let trade = reads.next().expect("unexpected extra read of the trade");
            Box::pin(async move { Ok(Some(trade)) })
        });
        mock_repository
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

    // --- AbandonTradeService ---

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

    // --- ConfirmTradeService ---

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
    async fn confirm_fails_with_not_fully_accepted_when_status_is_not_fully_accepted() {
        let mut mock_repository = mock_reading(vec![make_base_trade()]);
        mock_repository.expect_apply_transition().times(0);

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
        let mut mock_repository = mock_reading(vec![Trade {
            status: TradeStatus::Completed,
            ..make_base_trade()
        }]);
        mock_repository
            .expect_apply_transition()
            .times(1)
            .withf(|transition| {
                transition.next().initiator_rating == Some(5)
                    && transition.next().respondent_rating.is_none()
            })
            .returning(|_| Box::pin(async { Ok(true) }));

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
    async fn rate_fails_with_not_completed_when_status_is_not_completed() {
        let mut mock_repository = mock_reading(vec![make_base_trade()]);
        mock_repository.expect_apply_transition().times(0);

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

    // --- ListTradesService ---

    #[tokio::test]
    async fn list_trades_delegates_to_repository_with_caller_id_and_query() {
        let query = TradeListQuery {
            statuses: vec![TradeStatus::Pending],
            pagination: Pagination::try_new(0, 20, TRADES_MAX_OFFSET).unwrap(),
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
                    Ok(Paginated {
                        items: vec![],
                        total: 0,
                        pagination: query.pagination,
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
