use crate::domain::binder::Binder;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

#[derive(Debug, Serialize, TS, ToSchema)]
#[serde(rename = "Binder")]
#[ts(export)]
pub struct BinderResponse {
    pub id: String,
    pub name: String,
    /// Copies stored in the binder: the sum of its entries' quantities, 0 when it is empty.
    pub copies: u64,
    /// Whether the binder is open for trade.
    pub is_open: bool,
}

impl From<Binder> for BinderResponse {
    fn from(binder: Binder) -> Self {
        Self {
            id: binder.id.to_string(),
            name: binder.name,
            copies: binder.copies,
            is_open: binder.is_open,
        }
    }
}

#[derive(Debug, Serialize, TS, ToSchema)]
#[serde(rename = "Binders")]
#[ts(export)]
pub struct BindersResponse {
    pub binders: Vec<BinderResponse>,
}

/// Body of the creation and of the renaming of a binder.
#[derive(Debug, Deserialize, TS, ToSchema)]
#[serde(rename = "BinderName")]
#[ts(export)]
pub(crate) struct BinderNameRequest {
    pub(crate) name: String,
}
