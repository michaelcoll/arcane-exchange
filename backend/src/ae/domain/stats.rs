use chrono::NaiveDate;

#[derive(Debug, Clone, PartialEq)]
pub struct Stats {
    pub card_number: u32,
    pub card_price_number: u32,
    pub db_size_mb: u16,
    /// Date of the most recent Cardmarket price — prices are dated to the day. `None` until a
    /// price has been imported.
    pub last_price_date: Option<NaiveDate>,
    /// Copies offered for trade across the platform: the sum of every player's proposed
    /// quantities.
    pub proposed_copy_number: u32,
}
