use crate::application::error::InfraError;
use crate::application::repository::CardImageLookup;
use crate::domain::card::{Card, CardId, CollectionEntry, CopyId};
use crate::domain::language_code::LanguageCode;
use crate::domain::price::{FullPriceGuide, Price, PriceGuide, PriceHistoryEntry};
use crate::domain::rarity_code::RarityCode;
use crate::domain::set_name::{SetCode, SetName};
use crate::domain::trade::{Trade, TradeCard, TradeCardDetail, TradeId, TradeStatus, TradeSummary};
use crate::domain::user::{CollectionVisibility, User, UserId, UserSuggestion};
use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardIdEntity {
    pub set_code: String,
    pub collector_number: String,
    pub language_code: String,
    pub set_name: String,
    pub scryfall_id: Uuid,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardImageLookupEntity {
    pub set_code: String,
    pub collector_number: String,
    pub language_code: String,
    pub name: String,
    pub scryfall_id: Uuid,
}

impl TryFrom<CardImageLookupEntity> for (CardId, CardImageLookup) {
    type Error = InfraError;

    fn try_from(entity: CardImageLookupEntity) -> Result<Self, InfraError> {
        let card_id = card_id_from_db(
            &entity.set_code,
            entity.collector_number,
            &entity.language_code,
        )?;
        let lookup = CardImageLookup {
            name: entity.name,
            scryfall_id: entity.scryfall_id,
        };
        Ok((card_id, lookup))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetNameEntity {
    pub set_code: String,
    pub name: String,
}

/// An unexpected value read from the database is corrupt data, not a caller mistake: it surfaces
/// as an `InfraError` (500), never as a panic nor as a functional (4xx) error.
pub(crate) fn invalid_db_value(what: &str, value: &str) -> InfraError {
    InfraError::RepositoryError(format!("invalid {what} from database: {value}"))
}

pub(crate) fn from_db_rarity(s: &str) -> Result<RarityCode, InfraError> {
    RarityCode::try_new(s).map_err(|_| invalid_db_value("rarity code", s))
}

fn card_id_from_db(
    set_code: &str,
    collector_number: String,
    language_code: &str,
) -> Result<CardId, InfraError> {
    Ok(CardId {
        set_code: SetCode::try_new(set_code).map_err(|_| invalid_db_value("set code", set_code))?,
        collector_number,
        language_code: LanguageCode::try_new(language_code)
            .map_err(|_| invalid_db_value("language code", language_code))?,
    })
}

fn rating_from_db(rating: Option<i16>) -> Result<Option<u8>, InfraError> {
    rating
        .map(|v| u8::try_from(v).map_err(|_| invalid_db_value("rating", &v.to_string())))
        .transpose()
}

/// Inverse of [`TradeStatus::as_db_str`].
impl TryFrom<&str> for TradeStatus {
    type Error = InfraError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "PENDING" => Ok(TradeStatus::Pending),
            "ONE_ACCEPTED" => Ok(TradeStatus::OneAccepted),
            "FULLY_ACCEPTED" => Ok(TradeStatus::FullyAccepted),
            "COMPLETED" => Ok(TradeStatus::Completed),
            "CLOSED" => Ok(TradeStatus::Closed),
            "ABANDONED" => Ok(TradeStatus::Abandoned),
            other => Err(invalid_db_value("trade status", other)),
        }
    }
}

/// Inverse of [`CollectionVisibility::as_db_str`].
impl TryFrom<&str> for CollectionVisibility {
    type Error = InfraError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "public" => Ok(CollectionVisibility::Public),
            "trade" => Ok(CollectionVisibility::Trade),
            "private" => Ok(CollectionVisibility::Private),
            other => Err(invalid_db_value("collection visibility", other)),
        }
    }
}

impl TryFrom<CardIdEntity> for CardId {
    type Error = InfraError;

    fn try_from(entity: CardIdEntity) -> Result<CardId, InfraError> {
        card_id_from_db(
            &entity.set_code,
            entity.collector_number,
            &entity.language_code,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserEntity {
    pub id: String,
    pub username: String,
    pub image_url: Option<String>,
}

impl From<UserEntity> for User {
    fn from(entity: UserEntity) -> User {
        User::new(entity.id, None, Some(entity.username), entity.image_url)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserSuggestionEntity {
    pub username: String,
    pub card_count: i64,
}

impl From<UserSuggestionEntity> for UserSuggestion {
    fn from(e: UserSuggestionEntity) -> Self {
        UserSuggestion {
            username: e.username,
            card_count: e.card_count as u64,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradeEntity {
    pub id: Uuid,
    pub initiator_user_id: String,
    pub respondent_user_id: String,
    pub status: String,
    pub initiator_amount_due: Option<i32>,
    pub respondent_amount_due: Option<i32>,
    pub initiator_accepted_at: Option<DateTime<Utc>>,
    pub respondent_accepted_at: Option<DateTime<Utc>>,
    pub initiator_confirmed_at: Option<DateTime<Utc>>,
    pub respondent_confirmed_at: Option<DateTime<Utc>>,
    pub initiator_rating: Option<i16>,
    pub respondent_rating: Option<i16>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<TradeEntity> for Trade {
    type Error = InfraError;

    fn try_from(entity: TradeEntity) -> Result<Trade, InfraError> {
        Ok(Trade {
            id: TradeId(entity.id),
            initiator_user_id: UserId::new(entity.initiator_user_id),
            respondent_user_id: UserId::new(entity.respondent_user_id),
            status: TradeStatus::try_from(entity.status.as_str())?,
            initiator_amount_due: entity.initiator_amount_due.map(|v| v as u32),
            respondent_amount_due: entity.respondent_amount_due.map(|v| v as u32),
            initiator_accepted_at: entity.initiator_accepted_at,
            respondent_accepted_at: entity.respondent_accepted_at,
            initiator_confirmed_at: entity.initiator_confirmed_at,
            respondent_confirmed_at: entity.respondent_confirmed_at,
            initiator_rating: rating_from_db(entity.initiator_rating)?,
            respondent_rating: rating_from_db(entity.respondent_rating)?,
            created_at: entity.created_at,
            updated_at: entity.updated_at,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradeCardEntity {
    pub set_code: String,
    pub collector_number: String,
    pub language_code: String,
    pub foil: bool,
    pub owner_user_id: String,
    pub quantity: i32,
}

impl TryFrom<TradeCardEntity> for TradeCard {
    type Error = InfraError;

    fn try_from(entity: TradeCardEntity) -> Result<TradeCard, InfraError> {
        Ok(TradeCard {
            card_id: CopyId {
                card_id: card_id_from_db(
                    &entity.set_code,
                    entity.collector_number,
                    &entity.language_code,
                )?,
                foil: entity.foil,
            },
            owner_user_id: UserId::new(entity.owner_user_id),
            quantity: entity.quantity as u32,
        })
    }
}

/// Row for [`TradeRepository::find_trade_cards_with_details`]. Price columns are flat (not
/// `#[sqlx(flatten)]`-ed into `PriceGuideEntity`) because this is populated via the compile-time
/// `sqlx::query_as!` macro, which maps columns by field name and doesn't support flatten.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradeCardDetailEntity {
    pub set_code: String,
    pub collector_number: String,
    pub language_code: String,
    pub foil: bool,
    pub owner_user_id: String,
    pub quantity: i32,
    pub name: String,
    pub scryfall_id: Uuid,
    pub the_gatherer_id: Option<String>,
    pub low: Option<i32>,
    pub avg: Option<i32>,
    pub trend: Option<i32>,
}

impl TryFrom<TradeCardDetailEntity> for TradeCardDetail {
    type Error = InfraError;

    fn try_from(entity: TradeCardDetailEntity) -> Result<TradeCardDetail, InfraError> {
        let price_guide = if entity.low.is_some() || entity.avg.is_some() || entity.trend.is_some()
        {
            Some(PriceGuide::from(PriceGuideEntity {
                low: entity.low,
                avg: entity.avg,
                trend: entity.trend,
            }))
        } else {
            None
        };

        Ok(TradeCardDetail {
            card_id: CopyId {
                card_id: card_id_from_db(
                    &entity.set_code,
                    entity.collector_number,
                    &entity.language_code,
                )?,
                foil: entity.foil,
            },
            owner_user_id: UserId::new(entity.owner_user_id),
            name: entity.name,
            quantity: entity.quantity as u32,
            price_guide,
            scryfall_id: entity.scryfall_id,
            the_gatherer_id: entity.the_gatherer_id,
        })
    }
}

/// Row for [`TradeRepository::list_trades`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradeSummaryEntity {
    pub id: Uuid,
    pub status: String,
    pub updated_at: DateTime<Utc>,
    pub partner_username: String,
    pub my_card_count: i64,
    pub partner_card_count: i64,
}

impl TryFrom<TradeSummaryEntity> for TradeSummary {
    type Error = InfraError;

    fn try_from(entity: TradeSummaryEntity) -> Result<TradeSummary, InfraError> {
        Ok(TradeSummary {
            id: TradeId(entity.id),
            status: TradeStatus::try_from(entity.status.as_str())?,
            partner_username: entity.partner_username,
            my_card_count: entity.my_card_count as u32,
            partner_card_count: entity.partner_card_count as u32,
            updated_at: entity.updated_at,
        })
    }
}

/// Flat price guide data as stored in the database (3 optional price fields).
#[derive(sqlx::FromRow, Clone, Debug, PartialEq, Eq)]
pub struct PriceGuideEntity {
    pub low: Option<i32>,
    pub avg: Option<i32>,
    pub trend: Option<i32>,
}

impl PriceGuideEntity {
    pub fn empty() -> Self {
        Self {
            low: None,
            avg: None,
            trend: None,
        }
    }
}

impl From<PriceGuideEntity> for PriceGuide {
    fn from(e: PriceGuideEntity) -> Self {
        PriceGuide {
            low: Price::from(e.low),
            avg: Price::from(e.avg),
            trend: Price::from(e.trend),
        }
    }
}

/// Raw sqlx row for `cardmarket_price` table — flat field names match DB columns exactly.
/// Use `CardMarketPriceEntity` (structured) outside of sqlx query context.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CardMarketPriceRaw {
    pub id_produit: i32,
    pub date: NaiveDate,
    pub low: Option<i32>,
    pub avg: Option<i32>,
    pub trend: Option<i32>,
    pub low_foil: Option<i32>,
    pub avg_foil: Option<i32>,
    pub trend_foil: Option<i32>,
}

impl From<CardMarketPriceRaw> for CardMarketPriceEntity {
    fn from(r: CardMarketPriceRaw) -> Self {
        CardMarketPriceEntity {
            id_produit: r.id_produit,
            date: r.date,
            normal: PriceGuideEntity {
                low: r.low,
                avg: r.avg,
                trend: r.trend,
            },
            foil: PriceGuideEntity {
                low: r.low_foil,
                avg: r.avg_foil,
                trend: r.trend_foil,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardMarketPriceEntity {
    pub id_produit: i32,
    pub date: NaiveDate,
    pub normal: PriceGuideEntity,
    pub foil: PriceGuideEntity,
}

impl From<CardMarketPriceEntity> for FullPriceGuide {
    fn from(e: CardMarketPriceEntity) -> Self {
        FullPriceGuide {
            id_product: e.id_produit as u32,
            normal: PriceGuide::from(e.normal),
            foil: PriceGuide::from(e.foil),
        }
    }
}

impl Price {
    pub fn as_i32(&self) -> Option<i32> {
        self.value.map(|v| v as i32)
    }
}

impl User {
    pub fn from_id(id: UserId) -> Self {
        User {
            id,
            name: None,
            username: None,
            avatar_url: None,
        }
    }
}

#[derive(sqlx::FromRow)]
pub struct CollectionPriceHistoryEntity {
    pub date: NaiveDate,
    pub low: i32,
    pub trend: i32,
    pub avg: i32,
}

impl From<CollectionPriceHistoryEntity> for PriceHistoryEntry {
    fn from(e: CollectionPriceHistoryEntity) -> Self {
        PriceHistoryEntry {
            date: e.date,
            price_guide: PriceGuide {
                low: e.low.into(),
                trend: e.trend.into(),
                avg: e.avg.into(),
            },
        }
    }
}

#[derive(sqlx::FromRow)]
pub struct CardMarketPriceHistoryEntity {
    pub date: NaiveDate,
    pub low: Option<i32>,
    pub trend: Option<i32>,
    pub avg: Option<i32>,
}

impl From<CardMarketPriceHistoryEntity> for PriceHistoryEntry {
    fn from(e: CardMarketPriceHistoryEntity) -> Self {
        PriceHistoryEntry {
            date: e.date,
            price_guide: PriceGuide {
                low: e.low.into(),
                trend: e.trend.into(),
                avg: e.avg.into(),
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CountEntity {
    pub count: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SizeEntity {
    pub size: Option<i64>,
}

#[derive(sqlx::FromRow)]
pub struct CardWithPriceEntity {
    pub set_code: String,
    pub set_name: String,
    pub collector_number: String,
    pub language_code: String,
    pub foil: bool,
    pub name: String,
    pub rarity: String,
    pub scryfall_id: Uuid,
    pub the_gatherer_id: Option<String>,
    /// Always present: no longer masked for other users' rows.
    pub quantity: i32,
    /// `NULL` when the row belongs to another user (masked in SQL).
    pub purchase_price: Option<i32>,
    /// `NULL` when the row belongs to another user (masked in SQL).
    pub added_at: Option<DateTime<Utc>>,
    /// Number of distinct users owning this card (search mode only); `0` and unused
    /// in "my collection" mode.
    pub owner_count: i64,
    /// `true` if this card is engaged in one of its owner's trades in `ONE_ACCEPTED` or
    /// `FULLY_ACCEPTED` status. In search mode, only accurate when scoped to a single real
    /// owner (`player_username`); `false` in the unscoped multi-owner aggregate.
    pub reserved: bool,
    #[sqlx(flatten)]
    pub price: PriceGuideEntity,
}

impl From<i32> for Price {
    fn from(value: i32) -> Self {
        Price::from_cents(value as u32)
    }
}

impl From<Option<i32>> for Price {
    fn from(value: Option<i32>) -> Self {
        value
            .map(|v| v as u32)
            .map(Price::from_cents)
            .unwrap_or_else(Price::empty)
    }
}

impl TryFrom<CardWithPriceEntity> for Card {
    type Error = InfraError;

    fn try_from(e: CardWithPriceEntity) -> Result<Self, Self::Error> {
        let price_guide = if e.price.avg.is_some() || e.price.low.is_some() {
            Some(PriceGuide::from(e.price))
        } else {
            None
        };

        let collection_entry = match (e.purchase_price, e.added_at) {
            (Some(purchase_price), Some(added_at)) => CollectionEntry::Mine {
                quantity: e.quantity as u8,
                purchase_price: purchase_price as u32,
                added_at,
                reserved: e.reserved,
            },
            _ => CollectionEntry::Public {
                owner_count: e.owner_count as u64,
                reserved: e.reserved,
            },
        };

        let card_id = card_id_from_db(&e.set_code, e.collector_number, &e.language_code)?;
        Ok(Card {
            set_name: SetName::new(card_id.set_code.clone(), e.set_name),
            id: CopyId {
                card_id,
                foil: e.foil,
            },
            name: e.name,
            rarity_code: from_db_rarity(&e.rarity)?,
            scryfall_id: e.scryfall_id,
            cardmarket_id: None,
            the_gatherer_id: e.the_gatherer_id,
            collection_entry,
            price_guide,
        })
    }
}

#[derive(sqlx::FromRow)]
pub struct CardOfferEntity {
    pub owner_username: String,
    pub quantity: i32,
    pub selling_price: Option<i32>,
    pub reserved: bool,
}

impl From<CardOfferEntity> for CollectionEntry {
    fn from(e: CardOfferEntity) -> Self {
        CollectionEntry::Owned {
            owner_username: e.owner_username,
            quantity: e.quantity as u8,
            selling_price: e.selling_price.map(|v| v as u32),
            reserved: e.reserved,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_card_id_entity() -> CardIdEntity {
        CardIdEntity {
            set_code: "FDN".to_string(),
            collector_number: "123".to_string(),
            language_code: "FR".to_string(),
            set_name: "Foundations".to_string(),
            scryfall_id: Uuid::parse_str("4409a063-bf2a-4a49-803e-3ce6bd474353").unwrap(),
        }
    }

    #[test]
    fn from_db_rarity_returns_common_for_c() {
        assert_eq!(from_db_rarity("C"), Ok(RarityCode::C));
    }

    #[test]
    fn from_db_rarity_returns_uncommon_for_u() {
        assert_eq!(from_db_rarity("U"), Ok(RarityCode::U));
    }

    #[test]
    fn from_db_rarity_returns_rare_for_r() {
        assert_eq!(from_db_rarity("R"), Ok(RarityCode::R));
    }

    #[test]
    fn from_db_rarity_returns_mythic_for_m() {
        assert_eq!(from_db_rarity("M"), Ok(RarityCode::M));
    }

    #[test]
    fn from_db_rarity_returns_special_for_s() {
        assert_eq!(from_db_rarity("S"), Ok(RarityCode::S));
    }

    #[test]
    fn from_db_rarity_returns_special_for_lowercase_s() {
        assert_eq!(from_db_rarity("s"), Ok(RarityCode::S));
    }

    #[test]
    fn from_db_rarity_returns_common_for_lowercase() {
        assert_eq!(from_db_rarity("c"), Ok(RarityCode::C));
    }

    #[test]
    fn from_db_rarity_returns_repository_error_on_unknown_code() {
        assert_eq!(
            from_db_rarity("X"),
            Err(InfraError::RepositoryError(
                "invalid rarity code from database: X".to_string()
            ))
        );
    }

    // --- TradeStatus / CollectionVisibility ↔ database strings ---

    #[test]
    fn trade_status_round_trips_through_db_str() {
        for status in [
            TradeStatus::Pending,
            TradeStatus::OneAccepted,
            TradeStatus::FullyAccepted,
            TradeStatus::Completed,
            TradeStatus::Closed,
            TradeStatus::Abandoned,
        ] {
            assert_eq!(TradeStatus::try_from(status.as_db_str()), Ok(status));
        }
    }

    #[test]
    fn trade_status_try_from_returns_repository_error_on_unknown_value() {
        assert_eq!(
            TradeStatus::try_from("UNKNOWN"),
            Err(InfraError::RepositoryError(
                "invalid trade status from database: UNKNOWN".to_string()
            ))
        );
    }

    #[test]
    fn collection_visibility_round_trips_through_db_str() {
        for visibility in [
            CollectionVisibility::Public,
            CollectionVisibility::Trade,
            CollectionVisibility::Private,
        ] {
            assert_eq!(
                CollectionVisibility::try_from(visibility.as_db_str()),
                Ok(visibility)
            );
        }
    }

    #[test]
    fn collection_visibility_try_from_returns_repository_error_on_unknown_value() {
        assert_eq!(
            CollectionVisibility::try_from("unknown"),
            Err(InfraError::RepositoryError(
                "invalid collection visibility from database: unknown".to_string()
            ))
        );
    }

    #[test]
    fn card_id_entity_converts_to_card_id() {
        let entity = make_card_id_entity();

        let card_id = CardId::try_from(entity).unwrap();

        assert_eq!(card_id.collector_number, "123");
        assert_eq!(card_id.language_code, LanguageCode::FR);
        assert_eq!(card_id.set_code.to_string(), "FDN");
    }

    #[test]
    fn price_as_i32_returns_some_when_value_is_present() {
        let price = Price { value: Some(199) };

        assert_eq!(price.as_i32(), Some(199));
    }

    #[test]
    fn price_as_i32_returns_none_when_value_is_absent() {
        let price = Price::empty();

        assert_eq!(price.as_i32(), None);
    }

    #[test]
    fn user_from_id_sets_id_correctly() {
        let user = User::from_id(UserId::new("abc123"));

        assert_eq!(user.id, UserId::new("abc123"));
        assert_eq!(user.name, None);
    }

    #[test]
    fn price_guide_entity_converts_to_price_guide() {
        let entity = PriceGuideEntity {
            low: Some(100),
            avg: Some(200),
            trend: Some(150),
        };

        let guide = PriceGuide::from(entity);

        assert_eq!(guide.low.value, Some(100));
        assert_eq!(guide.avg.value, Some(200));
        assert_eq!(guide.trend.value, Some(150));
    }

    #[test]
    fn price_guide_entity_empty_converts_to_empty_price_guide() {
        let entity = PriceGuideEntity::empty();

        let guide = PriceGuide::from(entity);

        assert_eq!(guide.low.value, None);
        assert_eq!(guide.avg.value, None);
        assert_eq!(guide.trend.value, None);
    }

    #[test]
    fn card_market_price_raw_converts_to_entity() {
        let raw = CardMarketPriceRaw {
            id_produit: 42,
            date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            low: Some(10),
            avg: Some(20),
            trend: Some(15),
            low_foil: Some(100),
            avg_foil: Some(200),
            trend_foil: Some(150),
        };

        let entity = CardMarketPriceEntity::from(raw);

        assert_eq!(entity.id_produit, 42);
        assert_eq!(entity.normal.low, Some(10));
        assert_eq!(entity.normal.avg, Some(20));
        assert_eq!(entity.foil.low, Some(100));
        assert_eq!(entity.foil.avg, Some(200));
    }

    // --- CardImageLookupEntity ---

    #[test]
    fn card_image_lookup_entity_converts_to_card_id_and_lookup() {
        let scryfall_id = Uuid::new_v4();
        let entity = CardImageLookupEntity {
            set_code: "FDN".to_string(),
            collector_number: "42".to_string(),
            language_code: "EN".to_string(),
            name: "Sol Ring".to_string(),
            scryfall_id,
        };

        let (card_id, lookup) = <(CardId, CardImageLookup)>::try_from(entity).unwrap();

        assert_eq!(card_id, CardId::new("FDN", "42", LanguageCode::EN));
        assert_eq!(
            lookup,
            CardImageLookup {
                name: "Sol Ring".to_string(),
                scryfall_id,
            }
        );
    }

    #[test]
    fn card_image_lookup_entity_with_unknown_language_fails_to_convert() {
        let entity = CardImageLookupEntity {
            set_code: "FDN".to_string(),
            collector_number: "42".to_string(),
            language_code: "XX".to_string(),
            name: "Sol Ring".to_string(),
            scryfall_id: Uuid::new_v4(),
        };

        assert!(<(CardId, CardImageLookup)>::try_from(entity).is_err());
    }

    // --- UserEntity ---

    #[test]
    fn user_entity_converts_to_user() {
        let entity = UserEntity {
            id: "user-123".to_string(),
            username: "alice".to_string(),
            image_url: Some("https://img.example.com/avatar.png".to_string()),
        };

        let user: User = entity.into();

        assert_eq!(user.id, UserId::new("user-123"));
        assert_eq!(user.username, Some("alice".to_string()));
        assert_eq!(
            user.avatar_url,
            Some("https://img.example.com/avatar.png".to_string())
        );
    }

    // --- TradeEntity ---

    #[test]
    fn trade_entity_converts_to_trade_with_all_fields() {
        let entity = TradeEntity {
            id: Uuid::new_v4(),
            initiator_user_id: "init-1".to_string(),
            respondent_user_id: "resp-1".to_string(),
            status: "PENDING".to_string(),
            initiator_amount_due: Some(500),
            respondent_amount_due: Some(300),
            initiator_accepted_at: Some(chrono::Utc::now()),
            respondent_accepted_at: None,
            initiator_confirmed_at: None,
            respondent_confirmed_at: None,
            initiator_rating: None,
            respondent_rating: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let trade = Trade::try_from(entity).unwrap();

        assert!(matches!(trade.status, TradeStatus::Pending));
        assert_eq!(trade.initiator_user_id, UserId::new("init-1"));
        assert_eq!(trade.respondent_user_id, UserId::new("resp-1"));
        assert_eq!(trade.initiator_amount_due, Some(500));
        assert_eq!(trade.respondent_amount_due, Some(300));
    }

    #[test]
    fn trade_entity_converts_to_trade_with_no_optional_fields() {
        let entity = TradeEntity {
            id: Uuid::new_v4(),
            initiator_user_id: "init-2".to_string(),
            respondent_user_id: "resp-2".to_string(),
            status: "CLOSED".to_string(),
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
        };

        let trade = Trade::try_from(entity).unwrap();

        assert!(matches!(trade.status, TradeStatus::Closed));
        assert!(trade.initiator_amount_due.is_none());
        assert!(trade.respondent_amount_due.is_none());
    }

    #[test]
    fn trade_entity_with_unknown_status_fails_to_convert() {
        let entity = TradeEntity {
            id: Uuid::new_v4(),
            initiator_user_id: "init-3".to_string(),
            respondent_user_id: "resp-3".to_string(),
            status: "BOGUS".to_string(),
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
        };

        assert_eq!(
            Trade::try_from(entity),
            Err(InfraError::RepositoryError(
                "invalid trade status from database: BOGUS".to_string()
            ))
        );
    }

    // --- TradeCardEntity ---

    #[test]
    fn trade_card_entity_converts_to_trade_card() {
        let entity = TradeCardEntity {
            set_code: "FDN".to_string(),
            collector_number: "87".to_string(),
            language_code: "FR".to_string(),
            foil: true,
            owner_user_id: "owner-1".to_string(),
            quantity: 3,
        };

        let trade_card = TradeCard::try_from(entity).unwrap();

        assert_eq!(trade_card.card_id.card_id.collector_number, "87");
        assert_eq!(trade_card.card_id.card_id.language_code, LanguageCode::FR);
        assert!(trade_card.card_id.foil);
        assert_eq!(trade_card.card_id.card_id.set_code.to_string(), "FDN");
        assert_eq!(trade_card.owner_user_id, UserId::new("owner-1"));
        assert_eq!(trade_card.quantity, 3);
    }

    #[test]
    fn card_id_from_db_returns_repository_error_on_unknown_language_code() {
        assert_eq!(
            card_id_from_db("FDN", "87".to_string(), "XX"),
            Err(InfraError::RepositoryError(
                "invalid language code from database: XX".to_string()
            ))
        );
    }

    #[test]
    fn card_id_from_db_returns_repository_error_on_invalid_set_code() {
        assert_eq!(
            card_id_from_db("F", "87".to_string(), "EN"),
            Err(InfraError::RepositoryError(
                "invalid set code from database: F".to_string()
            ))
        );
    }

    #[test]
    fn rating_from_db_returns_repository_error_on_out_of_range_value() {
        assert_eq!(rating_from_db(Some(4)), Ok(Some(4)));
        assert_eq!(rating_from_db(None), Ok(None));
        assert_eq!(
            rating_from_db(Some(-1)),
            Err(InfraError::RepositoryError(
                "invalid rating from database: -1".to_string()
            ))
        );
    }

    // --- CardWithPriceEntity ---

    #[test]
    fn card_with_price_entity_converts_to_mine_entry() {
        let entity = CardWithPriceEntity {
            set_code: "FDN".to_string(),
            set_name: "Foundations".to_string(),
            collector_number: "1".to_string(),
            language_code: "EN".to_string(),
            foil: false,
            name: "Sol Ring".to_string(),
            rarity: "C".to_string(),
            scryfall_id: Uuid::new_v4(),
            the_gatherer_id: None,
            quantity: 2,
            purchase_price: Some(350),
            added_at: Some(chrono::Utc::now()),
            owner_count: 0,
            reserved: true,
            price: PriceGuideEntity {
                low: Some(300),
                avg: Some(350),
                trend: None,
            },
        };

        let card = Card::try_from(entity).unwrap();

        assert_eq!(card.name, "Sol Ring");
        assert!(card.price_guide.is_some());
        match card.collection_entry {
            CollectionEntry::Mine { reserved, .. } => assert!(reserved),
            _ => panic!("expected CollectionEntry::Mine"),
        }
    }

    #[test]
    fn card_with_price_entity_converts_to_public_entry_when_purchase_price_is_null() {
        let entity = CardWithPriceEntity {
            set_code: "FDN".to_string(),
            set_name: "Foundations".to_string(),
            collector_number: "1".to_string(),
            language_code: "EN".to_string(),
            foil: false,
            name: "Sol Ring".to_string(),
            rarity: "C".to_string(),
            scryfall_id: Uuid::new_v4(),
            the_gatherer_id: None,
            quantity: 0,
            purchase_price: None,
            added_at: None,
            owner_count: 5,
            reserved: false,
            price: PriceGuideEntity {
                low: None,
                avg: None,
                trend: None,
            },
        };

        let card = Card::try_from(entity).unwrap();

        match card.collection_entry {
            CollectionEntry::Public { owner_count, .. } => {
                assert_eq!(owner_count, 5);
            }
            _ => panic!("expected CollectionEntry::Public"),
        }
    }

    #[test]
    fn card_with_price_entity_converts_to_public_entry_when_added_at_is_null() {
        let entity = CardWithPriceEntity {
            set_code: "FDN".to_string(),
            set_name: "Foundations".to_string(),
            collector_number: "1".to_string(),
            language_code: "EN".to_string(),
            foil: false,
            name: "Sol Ring".to_string(),
            rarity: "C".to_string(),
            scryfall_id: Uuid::new_v4(),
            the_gatherer_id: None,
            quantity: 0,
            purchase_price: Some(350),
            added_at: None,
            owner_count: 3,
            reserved: false,
            price: PriceGuideEntity {
                low: None,
                avg: None,
                trend: None,
            },
        };

        let card = Card::try_from(entity).unwrap();

        match card.collection_entry {
            CollectionEntry::Public { owner_count, .. } => {
                assert_eq!(owner_count, 3);
            }
            _ => panic!("expected CollectionEntry::Public"),
        }
    }

    #[test]
    fn card_with_price_entity_with_no_price_returns_none_price_guide() {
        let entity = CardWithPriceEntity {
            set_code: "FDN".to_string(),
            set_name: "Foundations".to_string(),
            collector_number: "1".to_string(),
            language_code: "EN".to_string(),
            foil: false,
            name: "Sol Ring".to_string(),
            rarity: "C".to_string(),
            scryfall_id: Uuid::new_v4(),
            the_gatherer_id: None,
            quantity: 1,
            purchase_price: Some(100),
            added_at: Some(chrono::Utc::now()),
            owner_count: 0,
            reserved: false,
            price: PriceGuideEntity {
                low: None,
                avg: None,
                trend: None,
            },
        };

        let card = Card::try_from(entity).unwrap();

        assert!(card.price_guide.is_none());
    }

    // --- CardMarketPriceEntity → FullPriceGuide ---

    #[test]
    fn card_market_price_entity_converts_to_full_price_guide() {
        let entity = CardMarketPriceEntity {
            id_produit: 42,
            date: NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            normal: PriceGuideEntity {
                low: Some(100),
                avg: Some(200),
                trend: Some(150),
            },
            foil: PriceGuideEntity {
                low: Some(1000),
                avg: Some(2000),
                trend: Some(1500),
            },
        };

        let full: FullPriceGuide = entity.into();

        assert_eq!(full.id_product, 42);
        assert_eq!(full.normal.low.value, Some(100));
        assert_eq!(full.normal.avg.value, Some(200));
        assert_eq!(full.normal.trend.value, Some(150));
        assert_eq!(full.foil.low.value, Some(1000));
        assert_eq!(full.foil.avg.value, Some(2000));
        assert_eq!(full.foil.trend.value, Some(1500));
    }

    // --- CollectionPriceHistoryEntity ---

    #[test]
    fn collection_price_history_entity_converts_to_price_history_entry() {
        let entity = CollectionPriceHistoryEntity {
            date: NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
            low: 100,
            trend: 200,
            avg: 150,
        };

        let entry: PriceHistoryEntry = entity.into();

        assert_eq!(entry.date, NaiveDate::from_ymd_opt(2025, 3, 1).unwrap());
        assert_eq!(entry.price_guide.low.value, Some(100));
        assert_eq!(entry.price_guide.trend.value, Some(200));
        assert_eq!(entry.price_guide.avg.value, Some(150));
    }

    // --- CardMarketPriceHistoryEntity ---

    #[test]
    fn card_market_price_history_entity_converts_to_price_history_entry() {
        let entity = CardMarketPriceHistoryEntity {
            date: NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
            low: Some(100),
            trend: None,
            avg: Some(150),
        };

        let entry: PriceHistoryEntry = entity.into();

        assert_eq!(entry.date, NaiveDate::from_ymd_opt(2025, 3, 1).unwrap());
        assert_eq!(entry.price_guide.low.value, Some(100));
        assert_eq!(entry.price_guide.trend.value, None);
        assert_eq!(entry.price_guide.avg.value, Some(150));
    }

    // --- CardOfferEntity ---

    #[test]
    fn card_offer_entity_converts_to_owned_entry() {
        let entity = CardOfferEntity {
            owner_username: "bob".to_string(),
            quantity: 5,
            selling_price: Some(2500),
            reserved: false,
        };

        let entry: CollectionEntry = entity.into();

        match entry {
            CollectionEntry::Owned {
                owner_username,
                quantity,
                selling_price,
                ..
            } => {
                assert_eq!(owner_username, "bob");
                assert_eq!(quantity, 5);
                assert_eq!(selling_price, Some(2500));
            }
            _ => panic!("expected CollectionEntry::Owned"),
        }
    }

    #[test]
    fn card_offer_entity_converts_to_owned_entry_with_no_selling_price() {
        let entity = CardOfferEntity {
            owner_username: "carol".to_string(),
            quantity: 1,
            selling_price: None,
            reserved: false,
        };

        let entry: CollectionEntry = entity.into();

        match entry {
            CollectionEntry::Owned {
                selling_price,
                quantity,
                ..
            } => {
                assert_eq!(quantity, 1);
                assert!(selling_price.is_none());
            }
            _ => panic!("expected CollectionEntry::Owned"),
        }
    }

    #[test]
    fn card_offer_entity_converts_reserved_flag() {
        let entity = CardOfferEntity {
            owner_username: "bob".to_string(),
            quantity: 1,
            selling_price: Some(100),
            reserved: true,
        };

        let entry: CollectionEntry = entity.into();

        match entry {
            CollectionEntry::Owned { reserved, .. } => assert!(reserved),
            _ => panic!("expected CollectionEntry::Owned"),
        }
    }

    // --- Price From<i32> ---

    #[test]
    fn price_from_i32_creates_price_with_cents() {
        let price: Price = 199i32.into();
        assert_eq!(price.value, Some(199));
        let price: Price = 0i32.into();
        assert_eq!(price.value, Some(0));
    }

    // --- Price From<Option<i32>> ---

    #[test]
    fn price_from_some_i32() {
        let price: Price = (Some(500) as Option<i32>).into();
        assert_eq!(price.value, Some(500));
    }

    #[test]
    fn price_from_none_i32_returns_empty() {
        let price: Price = (None as Option<i32>).into();
        assert_eq!(price.value, None);
    }
}
