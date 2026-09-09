use crate::domain::error::FunctionalError;
use crate::domain::user::UserId;
use chrono::{DateTime, Utc};
use std::fmt::{Display, Formatter};
use uuid::Uuid;

/// Maximum number of line errors persisted on a single import. Beyond this, only
/// `line_error_count` keeps growing so a fully-invalid CSV cannot blow up the response size.
pub const MAX_STORED_LINE_ERRORS: usize = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CardImportId(pub Uuid);

impl CardImportId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for CardImportId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for CardImportId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for CardImportId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardImportStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

impl CardImportStatus {
    pub fn try_new<S: AsRef<str>>(s: S) -> Result<Self, FunctionalError> {
        match s.as_ref() {
            "pending" => Ok(CardImportStatus::Pending),
            "running" => Ok(CardImportStatus::Running),
            "completed" => Ok(CardImportStatus::Completed),
            "failed" => Ok(CardImportStatus::Failed),
            other => Err(FunctionalError::InvalidCardImportStatus(other.to_string())),
        }
    }
}

impl Display for CardImportStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CardImportStatus::Pending => write!(f, "pending"),
            CardImportStatus::Running => write!(f, "running"),
            CardImportStatus::Completed => write!(f, "completed"),
            CardImportStatus::Failed => write!(f, "failed"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardImportLineError {
    pub line: usize,
    pub field: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CardImport {
    pub id: CardImportId,
    pub user_id: UserId,
    pub status: CardImportStatus,
    pub source_lines: u32,
    pub total_lines: u32,
    pub processed_lines: u32,
    pub line_errors: Vec<CardImportLineError>,
    pub line_error_count: u32,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_new_round_trips_every_status_through_display() {
        for status in [
            CardImportStatus::Pending,
            CardImportStatus::Running,
            CardImportStatus::Completed,
            CardImportStatus::Failed,
        ] {
            let rendered = status.to_string();
            assert_eq!(CardImportStatus::try_new(&rendered), Ok(status));
        }
    }

    #[test]
    fn try_new_rejects_unknown_status() {
        assert!(CardImportStatus::try_new("bogus").is_err());
    }
}
