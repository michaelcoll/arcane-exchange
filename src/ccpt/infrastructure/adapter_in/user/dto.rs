use crate::domain::user::CollectionVisibility;
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

#[derive(Debug, Deserialize, TS, ToSchema)]
#[ts(export, export_to = "SetVisibilityRequest.ts")]
pub(crate) struct SetVisibilityRequest {
    pub(crate) visibility: CollectionVisibilityParam,
}
