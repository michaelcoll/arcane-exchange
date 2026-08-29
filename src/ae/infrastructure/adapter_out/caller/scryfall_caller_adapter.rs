use crate::application::caller::ScryfallCaller;
use crate::application::error::{AppError, InfraError};
use crate::infrastructure::adapter_out::caller::dto::ScryfallCardInfo;
use async_trait::async_trait;
use ratelimit::{Ratelimiter, TryWaitError};
use uuid::Uuid;

pub struct ScryfallCallerAdapter {
    client: reqwest::Client,
    scryfall_base_url: String,
    ratelimiter: Ratelimiter,
}

impl ScryfallCallerAdapter {
    pub fn new(scryfall_base_url: impl Into<String>, rate_limit_tokens: u32) -> Self {
        let rate_limit_tokens = u64::from(rate_limit_tokens);
        Self {
            client: reqwest::Client::builder()
                .user_agent("reqwest")
                .build()
                .unwrap(),
            scryfall_base_url: scryfall_base_url.into(),
            ratelimiter: Ratelimiter::builder(rate_limit_tokens)
                .max_tokens(rate_limit_tokens)
                .build()
                .unwrap(),
        }
    }
}

#[async_trait]
impl ScryfallCaller for ScryfallCallerAdapter {
    async fn get_card_market_id(&self, id: Uuid) -> Result<Option<u32>, AppError> {
        let url = format!("{}/cards/{}?format=json", self.scryfall_base_url, id);

        if let Err(err) = self.ratelimiter.try_wait() {
            match err {
                TryWaitError::Insufficient(duration) => {
                    tokio::time::sleep(duration).await;
                }
                TryWaitError::ExceedsCapacity => {
                    return Err(InfraError::CallError(
                        "Scryfall rate limiter overflow".to_string(),
                    )
                    .into());
                }
                _ => {
                    return Err(
                        InfraError::CallError("Scryfall rate limiter error".to_string()).into(),
                    );
                }
            }
        }

        let card_info: ScryfallCardInfo =
            self.client.get(url.as_str()).send().await?.json().await?;

        Ok(card_info.cardmarket_id.map(|id| id as u32))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use wiremock::matchers::path;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn get_card_market_id_returns_cardmarket_id() {
        let mock_server = MockServer::start().await;

        let card_id = Uuid::default();
        let cardmarket_id = 12345;
        let response_body = format!(r#"{{ "cardmarket_id": {} }}"#, cardmarket_id);

        Mock::given(path(format!("/cards/{}", card_id)))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let adapter = ScryfallCallerAdapter::new(mock_server.uri(), 8);
        let result = adapter.get_card_market_id(card_id).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(cardmarket_id));
    }

    #[tokio::test]
    async fn get_card_market_id_returns_none_for_missing_cardmarket_id() {
        let mock_server = MockServer::start().await;

        let card_id = Uuid::default();
        let response_body = r#"{ "cardmarket_id": null }"#;

        Mock::given(path(format!("/cards/{}", card_id)))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let adapter = ScryfallCallerAdapter::new(mock_server.uri(), 8);
        let result = adapter.get_card_market_id(card_id).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[tokio::test]
    async fn get_card_market_id_returns_none_for_invalid_response() {
        let mock_server = MockServer::start().await;

        let card_id = Uuid::default();
        let response_body = r#"{ "invalid_field": "invalid_value" }"#;

        Mock::given(path(format!("/cards/{}", card_id)))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let adapter = ScryfallCallerAdapter::new(mock_server.uri(), 8);
        let result = adapter.get_card_market_id(card_id).await;

        assert!(result.is_ok());
        if let Ok(id) = result {
            assert!(id.is_none());
        }
    }
}
