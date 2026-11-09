use crate::domain::catalog::CatalogSearchItem;
use crate::infrastructure::adapter_in::{SetInfoResponse, default_page_size};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize)]
pub(crate) struct CatalogSearchParams {
    pub(crate) q: Option<String>,
    pub(crate) set_code: Option<String>,
    #[serde(default)]
    pub(crate) page: u32,
    #[serde(default = "default_page_size")]
    pub(crate) page_size: u32,
}

/// One impression of the catalog: a set and a collector number, without language or finish.
#[derive(Serialize, Debug, ToSchema)]
#[serde(rename = "CatalogCard")]
pub struct CatalogCardResponse {
    pub set: SetInfoResponse,
    pub collector_number: String,
    /// Rarity code: `C`, `U`, `R`, `M` or `S`.
    pub rarity_code: String,
    /// English name.
    pub name: String,
    /// French name; absent when Scryfall has none.
    pub french_name: Option<String>,
    /// Scryfall image of the impression (its front face for a double-faced card), to load from
    /// Scryfall's CDN as is; `null` when Scryfall has none. It is not the image of a Card.
    pub image_url: Option<String>,
}

impl From<CatalogSearchItem> for CatalogCardResponse {
    fn from(item: CatalogSearchItem) -> Self {
        Self {
            set: SetInfoResponse::from(item.set),
            collector_number: item.collector_number,
            rarity_code: item.rarity.to_string(),
            name: item.name,
            french_name: item.french_name,
            image_url: item.image_url,
        }
    }
}

#[derive(Serialize, Debug, ToSchema)]
#[serde(rename = "PaginatedCatalog")]
pub struct PaginatedCatalogResponse {
    pub items: Vec<CatalogCardResponse>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}

/// One entry of the scan index, keyed by set and collector number.
///
/// The body of `GET /catalog/scan-index` is a JSON array of these; it is built and gzipped once
/// per catalog load by the `ScanIndexUseCase` from the serialization of `ScanIndexEntry`, this
/// type only documents its shape (a controller test decodes the served body as this type).
#[derive(Serialize, Debug, ToSchema)]
#[cfg_attr(test, derive(serde::Deserialize), serde(deny_unknown_fields))]
pub struct ScanIndexEntryResponse {
    /// Set code, 3 to 6 uppercase characters.
    pub set_code: String,
    /// Purely numeric collector number, without leading zeros.
    pub collector_number: String,
    /// Rarity code: `C`, `U`, `R`, `M` or `S`.
    pub rarity: String,
    /// English name.
    pub name: String,
    /// French name; absent when Scryfall does not have one, the client then shows the English name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub french_name: Option<String>,
}
