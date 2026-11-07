use serde::Serialize;
use utoipa::ToSchema;

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
