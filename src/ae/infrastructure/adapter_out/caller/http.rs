//! HTTP helpers shared by the callers that tell "not found" apart from a technical failure.

use crate::application::error::{AppError, InfraError};
use ratelimit::{Ratelimiter, TryWaitError};
use reqwest::{Client, StatusCode};
use std::time::Duration;

/// Beyond this, a call is a technical failure rather than a slow answer.
const TIMEOUT: Duration = Duration::from_secs(30);

pub fn client() -> Client {
    Client::builder()
        .user_agent("reqwest")
        .timeout(TIMEOUT)
        .build()
        .unwrap()
}

/// Waits for a token of `ratelimiter` (`source` is only used in errors).
pub async fn throttle(ratelimiter: &Ratelimiter, source: &str) -> Result<(), AppError> {
    match ratelimiter.try_wait() {
        Ok(()) => Ok(()),
        Err(TryWaitError::Insufficient(duration)) => {
            tokio::time::sleep(duration).await;
            Ok(())
        }
        Err(TryWaitError::ExceedsCapacity) => {
            Err(InfraError::CallError(format!("{source} rate limiter overflow")).into())
        }
        Err(_) => Err(InfraError::CallError(format!("{source} rate limiter error")).into()),
    }
}

/// GETs `url`: `None` on a 404, an error on any other failure (timeout, 5xx, 429…).
pub async fn get_unless_not_found(
    client: &Client,
    url: &str,
) -> Result<Option<reqwest::Response>, AppError> {
    let response = client.get(url).send().await?;
    match response.status() {
        StatusCode::NOT_FOUND => Ok(None),
        status if status.is_success() => Ok(Some(response)),
        status => {
            Err(InfraError::CallError(format!("GET {url} failed with status {status}")).into())
        }
    }
}

/// GETs the bytes at `url`: `None` on a 404, an error on any other failure.
pub async fn get_bytes_unless_not_found(
    client: &Client,
    url: &str,
) -> Result<Option<Vec<u8>>, AppError> {
    match get_unless_not_found(client, url).await? {
        Some(response) => Ok(Some(response.bytes().await?.to_vec())),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::path;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn server_answering(status: u16) -> MockServer {
        let server = MockServer::start().await;
        Mock::given(path("/x"))
            .respond_with(ResponseTemplate::new(status).set_body_string("body"))
            .mount(&server)
            .await;
        server
    }

    #[tokio::test]
    async fn a_success_returns_the_body() {
        let server = server_answering(200).await;

        let bytes = get_bytes_unless_not_found(&client(), &format!("{}/x", server.uri()))
            .await
            .unwrap();

        assert_eq!(bytes, Some(b"body".to_vec()));
    }

    #[tokio::test]
    async fn a_404_is_not_found() {
        let server = server_answering(404).await;

        let bytes = get_bytes_unless_not_found(&client(), &format!("{}/x", server.uri()))
            .await
            .unwrap();

        assert_eq!(bytes, None);
    }

    #[tokio::test]
    async fn a_server_error_or_a_rate_limit_is_an_error() {
        for status in [500, 503, 429] {
            let server = server_answering(status).await;

            let result =
                get_bytes_unless_not_found(&client(), &format!("{}/x", server.uri())).await;

            assert!(result.is_err(), "status {status} should be an error");
        }
    }
}
