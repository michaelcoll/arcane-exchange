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

/// What a Purchase bought.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PurchaseKind {
    SealedProduct {
        product_type: SealedProductType,
        name: Option<String>,
        set: SetName,
    },
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

#[cfg(test)]
mod tests {
    use super::*;

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
