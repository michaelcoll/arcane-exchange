use crate::domain::binder::BinderId;
use crate::domain::error::FunctionalError;
use chrono::{DateTime, Utc};
use std::fmt::{Display, Formatter};
use uuid::Uuid;

/// The most copies a Collection entry holds: the width of the quantity the API exposes.
pub const MAX_ENTRY_QUANTITY: u32 = 255;

/// Designates a Collection entry in the API.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CollectionEntryId(pub Uuid);

impl Display for CollectionEntryId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for CollectionEntryId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

/// The binder an entry is stored in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntryBinder {
    pub id: BinderId,
    pub name: String,
}

/// A Collection entry of one player, as the API shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollectionEntryDetail {
    pub id: CollectionEntryId,
    pub foil: bool,
    /// `None` for an entry stored in no binder.
    pub binder: Option<EntryBinder>,
    pub quantity: u8,
    /// Price in cents; `None` when the copies have no purchase price (ADR-0023).
    pub purchase_price: Option<u32>,
    pub added_at: DateTime<Utc>,
}

/// What a player owns of one Card, in both finishes, with what Trades have committed of it.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct CardEntries {
    pub entries: Vec<CollectionEntryDetail>,
    /// Copies committed by the accepted Trades, normal finish (a Carte réservée).
    pub reserved_normal: u32,
    /// Copies committed by the accepted Trades, foil finish.
    pub reserved_foil: u32,
}

/// The copies the accepted Trades commit of a card, per finish (Carte réservée). Read, never
/// stored (ADR-0008).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ReservedQuantities {
    pub normal: u32,
    pub foil: u32,
}

impl ReservedQuantities {
    /// What is committed in the finish.
    pub fn of(&self, foil: bool) -> u32 {
        if foil { self.foil } else { self.normal }
    }
}

/// An update of a Collection entry: a new quantity, a new finish, or both.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct EntryUpdate {
    pub quantity: Option<u8>,
    pub foil: Option<bool>,
}

/// Checks a requested quantity against the bounds of an entry, 1 to 255. A quantity of 0 is not
/// an edit: the entry is removed instead.
///
/// # Errors
///
/// [`FunctionalError::InvalidQuantity`] when `quantity` is out of bounds.
pub fn validate_quantity(quantity: i32) -> Result<u8, FunctionalError> {
    u8::try_from(quantity)
        .ok()
        .filter(|quantity| *quantity >= 1)
        .ok_or(FunctionalError::InvalidQuantity)
}

/// What two entries of the same copy share once merged into one: the copies they hold, with
/// their purchase price and added date.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntryCopies {
    pub quantity: u32,
    pub purchase_price: Option<u32>,
    pub added_at: DateTime<Utc>,
}

impl EntryCopies {
    /// The single rule by which two entries become one (a copy added to an existing entry, moved
    /// into one, or turned into another finish):
    ///
    /// - the quantities add up;
    /// - the purchase price is the average of the prices present weighted by their quantities,
    ///   absent when neither has one. The average gives a price to the copies that had none: an
    ///   assumed approximation, the exact spending lives in the Purchases;
    /// - the added date is the earliest.
    ///
    /// # Errors
    ///
    /// [`FunctionalError::InvalidQuantity`] when the sum goes over [`MAX_ENTRY_QUANTITY`].
    pub fn merge(&self, other: &EntryCopies) -> Result<EntryCopies, FunctionalError> {
        if self.quantity + other.quantity > MAX_ENTRY_QUANTITY {
            return Err(FunctionalError::InvalidQuantity);
        }
        Ok(self.merge_saturating(other))
    }

    /// [`Self::merge`] for the Import, which keeps what it can instead of refusing: a sum over
    /// [`MAX_ENTRY_QUANTITY`] is capped to it.
    pub fn merge_saturating(&self, other: &EntryCopies) -> EntryCopies {
        let quantity = (self.quantity + other.quantity).min(MAX_ENTRY_QUANTITY);

        let (weighted_sum, priced_quantity) = [self, other]
            .into_iter()
            .filter_map(|copies| {
                copies.purchase_price.map(|price| {
                    (
                        u64::from(price) * u64::from(copies.quantity),
                        copies.quantity,
                    )
                })
            })
            .fold((0u64, 0u32), |(sum, quantity), (price, q)| {
                (sum + price, quantity + q)
            });
        // An average of `u32` prices fits a `u32`: the fallback is never reached.
        let purchase_price = (priced_quantity > 0)
            .then(|| u32::try_from(weighted_sum / u64::from(priced_quantity)).unwrap_or(u32::MAX));

        EntryCopies {
            quantity,
            purchase_price,
            added_at: self.added_at.min(other.added_at),
        }
    }
}

/// The reservation guard: the copies of a card and finish a player keeps, all binders together,
/// must cover what the accepted Trades have committed of them.
///
/// # Errors
///
/// [`FunctionalError::ReservedQuantityExceeded`] when `owned_after` is under `reserved`.
pub fn ensure_covers_reserved(owned_after: u32, reserved: u32) -> Result<(), FunctionalError> {
    if owned_after < reserved {
        return Err(FunctionalError::ReservedQuantityExceeded);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn copies(quantity: u32, purchase_price: Option<u32>, day: u32) -> EntryCopies {
        EntryCopies {
            quantity,
            purchase_price,
            added_at: Utc.with_ymd_and_hms(2026, 1, day, 0, 0, 0).unwrap(),
        }
    }

    #[test]
    fn merge_adds_the_quantities() {
        let merged = copies(2, None, 1).merge(&copies(3, None, 1)).unwrap();

        assert_eq!(merged.quantity, 5);
    }

    #[test]
    fn merge_averages_the_prices_weighted_by_quantity() {
        // (2 * 1000 + 1 * 400) / 3 = 800
        let merged = copies(2, Some(1000), 1)
            .merge(&copies(1, Some(400), 1))
            .unwrap();

        assert_eq!(merged.purchase_price, Some(800));
    }

    #[test]
    fn merge_ignores_the_absent_price_in_the_average() {
        let merged = copies(1, Some(1000), 1).merge(&copies(4, None, 1)).unwrap();

        assert_eq!(merged.purchase_price, Some(1000));
        assert_eq!(merged.quantity, 5);
    }

    #[test]
    fn merge_has_no_price_when_neither_has_one() {
        let merged = copies(1, None, 1).merge(&copies(1, None, 1)).unwrap();

        assert_eq!(merged.purchase_price, None);
    }

    #[test]
    fn merge_keeps_the_earliest_added_date() {
        let merged = copies(1, None, 20).merge(&copies(1, None, 3)).unwrap();

        assert_eq!(merged.added_at, copies(1, None, 3).added_at);
    }

    #[test]
    fn merge_refuses_a_sum_over_the_bound() {
        assert_eq!(
            copies(200, None, 1).merge(&copies(56, None, 1)),
            Err(FunctionalError::InvalidQuantity)
        );
        assert!(copies(200, None, 1).merge(&copies(55, None, 1)).is_ok());
    }

    #[test]
    fn merge_saturating_caps_the_quantity_and_keeps_the_rest_of_the_rule() {
        let merged = copies(200, Some(1000), 20).merge_saturating(&copies(100, Some(400), 3));

        assert_eq!(merged.quantity, MAX_ENTRY_QUANTITY);
        // (200 * 1000 + 100 * 400) / 300 = 800
        assert_eq!(merged.purchase_price, Some(800));
        assert_eq!(merged.added_at, copies(1, None, 3).added_at);
    }

    #[test]
    fn quantity_is_bounded_from_1_to_255() {
        assert_eq!(validate_quantity(1), Ok(1));
        assert_eq!(validate_quantity(255), Ok(255));
        for out_of_bounds in [-1, 0, 256, i32::MAX] {
            assert_eq!(
                validate_quantity(out_of_bounds),
                Err(FunctionalError::InvalidQuantity)
            );
        }
    }

    #[test]
    fn the_guard_accepts_exactly_the_reserved_quantity() {
        assert_eq!(ensure_covers_reserved(2, 2), Ok(()));
        assert_eq!(
            ensure_covers_reserved(1, 2),
            Err(FunctionalError::ReservedQuantityExceeded)
        );
    }
}
