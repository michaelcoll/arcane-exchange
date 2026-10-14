pub mod abandon;
pub mod accept;
pub mod add_card;
pub mod confirm;
pub mod create;
pub mod get;
pub mod list;
pub mod rate;
pub mod remove_card;

use crate::application::error::AppError;
use crate::application::repository::{TradeRepository, UserRepository};
use crate::domain::error::FunctionalError;
use crate::domain::trade::{Party, Trade, TradeId, TradeTransition};
use crate::domain::user::UserId;
use std::sync::Arc;

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

/// Test fixtures shared by the tests of the Trade use cases.
#[cfg(test)]
mod test_fixtures {
    use crate::application::repository::MockTradeRepository;
    use crate::domain::card::CopyId;
    use crate::domain::language_code::LanguageCode;
    use crate::domain::trade::{Trade, TradeId, TradeStatus};
    use crate::domain::user::{User, UserId};

    pub(super) fn make_initiator_id() -> UserId {
        UserId::new("user_initiator")
    }

    pub(super) fn make_respondent_id() -> UserId {
        UserId::new("user_respondent")
    }

    pub(super) fn make_stranger_id() -> UserId {
        UserId::new("user_stranger")
    }

    pub(super) fn make_card_id() -> CopyId {
        CopyId::new("FDN", "87", LanguageCode::FR, false)
    }

    pub(super) fn make_respondent_user() -> User {
        User::new(
            make_respondent_id().to_string(),
            None,
            Some("respondent".to_string()),
            None,
        )
    }

    pub(super) fn make_base_trade() -> Trade {
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

    /// A repository whose `find_by_id` returns each of `reads` in turn, one per call.
    pub(super) fn mock_reading(reads: Vec<Trade>) -> MockTradeRepository {
        let mut mock_repository = MockTradeRepository::new();
        let mut reads = reads.into_iter();
        mock_repository.expect_find_by_id().returning(move |_| {
            let trade = reads.next().expect("unexpected extra read of the trade");
            Box::pin(async move { Ok(Some(trade)) })
        });
        mock_repository
    }
}
