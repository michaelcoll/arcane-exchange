use crate::domain::card::CopyId;
use crate::domain::card_image::CardImage;
use crate::domain::error::FunctionalError;
use crate::domain::pagination::{PageRequest, Pagination};
use crate::domain::price::PriceGuide;
use crate::domain::user::UserId;
use chrono::{DateTime, Utc};
use std::fmt::{Display, Formatter};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TradeId(pub uuid::Uuid);

impl TradeId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for TradeId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for TradeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TradeStatus {
    Pending,
    OneAccepted,
    FullyAccepted,
    Completed,
    Closed,
    Abandoned,
}

impl TradeStatus {
    pub fn as_db_str(&self) -> &'static str {
        match self {
            TradeStatus::Pending => "PENDING",
            TradeStatus::OneAccepted => "ONE_ACCEPTED",
            TradeStatus::FullyAccepted => "FULLY_ACCEPTED",
            TradeStatus::Completed => "COMPLETED",
            TradeStatus::Closed => "CLOSED",
            TradeStatus::Abandoned => "ABANDONED",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trade {
    pub id: TradeId,
    pub initiator_user_id: UserId,
    pub respondent_user_id: UserId,
    pub status: TradeStatus,
    pub initiator_amount_due: Option<u32>,
    pub respondent_amount_due: Option<u32>,
    pub initiator_accepted_at: Option<DateTime<Utc>>,
    pub respondent_accepted_at: Option<DateTime<Utc>>,
    pub initiator_confirmed_at: Option<DateTime<Utc>>,
    pub respondent_confirmed_at: Option<DateTime<Utc>>,
    pub initiator_rating: Option<u8>,
    pub respondent_rating: Option<u8>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Which side of a [`Trade`] a player is on. The role only identifies who acted: both parties
/// have exactly the same rights on the trade.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Party {
    Initiator,
    Respondent,
}

impl Party {
    pub fn other(self) -> Self {
        match self {
            Party::Initiator => Party::Respondent,
            Party::Respondent => Party::Initiator,
        }
    }
}

/// A state change decided by [`Trade`]: the trade as the decision read it, and as it must now be
/// written. It is only persisted if the trade's state is still exactly the one read, so a decision
/// taken on a stale read never reaches the database. Only `Trade` can build one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradeTransition {
    from: Trade,
    next: Trade,
}

impl TradeTransition {
    pub fn trade_id(&self) -> TradeId {
        self.next.id
    }

    /// The trade as the decision read it.
    pub fn from(&self) -> &Trade {
        &self.from
    }

    /// The trade as it must now be written.
    pub fn next(&self) -> &Trade {
        &self.next
    }

    /// The very first acceptance: the trade's cards become reserved, so every other active trade
    /// sharing one of them must be abandoned (ADR-0008).
    pub fn reserves_cards(&self) -> bool {
        self.from.status == TradeStatus::Pending && self.next.status == TradeStatus::OneAccepted
    }
}

/// One party's paired columns (ADR-0005), borrowed for writing.
struct PartyColumns<'a> {
    accepted_at: &'a mut Option<DateTime<Utc>>,
    confirmed_at: &'a mut Option<DateTime<Utc>>,
    rating: &'a mut Option<u8>,
}

impl Trade {
    /// Fails with `TradeAccessDenied` when `user_id` is not a party to this trade.
    pub fn party_of(&self, user_id: &UserId) -> Result<Party, FunctionalError> {
        if self.initiator_user_id == *user_id {
            Ok(Party::Initiator)
        } else if self.respondent_user_id == *user_id {
            Ok(Party::Respondent)
        } else {
            Err(FunctionalError::TradeAccessDenied)
        }
    }

    pub fn user_id(&self, party: Party) -> &UserId {
        match party {
            Party::Initiator => &self.initiator_user_id,
            Party::Respondent => &self.respondent_user_id,
        }
    }

    pub fn party_state(&self, party: Party) -> TradePartyState {
        let (accepted_at, confirmed_at, rating) = match party {
            Party::Initiator => (
                self.initiator_accepted_at,
                self.initiator_confirmed_at,
                self.initiator_rating,
            ),
            Party::Respondent => (
                self.respondent_accepted_at,
                self.respondent_confirmed_at,
                self.respondent_rating,
            ),
        };
        TradePartyState {
            accepted: accepted_at.is_some(),
            confirmed: confirmed_at.is_some(),
            rating,
        }
    }

    fn columns_mut(&mut self, party: Party) -> PartyColumns<'_> {
        match party {
            Party::Initiator => PartyColumns {
                accepted_at: &mut self.initiator_accepted_at,
                confirmed_at: &mut self.initiator_confirmed_at,
                rating: &mut self.initiator_rating,
            },
            Party::Respondent => PartyColumns {
                accepted_at: &mut self.respondent_accepted_at,
                confirmed_at: &mut self.respondent_confirmed_at,
                rating: &mut self.respondent_rating,
            },
        }
    }

    fn transition(&self, change: impl FnOnce(&mut Trade)) -> TradeTransition {
        let mut next = self.clone();
        change(&mut next);
        TradeTransition {
            from: self.clone(),
            next,
        }
    }

    /// A card added or removed by either party. `PENDING` stays as is; `ONE_ACCEPTED` goes back
    /// to `PENDING` with both acceptances withdrawn; anything later can no longer be modified.
    pub fn modify(&self) -> Result<TradeTransition, FunctionalError> {
        match self.status {
            TradeStatus::Pending => Ok(self.transition(|_| {})),
            TradeStatus::OneAccepted => Ok(self.transition(|next| {
                next.status = TradeStatus::Pending;
                next.initiator_accepted_at = None;
                next.respondent_accepted_at = None;
            })),
            TradeStatus::FullyAccepted
            | TradeStatus::Completed
            | TradeStatus::Closed
            | TradeStatus::Abandoned => Err(FunctionalError::TradeNotModifiable),
        }
    }

    /// `cards` are the cards currently in the trade: an empty trade cannot be accepted.
    pub fn accept(
        &self,
        party: Party,
        cards: &[TradeCard],
    ) -> Result<TradeTransition, FunctionalError> {
        match self.status {
            TradeStatus::Pending | TradeStatus::OneAccepted => {}
            TradeStatus::FullyAccepted
            | TradeStatus::Completed
            | TradeStatus::Closed
            | TradeStatus::Abandoned => return Err(FunctionalError::TradeNotAcceptable),
        }
        if self.party_state(party).accepted {
            return Err(FunctionalError::TradeAlreadyAccepted);
        }
        if cards.is_empty() {
            return Err(FunctionalError::TradeEmpty);
        }

        let fully_accepted = self.party_state(party.other()).accepted;
        Ok(self.transition(|next| {
            *next.columns_mut(party).accepted_at = Some(Utc::now());
            next.status = if fully_accepted {
                TradeStatus::FullyAccepted
            } else {
                TradeStatus::OneAccepted
            };
        }))
    }

    /// Confirms the physical exchange. The trade is `COMPLETED` once both parties have.
    pub fn confirm(&self, party: Party) -> Result<TradeTransition, FunctionalError> {
        if self.status != TradeStatus::FullyAccepted {
            return Err(FunctionalError::TradeNotFullyAccepted);
        }
        if self.party_state(party).confirmed {
            return Err(FunctionalError::TradeAlreadyConfirmed);
        }

        let completed = self.party_state(party.other()).confirmed;
        Ok(self.transition(|next| {
            *next.columns_mut(party).confirmed_at = Some(Utc::now());
            if completed {
                next.status = TradeStatus::Completed;
            }
        }))
    }

    /// Rates the other party. The trade is `CLOSED` once both parties have.
    pub fn rate(&self, party: Party, rating: u8) -> Result<TradeTransition, FunctionalError> {
        if self.status != TradeStatus::Completed {
            return Err(FunctionalError::TradeNotCompleted);
        }
        if self.party_state(party).rating.is_some() {
            return Err(FunctionalError::TradeAlreadyRated);
        }

        let closed = self.party_state(party.other()).rating.is_some();
        Ok(self.transition(|next| {
            *next.columns_mut(party).rating = Some(rating);
            if closed {
                next.status = TradeStatus::Closed;
            }
        }))
    }

    /// Either party can abandon the trade at any time before the physical exchange is confirmed.
    pub fn abandon(&self) -> Result<TradeTransition, FunctionalError> {
        match self.status {
            TradeStatus::Pending | TradeStatus::OneAccepted | TradeStatus::FullyAccepted => {
                Ok(self.transition(|next| next.status = TradeStatus::Abandoned))
            }
            TradeStatus::Completed | TradeStatus::Closed | TradeStatus::Abandoned => {
                Err(FunctionalError::TradeAlreadyFinalized)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradeCard {
    pub card_id: CopyId,
    pub owner_user_id: UserId,
    pub quantity: u32,
}

/// A card offered in a trade, enriched with the display data the trade detail screen needs
/// (name, price, image ids). `owner_user_id` is used by [`TradeDetail`]'s assembly to split
/// cards into `my_cards`/`partner_cards`; it isn't re-exposed once split.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradeCardDetail {
    pub card_id: CopyId,
    pub owner_user_id: UserId,
    pub name: String,
    pub quantity: u32,
    pub price_guide: Option<PriceGuide>,
    pub scryfall_id: uuid::Uuid,
    /// `None` while the card's image is pending.
    pub image: Option<CardImage>,
}

/// One party's acceptance/confirmation/rating state on a trade.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradePartyState {
    pub accepted: bool,
    pub confirmed: bool,
    pub rating: Option<u8>,
}

/// Full read model for a trade, already resolved from the caller's point of view (`me` vs
/// `partner`) — the `initiator_*`/`respondent_*` split never leaks past the repository/service
/// layer, mirroring [`Trade::party_of`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradeDetail {
    pub id: TradeId,
    pub status: TradeStatus,
    pub partner_username: String,
    pub my_cards: Vec<TradeCardDetail>,
    pub partner_cards: Vec<TradeCardDetail>,
    pub me: TradePartyState,
    pub partner: TradePartyState,
}

/// One row of the trade list: everything the summary screen needs without fetching each
/// trade's full card list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradeSummary {
    pub id: TradeId,
    pub status: TradeStatus,
    pub partner_username: String,
    pub my_card_count: u32,
    pub partner_card_count: u32,
    pub updated_at: DateTime<Utc>,
}

/// `statuses` empty means no filter (every status included).
///
/// `P` is the page: a raw [`PageRequest`] as it leaves the HTTP adapter, then a validated
/// [`Pagination`] once the use case has applied its own depth limit with
/// [`TradeListQuery::paginate`] — only the latter reaches a repository.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradeListQuery<P = Pagination> {
    pub statuses: Vec<TradeStatus>,
    pub pagination: P,
}

impl TradeListQuery<PageRequest> {
    /// Validates the requested page against `max_offset`, keeping the status filter.
    ///
    /// # Errors
    ///
    /// Same as [`PageRequest::paginate`].
    pub fn paginate(self, max_offset: u32) -> Result<TradeListQuery, FunctionalError> {
        Ok(TradeListQuery {
            statuses: self.statuses,
            pagination: self.pagination.paginate(max_offset)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trade_id_new_produces_a_valid_v4_uuid() {
        let id = TradeId::new();

        assert_eq!(id.0.get_version_num(), 4);
    }

    #[test]
    fn trade_id_new_produces_different_ids() {
        assert_ne!(TradeId::new(), TradeId::new());
    }

    // --- State machine ---

    const ALL_STATUSES: [TradeStatus; 6] = [
        TradeStatus::Pending,
        TradeStatus::OneAccepted,
        TradeStatus::FullyAccepted,
        TradeStatus::Completed,
        TradeStatus::Closed,
        TradeStatus::Abandoned,
    ];
    const BOTH_PARTIES: [Party; 2] = [Party::Initiator, Party::Respondent];

    fn make_trade(status: TradeStatus) -> Trade {
        let now = Utc::now();
        Trade {
            id: TradeId::new(),
            initiator_user_id: UserId::new("user_initiator"),
            respondent_user_id: UserId::new("user_respondent"),
            status,
            initiator_amount_due: None,
            respondent_amount_due: None,
            initiator_accepted_at: None,
            respondent_accepted_at: None,
            initiator_confirmed_at: None,
            respondent_confirmed_at: None,
            initiator_rating: None,
            respondent_rating: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn with_accepted(mut trade: Trade, party: Party) -> Trade {
        *trade.columns_mut(party).accepted_at = Some(Utc::now());
        trade
    }

    fn with_confirmed(mut trade: Trade, party: Party) -> Trade {
        *trade.columns_mut(party).confirmed_at = Some(Utc::now());
        trade
    }

    fn with_rating(mut trade: Trade, party: Party, rating: u8) -> Trade {
        *trade.columns_mut(party).rating = Some(rating);
        trade
    }

    fn make_cards() -> Vec<TradeCard> {
        vec![TradeCard {
            card_id: CopyId::new(
                "FDN",
                "87",
                crate::domain::language_code::LanguageCode::FR,
                false,
            ),
            owner_user_id: UserId::new("user_respondent"),
            quantity: 1,
        }]
    }

    #[test]
    fn party_of_resolves_both_parties() {
        let trade = make_trade(TradeStatus::Pending);

        assert_eq!(
            trade.party_of(&UserId::new("user_initiator")),
            Ok(Party::Initiator)
        );
        assert_eq!(
            trade.party_of(&UserId::new("user_respondent")),
            Ok(Party::Respondent)
        );
    }

    #[test]
    fn party_of_denies_a_stranger() {
        let trade = make_trade(TradeStatus::Pending);

        assert_eq!(
            trade.party_of(&UserId::new("user_stranger")),
            Err(FunctionalError::TradeAccessDenied)
        );
    }

    #[test]
    fn user_id_and_party_state_follow_the_party() {
        let trade = with_rating(
            with_accepted(make_trade(TradeStatus::OneAccepted), Party::Initiator),
            Party::Respondent,
            4,
        );

        assert_eq!(
            trade.user_id(Party::Respondent),
            &UserId::new("user_respondent")
        );
        assert_eq!(
            trade.party_state(Party::Initiator),
            TradePartyState {
                accepted: true,
                confirmed: false,
                rating: None
            }
        );
        assert_eq!(
            trade.party_state(Party::Respondent),
            TradePartyState {
                accepted: false,
                confirmed: false,
                rating: Some(4)
            }
        );
    }

    #[test]
    fn transition_records_the_state_it_was_decided_from() {
        let trade = make_trade(TradeStatus::Pending);

        let transition = trade.abandon().unwrap();

        assert_eq!(transition.from(), &trade);
        assert_eq!(transition.trade_id(), trade.id);
    }

    // modify

    #[test]
    fn modify_keeps_a_pending_trade_unchanged() {
        let trade = make_trade(TradeStatus::Pending);

        let transition = trade.modify().unwrap();

        assert_eq!(transition.next, trade);
    }

    #[test]
    fn modify_reopens_a_one_accepted_trade_and_withdraws_acceptances() {
        for party in BOTH_PARTIES {
            let trade = with_accepted(make_trade(TradeStatus::OneAccepted), party);

            let next = trade.modify().unwrap().next;

            assert_eq!(next.status, TradeStatus::Pending);
            assert_eq!(next.initiator_accepted_at, None);
            assert_eq!(next.respondent_accepted_at, None);
        }
    }

    #[test]
    fn modify_is_refused_from_fully_accepted_on() {
        for status in [
            TradeStatus::FullyAccepted,
            TradeStatus::Completed,
            TradeStatus::Closed,
            TradeStatus::Abandoned,
        ] {
            assert_eq!(
                make_trade(status).modify(),
                Err(FunctionalError::TradeNotModifiable),
                "status {status:?}"
            );
        }
    }

    // accept

    #[test]
    fn first_acceptance_moves_to_one_accepted_and_reserves_cards() {
        for party in BOTH_PARTIES {
            let trade = make_trade(TradeStatus::Pending);

            let transition = trade.accept(party, &make_cards()).unwrap();

            assert_eq!(transition.next.status, TradeStatus::OneAccepted);
            assert!(transition.next.party_state(party).accepted);
            assert!(!transition.next.party_state(party.other()).accepted);
            assert!(transition.reserves_cards());
        }
    }

    #[test]
    fn second_acceptance_moves_to_fully_accepted_without_reserving_again() {
        for party in BOTH_PARTIES {
            let trade = with_accepted(make_trade(TradeStatus::OneAccepted), party.other());

            let transition = trade.accept(party, &make_cards()).unwrap();

            assert_eq!(transition.next.status, TradeStatus::FullyAccepted);
            assert!(transition.next.party_state(party).accepted);
            assert!(transition.next.party_state(party.other()).accepted);
            assert!(!transition.reserves_cards());
        }
    }

    #[test]
    fn accept_is_refused_to_a_party_who_already_accepted() {
        for party in BOTH_PARTIES {
            let trade = with_accepted(make_trade(TradeStatus::OneAccepted), party);

            assert_eq!(
                trade.accept(party, &make_cards()),
                Err(FunctionalError::TradeAlreadyAccepted)
            );
        }
    }

    #[test]
    fn accept_is_refused_on_an_empty_trade() {
        let trade = make_trade(TradeStatus::Pending);

        assert_eq!(
            trade.accept(Party::Initiator, &[]),
            Err(FunctionalError::TradeEmpty)
        );
    }

    #[test]
    fn accept_is_refused_from_fully_accepted_on() {
        for status in [
            TradeStatus::FullyAccepted,
            TradeStatus::Completed,
            TradeStatus::Closed,
            TradeStatus::Abandoned,
        ] {
            for party in BOTH_PARTIES {
                assert_eq!(
                    make_trade(status).accept(party, &make_cards()),
                    Err(FunctionalError::TradeNotAcceptable),
                    "status {status:?}"
                );
            }
        }
    }

    // confirm

    #[test]
    fn first_confirmation_stays_fully_accepted() {
        for party in BOTH_PARTIES {
            let next = make_trade(TradeStatus::FullyAccepted)
                .confirm(party)
                .unwrap()
                .next;

            assert_eq!(next.status, TradeStatus::FullyAccepted);
            assert!(next.party_state(party).confirmed);
            assert!(!next.party_state(party.other()).confirmed);
        }
    }

    #[test]
    fn second_confirmation_completes_the_trade() {
        for party in BOTH_PARTIES {
            let trade = with_confirmed(make_trade(TradeStatus::FullyAccepted), party.other());

            let next = trade.confirm(party).unwrap().next;

            assert_eq!(next.status, TradeStatus::Completed);
            assert!(next.party_state(party).confirmed);
        }
    }

    #[test]
    fn confirm_is_refused_to_a_party_who_already_confirmed() {
        let trade = with_confirmed(make_trade(TradeStatus::FullyAccepted), Party::Initiator);

        assert_eq!(
            trade.confirm(Party::Initiator),
            Err(FunctionalError::TradeAlreadyConfirmed)
        );
    }

    #[test]
    fn confirm_is_refused_unless_fully_accepted() {
        for status in ALL_STATUSES
            .into_iter()
            .filter(|s| *s != TradeStatus::FullyAccepted)
        {
            assert_eq!(
                make_trade(status).confirm(Party::Respondent),
                Err(FunctionalError::TradeNotFullyAccepted),
                "status {status:?}"
            );
        }
    }

    // rate

    #[test]
    fn first_rating_stays_completed() {
        for party in BOTH_PARTIES {
            let next = make_trade(TradeStatus::Completed)
                .rate(party, 5)
                .unwrap()
                .next;

            assert_eq!(next.status, TradeStatus::Completed);
            assert_eq!(next.party_state(party).rating, Some(5));
            assert_eq!(next.party_state(party.other()).rating, None);
        }
    }

    #[test]
    fn second_rating_closes_the_trade() {
        for party in BOTH_PARTIES {
            let trade = with_rating(make_trade(TradeStatus::Completed), party.other(), 0);

            let next = trade.rate(party, 3).unwrap().next;

            assert_eq!(next.status, TradeStatus::Closed);
            assert_eq!(next.party_state(party).rating, Some(3));
            assert_eq!(next.party_state(party.other()).rating, Some(0));
        }
    }

    #[test]
    fn rate_is_refused_to_a_party_who_already_rated() {
        let trade = with_rating(make_trade(TradeStatus::Completed), Party::Respondent, 4);

        assert_eq!(
            trade.rate(Party::Respondent, 2),
            Err(FunctionalError::TradeAlreadyRated)
        );
    }

    #[test]
    fn rate_is_refused_unless_completed() {
        for status in ALL_STATUSES
            .into_iter()
            .filter(|s| *s != TradeStatus::Completed)
        {
            assert_eq!(
                make_trade(status).rate(Party::Initiator, 5),
                Err(FunctionalError::TradeNotCompleted),
                "status {status:?}"
            );
        }
    }

    // abandon

    #[test]
    fn abandon_is_allowed_before_the_exchange_is_confirmed() {
        for status in [
            TradeStatus::Pending,
            TradeStatus::OneAccepted,
            TradeStatus::FullyAccepted,
        ] {
            let transition = make_trade(status).abandon().unwrap();

            assert_eq!(transition.next.status, TradeStatus::Abandoned);
            assert!(!transition.reserves_cards());
        }
    }

    #[test]
    fn abandon_is_refused_once_finalized() {
        for status in [
            TradeStatus::Completed,
            TradeStatus::Closed,
            TradeStatus::Abandoned,
        ] {
            assert_eq!(
                make_trade(status).abandon(),
                Err(FunctionalError::TradeAlreadyFinalized),
                "status {status:?}"
            );
        }
    }
}
