use crate::domain::price::Price;

pub mod caller;
pub mod clerk_token_validator_adapter;
pub mod repository;

impl Price {
    pub fn from_cents(cents: u32) -> Price {
        Price { value: Some(cents) }
    }
}
