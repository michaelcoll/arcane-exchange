use crate::application::error::{AppError, InfraError};

pub mod cardmarket_caller_adapter;
mod dto;
pub mod edhrec_caller_adapter;
pub mod gatherer_caller_adapter;
pub mod scryfall_caller_adapter;

impl From<reqwest::Error> for AppError {
    fn from(value: reqwest::Error) -> Self {
        println!("Reqwest error: {:?}", value);
        InfraError::CallError(value.to_string()).into()
    }
}
