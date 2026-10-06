use crate::domain::card::Card;

/// A card parsed from a ManaBox import, together with the binder it was found in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedCard {
    pub card: Card,
    /// Value of the CSV's `Binder Name` column, `None` if absent or blank.
    pub binder_name: Option<String>,
}
