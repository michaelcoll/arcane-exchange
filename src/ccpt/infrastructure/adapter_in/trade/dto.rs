use serde::Deserialize;
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub(crate) struct CreateTradeRequest {
    pub(crate) set_code: String,
    pub(crate) collector_number: String,
    pub(crate) language_code: String,
    pub(crate) foil: bool,
    pub(crate) respondent_user_id: String,
    pub(crate) quantity: u8,
}

#[derive(Deserialize, ToSchema)]
pub(crate) struct RateTradeRequest {
    /// Rating given to the other party, from 0 to 5 inclusive.
    pub(crate) rating: u8,
}
