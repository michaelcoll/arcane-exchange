use crate::domain::price::Price;

pub mod caller;
pub mod repository;

impl Price {
    pub fn from_cents(cents: u32) -> Price {
        Price { value: Some(cents) }
    }
}
