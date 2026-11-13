use crate::domain::purchase::{
    Purchase, PurchaseContentLine, PurchaseDetail, PurchaseEditRequest, PurchaseKind,
    SealedProductRequest, SealedProductType,
};
use crate::domain::set_name::SetCode;
use crate::infrastructure::adapter_in::collection::dto::card_image_urls;
use crate::infrastructure::adapter_in::{SetInfoResponse, default_page_size};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, ToSchema)]
#[serde(rename = "SealedProductType", rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export)]
pub enum SealedProductTypeParam {
    Booster,
    Display,
    Bundle,
    PrereleasePack,
    PreconstructedDeck,
}

impl From<SealedProductTypeParam> for SealedProductType {
    fn from(param: SealedProductTypeParam) -> Self {
        match param {
            SealedProductTypeParam::Booster => SealedProductType::Booster,
            SealedProductTypeParam::Display => SealedProductType::Display,
            SealedProductTypeParam::Bundle => SealedProductType::Bundle,
            SealedProductTypeParam::PrereleasePack => SealedProductType::PrereleasePack,
            SealedProductTypeParam::PreconstructedDeck => SealedProductType::PreconstructedDeck,
        }
    }
}

impl From<SealedProductType> for SealedProductTypeParam {
    fn from(product_type: SealedProductType) -> Self {
        match product_type {
            SealedProductType::Booster => SealedProductTypeParam::Booster,
            SealedProductType::Display => SealedProductTypeParam::Display,
            SealedProductType::Bundle => SealedProductTypeParam::Bundle,
            SealedProductType::PrereleasePack => SealedProductTypeParam::PrereleasePack,
            SealedProductType::PreconstructedDeck => SealedProductTypeParam::PreconstructedDeck,
        }
    }
}

/// What a Purchase bought.
#[derive(Debug, Serialize, TS, ToSchema)]
#[serde(rename = "PurchaseKind", rename_all = "snake_case")]
#[ts(export)]
pub enum PurchaseKindResponse {
    SealedProduct,
}

#[derive(Debug, Serialize, TS, ToSchema)]
#[serde(rename = "Purchase")]
#[ts(export)]
pub struct PurchaseResponse {
    pub id: String,
    pub kind: PurchaseKindResponse,
    /// Amount paid, in cents.
    pub amount_cents: u32,
    /// Day of the purchase, `YYYY-MM-DD`.
    #[ts(type = "string")]
    #[schema(value_type = String, format = Date)]
    pub purchased_on: NaiveDate,
    /// Sealed product only.
    #[schema(value_type = SealedProductTypeParam, required = false)]
    pub product_type: Option<SealedProductTypeParam>,
    /// Sealed product only: the optional name the player gave it.
    pub name: Option<String>,
    /// Sealed product only.
    #[schema(value_type = SetInfoResponse, required = false)]
    pub set: Option<SetInfoResponse>,
}

impl From<Purchase> for PurchaseResponse {
    fn from(purchase: Purchase) -> Self {
        let PurchaseKind::SealedProduct {
            product_type,
            name,
            set,
        } = purchase.kind;

        Self {
            id: purchase.id.to_string(),
            kind: PurchaseKindResponse::SealedProduct,
            amount_cents: purchase.amount.cents(),
            purchased_on: purchase.purchased_on,
            product_type: Some(product_type.into()),
            name,
            set: Some(set.into()),
        }
    }
}

/// A card of the content of a Purchase, in a finish.
#[derive(Debug, Serialize, TS, ToSchema)]
#[serde(rename = "PurchaseContentLine")]
#[ts(export)]
pub struct PurchaseContentLineResponse {
    pub set_code: String,
    pub collector_number: String,
    pub language_code: String,
    pub name: String,
    /// Front image, `null` while it is pending.
    pub image_url: Option<String>,
    pub foil: bool,
    /// Copies of this card and finish the Purchase brought.
    pub quantity: u32,
    /// Current Cardmarket trend price of the card in its finish, in cents; `null` when
    /// Cardmarket has none. Read on each request, never stored with the Purchase.
    pub trend_cents: Option<u32>,
}

impl From<PurchaseContentLine> for PurchaseContentLineResponse {
    fn from(line: PurchaseContentLine) -> Self {
        let (image_url, _) = card_image_urls(&line.card_id, line.image);
        Self {
            set_code: line.card_id.set_code.to_string(),
            collector_number: line.card_id.collector_number,
            language_code: line.card_id.language_code.to_string(),
            name: line.name,
            image_url,
            foil: line.foil,
            quantity: line.quantity,
            trend_cents: line.trend_cents,
        }
    }
}

/// A Purchase with its content and its profitability.
#[derive(Debug, Serialize, TS, ToSchema)]
#[serde(rename = "PurchaseDetail")]
#[ts(export)]
pub struct PurchaseDetailResponse {
    pub purchase: PurchaseResponse,
    pub content: Vec<PurchaseContentLineResponse>,
    /// Sum of the quantities times the trend price of the lines that have one, in cents.
    pub content_value_cents: u64,
    /// Lines left out of the value because they have no trend price.
    pub lines_without_trend: u32,
    /// Profitability: the value of the content minus the amount of the Purchase, in cents,
    /// negative when the content is worth less than was paid.
    pub profitability_cents: i64,
    /// The profitability as a percentage of the amount of the Purchase.
    pub profitability_percent: f64,
}

impl From<PurchaseDetail> for PurchaseDetailResponse {
    fn from(detail: PurchaseDetail) -> Self {
        Self {
            content_value_cents: detail.content_value_cents(),
            lines_without_trend: detail.lines_without_trend(),
            profitability_cents: detail.profitability_cents(),
            profitability_percent: detail.profitability_percent(),
            purchase: detail.purchase.into(),
            content: detail.content.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Serialize, TS, ToSchema)]
#[serde(rename = "PaginatedPurchases")]
#[ts(export)]
pub struct PaginatedPurchasesResponse {
    pub items: Vec<PurchaseResponse>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub(crate) struct PurchasesParams {
    #[serde(default)]
    pub(crate) page: u32,
    #[serde(default = "default_page_size")]
    pub(crate) page_size: u32,
}

/// A sealed product to record.
#[derive(Debug, Deserialize, TS, ToSchema)]
#[serde(rename = "CreateSealedProductRequest")]
#[ts(export)]
pub(crate) struct CreateSealedProductRequest {
    pub(crate) product_type: SealedProductTypeParam,
    /// Code of a known set.
    pub(crate) set_code: String,
    /// Free-text label; a blank name is stored as absent.
    #[serde(default)]
    #[ts(optional)]
    pub(crate) name: Option<String>,
    /// Amount paid, in cents. Signed so an amount that is not strictly positive fails with
    /// `invalid_purchase_price` (400) instead of a JSON extractor rejection.
    pub(crate) amount_cents: i32,
    /// Day of the purchase, `YYYY-MM-DD`.
    #[ts(type = "string")]
    #[schema(value_type = String, format = Date)]
    pub(crate) purchased_on: NaiveDate,
}

impl CreateSealedProductRequest {
    pub(crate) fn into_domain(
        self,
    ) -> Result<SealedProductRequest, crate::application::error::AppError> {
        Ok(SealedProductRequest {
            product_type: self.product_type.into(),
            set_code: SetCode::try_new(self.set_code)?,
            name: self.name,
            amount_cents: self.amount_cents,
            purchased_on: self.purchased_on,
        })
    }
}

/// The fields of a Purchase to change; those left out are kept.
#[derive(Debug, Deserialize, TS, ToSchema)]
#[serde(rename = "UpdatePurchaseRequest")]
#[ts(export)]
pub(crate) struct UpdatePurchaseRequest {
    /// Amount paid, in cents, strictly positive (`invalid_purchase_price`, 400).
    #[serde(default)]
    #[ts(optional)]
    pub(crate) amount_cents: Option<i32>,
    /// Day of the purchase, `YYYY-MM-DD`.
    #[serde(default)]
    #[ts(optional, type = "string")]
    #[schema(value_type = Option<String>, format = Date)]
    pub(crate) purchased_on: Option<NaiveDate>,
    #[serde(default)]
    #[ts(optional)]
    #[schema(value_type = SealedProductTypeParam, required = false)]
    pub(crate) product_type: Option<SealedProductTypeParam>,
    /// Code of a known set.
    #[serde(default)]
    #[ts(optional)]
    pub(crate) set_code: Option<String>,
    /// An empty name clears the stored one.
    #[serde(default)]
    #[ts(optional)]
    pub(crate) name: Option<String>,
}

impl UpdatePurchaseRequest {
    pub(crate) fn into_domain(
        self,
    ) -> Result<PurchaseEditRequest, crate::application::error::AppError> {
        Ok(PurchaseEditRequest {
            amount_cents: self.amount_cents,
            purchased_on: self.purchased_on,
            product_type: self.product_type.map(Into::into),
            set_code: self.set_code.map(SetCode::try_new).transpose()?,
            name: self.name,
        })
    }
}
