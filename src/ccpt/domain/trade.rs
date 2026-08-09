use crate::domain::card::CardId;
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

#[derive(Clone, Debug, PartialEq, Eq)]
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

    pub fn from_db_str(s: &str) -> Self {
        match s {
            "PENDING" => TradeStatus::Pending,
            "ONE_ACCEPTED" => TradeStatus::OneAccepted,
            "FULLY_ACCEPTED" => TradeStatus::FullyAccepted,
            "COMPLETED" => TradeStatus::Completed,
            "CLOSED" => TradeStatus::Closed,
            "ABANDONED" => TradeStatus::Abandoned,
            _ => panic!("invalid trade status from database: {}", s),
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradeCard {
    pub card_id: CardId,
    pub owner_user_id: UserId,
    pub quantity: u32,
}

/// A card offered in a trade, enriched with the display data the trade detail screen needs
/// (name, price, image ids). `owner_user_id` is used by [`TradeDetail`]'s assembly to split
/// cards into `my_cards`/`partner_cards`; it isn't re-exposed once split.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradeCardDetail {
    pub card_id: CardId,
    pub owner_user_id: UserId,
    pub name: String,
    pub quantity: u32,
    pub price_guide: Option<PriceGuide>,
    pub scryfall_id: uuid::Uuid,
    pub the_gatherer_id: Option<String>,
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
/// layer, mirroring `resolve_party`.
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaginatedTrades {
    pub items: Vec<TradeSummary>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}

/// `statuses` empty means no filter (every status included).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradeListQuery {
    pub statuses: Vec<TradeStatus>,
    pub page: u32,
    pub page_size: u32,
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

    #[test]
    fn trade_status_round_trip_pending() {
        assert_eq!(
            TradeStatus::from_db_str(TradeStatus::Pending.as_db_str()),
            TradeStatus::Pending
        );
    }

    #[test]
    fn trade_status_round_trip_one_accepted() {
        assert_eq!(
            TradeStatus::from_db_str(TradeStatus::OneAccepted.as_db_str()),
            TradeStatus::OneAccepted
        );
    }

    #[test]
    fn trade_status_round_trip_fully_accepted() {
        assert_eq!(
            TradeStatus::from_db_str(TradeStatus::FullyAccepted.as_db_str()),
            TradeStatus::FullyAccepted
        );
    }

    #[test]
    fn trade_status_round_trip_completed() {
        assert_eq!(
            TradeStatus::from_db_str(TradeStatus::Completed.as_db_str()),
            TradeStatus::Completed
        );
    }

    #[test]
    fn trade_status_round_trip_closed() {
        assert_eq!(
            TradeStatus::from_db_str(TradeStatus::Closed.as_db_str()),
            TradeStatus::Closed
        );
    }

    #[test]
    fn trade_status_round_trip_abandoned() {
        assert_eq!(
            TradeStatus::from_db_str(TradeStatus::Abandoned.as_db_str()),
            TradeStatus::Abandoned
        );
    }

    #[test]
    #[should_panic(expected = "invalid trade status from database: UNKNOWN")]
    fn trade_status_from_db_str_panics_on_unknown_value() {
        TradeStatus::from_db_str("UNKNOWN");
    }
}
