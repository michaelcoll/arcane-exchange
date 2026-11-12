use crate::domain::error::FunctionalError;
use std::fmt::{Display, Formatter};
use uuid::Uuid;

/// Longest binder name, in characters: the width of the `binder.name` column.
pub const MAX_BINDER_NAME_LENGTH: usize = 255;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BinderId(pub Uuid);

impl Display for BinderId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for BinderId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

/// A binder name: trimmed, not empty, at most [`MAX_BINDER_NAME_LENGTH`] characters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BinderName(String);

impl BinderName {
    pub fn try_new(name: &str) -> Result<Self, FunctionalError> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > MAX_BINDER_NAME_LENGTH {
            return Err(FunctionalError::InvalidBinderName);
        }
        Ok(Self(name.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The binder a player chose to receive the cards of their Trades into.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReceivingBinder {
    pub id: BinderId,
    pub name: String,
}

/// A binder of a player, with what the list of their binders shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binder {
    pub id: BinderId,
    pub name: String,
    /// Sum of the quantities of the entries stored in the binder, 0 for an empty one.
    pub copies: u64,
    pub is_open: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_is_trimmed() {
        assert_eq!(BinderName::try_new("  Trade  ").unwrap().as_str(), "Trade");
    }

    #[test]
    fn blank_name_is_refused() {
        for name in ["", "   ", "\t\n"] {
            assert_eq!(
                BinderName::try_new(name),
                Err(FunctionalError::InvalidBinderName)
            );
        }
    }

    #[test]
    fn name_is_capped_in_characters() {
        assert!(BinderName::try_new(&"é".repeat(MAX_BINDER_NAME_LENGTH)).is_ok());
        assert_eq!(
            BinderName::try_new(&"é".repeat(MAX_BINDER_NAME_LENGTH + 1)),
            Err(FunctionalError::InvalidBinderName)
        );
    }
}
