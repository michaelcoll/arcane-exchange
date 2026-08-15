use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub String);

impl UserId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for UserId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for UserId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

impl From<&str> for UserId {
    fn from(id: &str) -> Self {
        Self(id.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub name: Option<String>,
    pub username: Option<String>,
}

impl User {
    pub fn new(id: impl Into<UserId>, name: Option<String>, username: Option<String>) -> Self {
        Self {
            id: id.into(),
            name,
            username,
        }
    }

    #[cfg(test)]
    pub fn for_testing() -> Self {
        Self {
            id: UserId::new("test-user-id"),
            name: Some("Test User".to_string()),
            username: Some("testuser".to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserSuggestion {
    pub username: String,
    pub card_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectionVisibility {
    Public,
    Trade,
    Private,
}

impl CollectionVisibility {
    pub fn as_db_str(&self) -> &'static str {
        match self {
            CollectionVisibility::Public => "public",
            CollectionVisibility::Trade => "trade",
            CollectionVisibility::Private => "private",
        }
    }

    pub fn from_db_str(s: &str) -> Self {
        match s {
            "public" => CollectionVisibility::Public,
            "trade" => CollectionVisibility::Trade,
            "private" => CollectionVisibility::Private,
            _ => panic!("invalid collection visibility from database: {}", s),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_creation_with_full_info() {
        let user = User::new(
            "123456".to_string(),
            Some("Test User".to_string()),
            Some("testuser".to_string()),
        );

        assert_eq!(user.id, UserId::new("123456"));
        assert_eq!(user.name, Some("Test User".to_string()));
        assert_eq!(user.username, Some("testuser".to_string()));
    }

    #[test]
    fn user_creation_without_name() {
        let user = User::new("789".to_string(), None, None);

        assert_eq!(user.id, UserId::new("789"));
        assert_eq!(user.name, None);
        assert_eq!(user.username, None);
    }

    #[test]
    fn user_clone_works_correctly() {
        let user1 = User::new(
            "456".to_string(),
            Some("Cloned User".to_string()),
            Some("cloneduser".to_string()),
        );

        let user2 = user1.clone();

        assert_eq!(user1, user2);
    }

    #[test]
    fn collection_visibility_round_trips_through_db_str() {
        for variant in [
            CollectionVisibility::Public,
            CollectionVisibility::Trade,
            CollectionVisibility::Private,
        ] {
            assert_eq!(
                CollectionVisibility::from_db_str(variant.as_db_str()),
                variant
            );
        }
    }

    #[test]
    #[should_panic(expected = "invalid collection visibility from database: unknown")]
    fn collection_visibility_from_db_str_panics_on_unknown_value() {
        CollectionVisibility::from_db_str("unknown");
    }
}
