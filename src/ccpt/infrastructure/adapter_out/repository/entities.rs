use crate::domain::card::{Card, CardId, CollectionEntry};
use crate::domain::language_code::LanguageCode;
use crate::domain::price::{FullPriceGuide, Price, PriceGuide, PriceHistoryEntry};
use crate::domain::rarity_code::RarityCode;
use crate::domain::set_name::{SetCode, SetName};
use crate::domain::trade::{Trade, TradeCard, TradeId, TradeStatus};
use crate::domain::user::{User, UserId};
use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardEntity {
    pub set_code: String,
    pub collector_number: String,
    pub language_code: String,
    pub foil: bool,
    pub set_name: String,
    pub name: String,
    pub rarity: String,
    pub quantity: i32,
    /// Price in cents
    pub purchase_price: i32,
    pub added_at: Option<DateTime<Utc>>,
    pub scryfall_id: Uuid,
    pub cardmarket_id: Option<i32>,
    pub the_gatherer_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardIdEntity {
    pub set_code: String,
    pub collector_number: String,
    pub language_code: String,
    pub foil: bool,
    pub set_name: String,
    pub scryfall_id: Uuid,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardNameEntity {
    pub set_code: String,
    pub collector_number: String,
    pub language_code: String,
    pub foil: bool,
    pub name: String,
}

impl From<CardNameEntity> for CardId {
    fn from(entity: CardNameEntity) -> CardId {
        let set_code =
            SetCode::try_new(entity.set_code).expect("database contains invalid set_code");
        CardId {
            set_code,
            collector_number: entity.collector_number,
            language_code: LanguageCode::try_new(entity.language_code)
                .expect("database contains invalid language_code"),
            foil: entity.foil,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetNameEntity {
    pub set_code: String,
    pub name: String,
}

impl From<CardEntity> for Card {
    fn from(entity: CardEntity) -> Card {
        let set_code =
            SetCode::try_new(entity.set_code).expect("database contains invalid set_code");
        Card {
            id: CardId {
                set_code: set_code.clone(),
                collector_number: entity.collector_number,
                language_code: LanguageCode::try_new(entity.language_code)
                    .expect("database contains invalid language_code"),
                foil: entity.foil,
            },
            set_name: SetName {
                code: set_code.clone(),
                name: entity.set_name,
            },
            name: entity.name,
            rarity_code: from_db_rarity(entity.rarity),
            collection_entry: CollectionEntry::Mine {
                quantity: entity.quantity as u8,
                purchase_price: entity.purchase_price as u32,
                added_at: entity.added_at.expect(
                    "collection_entry.added_at should always be set (ManaBox import guarantee)",
                ),
            },
            scryfall_id: entity.scryfall_id,
            cardmarket_id: entity.cardmarket_id.map(|id| id as u32),
            the_gatherer_id: entity.the_gatherer_id,
            price_guide: None,
        }
    }
}

fn from_db_rarity<S: AsRef<str>>(s: S) -> RarityCode {
    let s = s.as_ref().to_uppercase();
    match s.as_str() {
        "C" | "c" => RarityCode::C,
        "U" | "u" => RarityCode::U,
        "R" | "r" => RarityCode::R,
        "M" | "m" => RarityCode::M,
        _ => panic!("invalid rarity code from database: {}", s),
    }
}

impl From<CardIdEntity> for CardId {
    fn from(entity: CardIdEntity) -> CardId {
        let set_code =
            SetCode::try_new(entity.set_code).expect("database contains invalid set_code");
        CardId {
            set_code: set_code.clone(),
            collector_number: entity.collector_number,
            language_code: LanguageCode::try_new(entity.language_code)
                .expect("database contains invalid language_code"),
            foil: entity.foil,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserEntity {
    pub id: String,
    pub username: String,
}

impl From<UserEntity> for User {
    fn from(entity: UserEntity) -> User {
        User::new(entity.id, None, Some(entity.username))
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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<TradeEntity> for Trade {
    fn from(entity: TradeEntity) -> Trade {
        Trade {
            id: TradeId(entity.id),
            initiator_user_id: UserId::new(entity.initiator_user_id),
            respondent_user_id: UserId::new(entity.respondent_user_id),
            status: TradeStatus::from_db_str(&entity.status),
            initiator_amount_due: entity.initiator_amount_due.map(|v| v as u32),
            respondent_amount_due: entity.respondent_amount_due.map(|v| v as u32),
            initiator_accepted_at: entity.initiator_accepted_at,
            respondent_accepted_at: entity.respondent_accepted_at,
            created_at: entity.created_at,
            updated_at: entity.updated_at,
        }
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

impl From<TradeCardEntity> for TradeCard {
    fn from(entity: TradeCardEntity) -> TradeCard {
        let set_code =
            SetCode::try_new(entity.set_code).expect("database contains invalid set_code");
        TradeCard {
            card_id: CardId {
                set_code,
                collector_number: entity.collector_number,
                language_code: LanguageCode::try_new(entity.language_code)
                    .expect("database contains invalid language_code"),
                foil: entity.foil,
            },
            owner_user_id: UserId::new(entity.owner_user_id),
            quantity: entity.quantity as u32,
        }
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
    /// Username of the owner, `NULL` when the row is mine.
    pub owner_username: Option<String>,
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

impl From<CardWithPriceEntity> for Card {
    fn from(e: CardWithPriceEntity) -> Self {
        let selling_price = e.price.trend.map(|v| v as u32);

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
            },
            _ => CollectionEntry::Owned {
                owner_username: e.owner_username.unwrap_or_default(),
                quantity: e.quantity as u8,
                selling_price,
            },
        };

        let set_code = SetCode::try_new(&e.set_code).expect("database contains invalid set_code");
        Card {
            id: CardId::new(
                set_code.clone(),
                e.collector_number,
                LanguageCode::try_new(&e.language_code)
                    .expect("database contains invalid language_code"),
                e.foil,
            ),
            set_name: SetName::new(set_code, e.set_name),
            name: e.name,
            rarity_code: from_db_rarity(e.rarity),
            scryfall_id: e.scryfall_id,
            cardmarket_id: None,
            the_gatherer_id: e.the_gatherer_id,
            collection_entry,
            price_guide,
        }
    }
}

#[derive(sqlx::FromRow)]
pub struct CardOfferEntity {
    pub owner_username: String,
    pub quantity: i32,
    pub selling_price: Option<i32>,
}

impl From<CardOfferEntity> for CollectionEntry {
    fn from(e: CardOfferEntity) -> Self {
        CollectionEntry::Owned {
            owner_username: e.owner_username,
            quantity: e.quantity as u8,
            selling_price: e.selling_price.map(|v| v as u32),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_card_entity(rarity: &str, foil: bool, cardmarket_id: Option<i32>) -> CardEntity {
        CardEntity {
            set_code: "FDN".to_string(),
            collector_number: "123".to_string(),
            language_code: "EN".to_string(),
            foil,
            set_name: "Foundations".to_string(),
            name: "Goblin Guide".to_string(),
            rarity: rarity.to_string(),
            quantity: 2,
            purchase_price: 350,
            scryfall_id: Uuid::parse_str("4409a063-bf2a-4a49-803e-3ce6bd474353").unwrap(),
            cardmarket_id,
            the_gatherer_id: None,
            added_at: Some(chrono::Utc::now()),
        }
    }

    fn make_card_id_entity(foil: bool) -> CardIdEntity {
        CardIdEntity {
            set_code: "FDN".to_string(),
            collector_number: "123".to_string(),
            language_code: "FR".to_string(),
            foil,
            set_name: "Foundations".to_string(),
            scryfall_id: Uuid::parse_str("4409a063-bf2a-4a49-803e-3ce6bd474353").unwrap(),
        }
    }

    #[test]
    fn card_entity_converts_to_card_with_all_fields() {
        let entity = make_card_entity("R", false, Some(42));

        let card: Card = entity.into();

        assert_eq!(card.id.collector_number, "123");
        assert_eq!(card.id.language_code, LanguageCode::EN);
        assert!(!card.id.foil);
        assert_eq!(card.name, "Goblin Guide");
        assert_eq!(card.set_name.name, "Foundations");
        assert_eq!(card.rarity_code, RarityCode::R);
        match &card.collection_entry {
            CollectionEntry::Mine {
                quantity,
                purchase_price,
                ..
            } => {
                assert_eq!(*quantity, 2);
                assert_eq!(*purchase_price, 350);
            }
            CollectionEntry::Owned { .. } => panic!("expected CollectionEntry::Mine"),
        }
        assert_eq!(card.cardmarket_id, Some(42));
    }

    #[test]
    fn card_entity_converts_to_card_without_cardmarket_id() {
        let entity = make_card_entity("C", false, None);

        let card: Card = entity.into();

        assert_eq!(card.cardmarket_id, None);
    }

    #[test]
    fn card_entity_converts_to_card_with_foil_flag() {
        let entity = make_card_entity("U", true, None);

        let card: Card = entity.into();

        assert!(card.id.foil);
    }

    #[test]
    fn card_entity_set_code_is_uppercased_in_card_id_and_set_name() {
        let entity = make_card_entity("M", false, None);

        let card: Card = entity.into();

        assert_eq!(card.id.set_code.to_string(), "FDN");
        assert_eq!(card.set_name.code.to_string(), "FDN");
    }

    #[test]
    fn card_entity_purchase_price_in_cents_is_preserved() {
        let entity = make_card_entity("C", false, None);

        let card: Card = entity.into();

        match card.collection_entry {
            CollectionEntry::Mine { purchase_price, .. } => assert_eq!(purchase_price, 350),
            CollectionEntry::Owned { .. } => panic!("expected CollectionEntry::Mine"),
        }
    }

    #[test]
    fn from_db_rarity_returns_common_for_c() {
        assert_eq!(from_db_rarity("C"), RarityCode::C);
    }

    #[test]
    fn from_db_rarity_returns_uncommon_for_u() {
        assert_eq!(from_db_rarity("U"), RarityCode::U);
    }

    #[test]
    fn from_db_rarity_returns_rare_for_r() {
        assert_eq!(from_db_rarity("R"), RarityCode::R);
    }

    #[test]
    fn from_db_rarity_returns_mythic_for_m() {
        assert_eq!(from_db_rarity("M"), RarityCode::M);
    }

    #[test]
    fn from_db_rarity_returns_common_for_lowercase() {
        assert_eq!(from_db_rarity("c"), RarityCode::C);
    }

    #[test]
    #[should_panic(expected = "invalid rarity code from database")]
    fn from_db_rarity_panics_on_unknown_code() {
        from_db_rarity("X");
    }

    #[test]
    fn card_id_entity_converts_to_card_id_with_foil() {
        let entity = make_card_id_entity(true);

        let card_id: CardId = entity.into();

        assert_eq!(card_id.collector_number, "123");
        assert_eq!(card_id.language_code, LanguageCode::FR);
        assert!(card_id.foil);
        assert_eq!(card_id.set_code.to_string(), "FDN");
    }

    #[test]
    fn card_id_entity_converts_to_card_id_without_foil() {
        let entity = make_card_id_entity(false);

        let card_id: CardId = entity.into();

        assert!(!card_id.foil);
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
    fn price_as_i32_returns_zero_when_value_is_zero() {
        let price = Price { value: Some(0) };

        assert_eq!(price.as_i32(), Some(0));
    }

    #[test]
    fn user_from_id_sets_id_correctly() {
        let user = User::from_id(UserId::new("abc123"));

        assert_eq!(user.id, UserId::new("abc123"));
    }

    #[test]
    fn user_from_id_has_no_name() {
        let user = User::from_id(UserId::new("abc123"));

        assert_eq!(user.name, None);
    }

    #[test]
    fn user_from_id_with_complex_id() {
        let user = User::from_id(UserId::new("google|105262637836230123456"));

        assert_eq!(user.id, UserId::new("google|105262637836230123456"));
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
}
