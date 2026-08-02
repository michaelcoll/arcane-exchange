use crate::domain::user::UserSuggestion;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

#[derive(Deserialize, Debug, TS)]
#[ts(export, export_to = "AutocompleteUserParams.ts")]
pub(crate) struct AutocompleteUserParams {
    #[ts(optional)]
    pub(crate) q: Option<String>,
}

#[derive(Serialize, Debug, TS, ToSchema)]
#[serde(rename = "UserSuggestion")]
#[ts(export, export_to = "UserSuggestion.ts")]
pub struct UserSuggestionResponse {
    pub username: String,
    /// Always 5 — hardcoded display value, not backed by any stored rating yet.
    pub note: u8,
    pub card_count: u64,
}

impl From<UserSuggestion> for UserSuggestionResponse {
    fn from(s: UserSuggestion) -> Self {
        Self {
            username: s.username,
            note: 5,
            card_count: s.card_count,
        }
    }
}
