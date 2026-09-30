use crate::domain::user::User;
use serde::Serialize;
use ts_rs::TS;
use utoipa::ToSchema;

#[derive(Debug, Serialize, TS, ToSchema)]
#[serde(rename = "UserProfileResponse")]
#[ts(export, export_to = "UserProfileResponse.ts")]
pub struct UserProfileResponse {
    pub id: String,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
}

impl From<User> for UserProfileResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id.to_string(),
            username: user.username,
            avatar_url: user.avatar_url,
        }
    }
}
