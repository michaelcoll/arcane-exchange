use crate::domain::card::CardId;
use crate::domain::card_image::CardImage;
use crate::domain::error::FunctionalError;
use crate::domain::set_name::{SetCode, SetName};
use chrono::NaiveDate;
use std::fmt::{Display, Formatter};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PurchaseId(pub Uuid);

impl Display for PurchaseId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The amount of a Purchase, in cents: strictly positive. Signed on the way in so a zero or
/// negative amount fails with `invalid_purchase_price` (400) instead of a JSON extractor rejection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PurchaseAmount(u32);

impl PurchaseAmount {
    pub fn try_new(cents: i32) -> Result<Self, FunctionalError> {
        match u32::try_from(cents) {
            Ok(cents) if cents > 0 => Ok(Self(cents)),
            _ => Err(FunctionalError::InvalidPurchasePrice),
        }
    }

    pub fn cents(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SealedProductType {
    Booster,
    Display,
    Bundle,
    PrereleasePack,
    PreconstructedDeck,
}

impl SealedProductType {
    /// The value stored in `purchase.product_type`.
    pub fn as_str(self) -> &'static str {
        match self {
            SealedProductType::Booster => "BOOSTER",
            SealedProductType::Display => "DISPLAY",
            SealedProductType::Bundle => "BUNDLE",
            SealedProductType::PrereleasePack => "PRERELEASE_PACK",
            SealedProductType::PreconstructedDeck => "PRECONSTRUCTED_DECK",
        }
    }

    pub fn from_str_opt(value: &str) -> Option<Self> {
        match value {
            "BOOSTER" => Some(SealedProductType::Booster),
            "DISPLAY" => Some(SealedProductType::Display),
            "BUNDLE" => Some(SealedProductType::Bundle),
            "PRERELEASE_PACK" => Some(SealedProductType::PrereleasePack),
            "PRECONSTRUCTED_DECK" => Some(SealedProductType::PreconstructedDeck),
            _ => None,
        }
    }
}

/// The optional free-text name of a sealed product: trimmed, and absent when blank.
pub fn normalize_sealed_product_name(name: Option<&str>) -> Option<String> {
    name.map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
}

/// A card bought on its own, as a Purchase shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PurchasedCard {
    pub id: CardId,
    pub name: String,
    pub set: SetName,
    /// `None` while the card's image is pending.
    pub image: Option<CardImage>,
}

/// What a Purchase bought.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PurchaseKind {
    SealedProduct {
        product_type: SealedProductType,
        name: Option<String>,
        set: SetName,
    },
    /// The card, finish and quantity added with a purchase price.
    SingleCard {
        card: PurchasedCard,
        foil: bool,
        quantity: u32,
    },
}

/// The single card Purchase an addition with a purchase price records: the copies of the card
/// added, dated the day they are.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NewSingleCardPurchase {
    /// The purchase price of each copy.
    pub price_per_copy: PurchaseAmount,
    /// What the Purchase cost: the price of a copy times the copies.
    pub amount: PurchaseAmount,
    pub purchased_on: NaiveDate,
}

impl NewSingleCardPurchase {
    /// # Errors
    ///
    /// [`FunctionalError::InvalidPurchasePrice`] when the amount does not fit what a Purchase
    /// stores.
    pub fn try_new(
        price_per_copy: PurchaseAmount,
        quantity: u8,
        purchased_on: NaiveDate,
    ) -> Result<Self, FunctionalError> {
        let amount = i32::try_from(u64::from(price_per_copy.cents()) * u64::from(quantity))
            .map_err(|_| FunctionalError::InvalidPurchasePrice)
            .and_then(PurchaseAmount::try_new)?;

        Ok(Self {
            price_per_copy,
            amount,
            purchased_on,
        })
    }
}

/// What a player bought, on a given day.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Purchase {
    pub id: PurchaseId,
    pub kind: PurchaseKind,
    pub amount: PurchaseAmount,
    pub purchased_on: NaiveDate,
}

/// What a player asks to record as a sealed product, before its amount and name are checked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SealedProductRequest {
    pub product_type: SealedProductType,
    pub set_code: SetCode,
    pub name: Option<String>,
    pub amount_cents: i32,
    pub purchased_on: NaiveDate,
}

/// What a player asks to change on a Purchase, before its amount and name are checked: what is
/// `None` is kept.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct PurchaseEditRequest {
    pub amount_cents: Option<i32>,
    pub purchased_on: Option<NaiveDate>,
    pub product_type: Option<SealedProductType>,
    pub set_code: Option<SetCode>,
    /// An empty or blank name clears the stored one.
    pub name: Option<String>,
}

/// A sealed product to record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewSealedProduct {
    pub product_type: SealedProductType,
    pub set_code: SetCode,
    pub name: Option<String>,
    pub amount: PurchaseAmount,
    pub purchased_on: NaiveDate,
}

/// The fields of a Purchase an edit replaces: what is `None` is kept.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct PurchaseChanges {
    pub amount: Option<PurchaseAmount>,
    pub purchased_on: Option<NaiveDate>,
    pub product_type: Option<SealedProductType>,
    pub set_code: Option<SetCode>,
    /// `Some(None)` clears the name, `None` keeps it.
    pub name: Option<Option<String>>,
}

/// A line of the content of a Purchase: a card in a finish, with the quantity it brought.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PurchaseContentLine {
    pub card_id: CardId,
    pub name: String,
    pub image: Option<CardImage>,
    pub foil: bool,
    pub quantity: u32,
    /// The current Cardmarket trend price of the card in its finish, in cents, read when the
    /// Purchase is read and never stored. `None` when Cardmarket has none.
    pub trend_cents: Option<u32>,
}

/// A Purchase with what it contained.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PurchaseDetail {
    pub purchase: Purchase,
    pub content: Vec<PurchaseContentLine>,
}

impl PurchaseDetail {
    /// The sum of the quantities times the trend price of the lines that have one, in cents.
    pub fn content_value_cents(&self) -> u64 {
        self.content
            .iter()
            .filter_map(|line| {
                line.trend_cents
                    .map(|trend| u64::from(trend) * u64::from(line.quantity))
            })
            .sum()
    }

    /// How many lines have no trend price, so are left out of the value.
    pub fn lines_without_trend(&self) -> u32 {
        self.content
            .iter()
            .filter(|line| line.trend_cents.is_none())
            .count() as u32
    }

    /// The Profitability of the Purchase in cents: the value of the content minus the amount of
    /// the Purchase, positive when the content is worth more than was paid.
    pub fn profitability_cents(&self) -> i64 {
        self.content_value_cents() as i64 - i64::from(self.purchase.amount.cents())
    }

    /// [`Self::profitability_cents`] as a percentage of the amount of the Purchase.
    pub fn profitability_percent(&self) -> f64 {
        self.profitability_cents() as f64 * 100.0 / f64::from(self.purchase.amount.cents())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::language_code::LanguageCode;
    use uuid::Uuid;

    fn line(quantity: u32, trend_cents: Option<u32>) -> PurchaseContentLine {
        PurchaseContentLine {
            card_id: CardId::new("FDN", "1", LanguageCode::EN),
            name: "Card".to_string(),
            image: None,
            foil: false,
            quantity,
            trend_cents,
        }
    }

    fn detail(amount: i32, content: Vec<PurchaseContentLine>) -> PurchaseDetail {
        PurchaseDetail {
            purchase: Purchase {
                id: PurchaseId(Uuid::nil()),
                kind: PurchaseKind::SealedProduct {
                    product_type: SealedProductType::Booster,
                    name: None,
                    set: SetName {
                        code: SetCode::new("FDN"),
                        name: "Foundations".to_string(),
                    },
                },
                amount: PurchaseAmount::try_new(amount).unwrap(),
                purchased_on: NaiveDate::from_ymd_opt(2026, 11, 13).unwrap(),
            },
            content,
        }
    }

    #[test]
    fn value_sums_quantity_times_trend_and_skips_lines_without_trend() {
        let detail = detail(
            500,
            vec![line(2, Some(100)), line(1, Some(350)), line(3, None)],
        );

        assert_eq!(detail.content_value_cents(), 550);
        assert_eq!(detail.lines_without_trend(), 1);
        assert_eq!(detail.profitability_cents(), 50);
        assert_eq!(detail.profitability_percent(), 10.0);
    }

    #[test]
    fn profitability_is_negative_when_the_content_is_worth_less_than_the_amount() {
        let detail = detail(400, vec![line(1, Some(100))]);

        assert_eq!(detail.profitability_cents(), -300);
        assert_eq!(detail.profitability_percent(), -75.0);
    }

    #[test]
    fn an_empty_content_is_worth_nothing() {
        let detail = detail(400, vec![]);

        assert_eq!(detail.content_value_cents(), 0);
        assert_eq!(detail.lines_without_trend(), 0);
        assert_eq!(detail.profitability_cents(), -400);
        assert_eq!(detail.profitability_percent(), -100.0);
    }

    #[test]
    fn amount_must_be_strictly_positive() {
        for cents in [0, -1, i32::MIN] {
            assert_eq!(
                PurchaseAmount::try_new(cents),
                Err(FunctionalError::InvalidPurchasePrice)
            );
        }
        assert_eq!(PurchaseAmount::try_new(1).unwrap().cents(), 1);
        assert_eq!(
            PurchaseAmount::try_new(i32::MAX).unwrap().cents(),
            i32::MAX as u32
        );
    }

    #[test]
    fn a_single_card_purchase_costs_the_price_times_the_quantity() {
        let day = NaiveDate::from_ymd_opt(2026, 11, 13).unwrap();

        let price = PurchaseAmount::try_new(200).unwrap();

        let purchase = NewSingleCardPurchase::try_new(price, 4, day).unwrap();

        assert_eq!(purchase.price_per_copy.cents(), 200);
        assert_eq!(purchase.amount.cents(), 800);
        assert_eq!(purchase.purchased_on, day);
    }

    #[test]
    fn a_single_card_amount_over_what_a_purchase_stores_is_refused() {
        let day = NaiveDate::from_ymd_opt(2026, 11, 13).unwrap();
        let price = PurchaseAmount::try_new(i32::MAX).unwrap();

        assert_eq!(
            NewSingleCardPurchase::try_new(price, 2, day),
            Err(FunctionalError::InvalidPurchasePrice)
        );
    }

    #[test]
    fn blank_name_is_absent_and_the_others_are_trimmed() {
        assert_eq!(normalize_sealed_product_name(None), None);
        assert_eq!(normalize_sealed_product_name(Some("")), None);
        assert_eq!(normalize_sealed_product_name(Some(" \t ")), None);
        assert_eq!(
            normalize_sealed_product_name(Some("  Deck Bleu ")),
            Some("Deck Bleu".to_string())
        );
    }

    #[test]
    fn product_type_round_trips_through_its_stored_value() {
        for product_type in [
            SealedProductType::Booster,
            SealedProductType::Display,
            SealedProductType::Bundle,
            SealedProductType::PrereleasePack,
            SealedProductType::PreconstructedDeck,
        ] {
            assert_eq!(
                SealedProductType::from_str_opt(product_type.as_str()),
                Some(product_type)
            );
        }
        assert_eq!(SealedProductType::from_str_opt("BOX"), None);
    }
}
