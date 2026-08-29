use crate::domain::user::{CollectionVisibility, User};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[serde(rename = "CollectionVisibility", rename_all = "snake_case")]
#[ts(export, export_to = "CollectionVisibility.ts")]
pub enum CollectionVisibilityParam {
    Public,
    Trade,
    Private,
}

impl From<CollectionVisibilityParam> for CollectionVisibility {
    fn from(p: CollectionVisibilityParam) -> Self {
        match p {
            CollectionVisibilityParam::Public => CollectionVisibility::Public,
            CollectionVisibilityParam::Trade => CollectionVisibility::Trade,
            CollectionVisibilityParam::Private => CollectionVisibility::Private,
        }
    }
}

impl From<CollectionVisibility> for CollectionVisibilityParam {
    fn from(v: CollectionVisibility) -> Self {
        match v {
            CollectionVisibility::Public => CollectionVisibilityParam::Public,
            CollectionVisibility::Trade => CollectionVisibilityParam::Trade,
            CollectionVisibility::Private => CollectionVisibilityParam::Private,
        }
    }
}

#[derive(Debug, Serialize, TS, ToSchema)]
#[serde(rename = "VisibilityResponse")]
#[ts(export, export_to = "VisibilityResponse.ts")]
pub struct VisibilityResponse {
    pub visibility: CollectionVisibilityParam,
}

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

#[derive(Debug, Deserialize, TS, ToSchema)]
#[ts(export, export_to = "SetVisibilityRequest.ts")]
pub(crate) struct SetVisibilityRequest {
    pub(crate) visibility: CollectionVisibilityParam,
}

#[derive(Debug, Serialize, TS, ToSchema)]
#[ts(export, export_to = "TradeBindersResponse.ts")]
pub struct TradeBindersResponse {
    pub binders: Vec<String>,
}

#[derive(Debug, Deserialize, TS, ToSchema)]
#[ts(export, export_to = "AddTradeBinderRequest.ts")]
pub(crate) struct AddTradeBinderRequest {
    pub(crate) binder_name: String,
}
