use crate::application::caller::CardMarketCaller;
use crate::application::error::AppError;
use crate::domain::price::FullPriceGuide;
use crate::infrastructure::adapter_out::caller::dto::CardmarketPriceGuides;
use async_trait::async_trait;
use chrono::NaiveDate;

pub struct CardMarketCallerAdapter {
    pub client: reqwest::Client,
    pub url: String,
}

impl CardMarketCallerAdapter {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            url: url.into(),
        }
    }
}

#[async_trait]
impl CardMarketCaller for CardMarketCallerAdapter {
    async fn get_price_guides(&self) -> Result<(NaiveDate, Vec<FullPriceGuide>), AppError> {
        let price_guides: CardmarketPriceGuides = self
            .client
            .get(self.url.as_str())
            .send()
            .await?
            .json()
            .await?;

        let domain = price_guides
            .price_guides
            .into_iter()
            .map(|pg| pg.into())
            .collect();

        Ok((price_guides.created_at.date_naive(), domain))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn should_fetch_and_deserialize_price_guides() {
        // Arrange
        let mock_server = MockServer::start().await;

        let json_body = r#"
        {
          "version": 1,
          "createdAt": "2025-12-23T02:47:26+0100",
          "priceGuides": [
            {
              "idProduct": 1,
              "idCategory": 1,
              "avg": 0.06,
              "low": 0.02,
              "trend": 0.09,
              "avg-foil": 0.5,
              "low-foil": 0.04,
              "trend-foil": 0.42
            },
            {
              "idProduct": 2,
              "idCategory": 1,
              "avg": 0.06,
              "low": 0.02,
              "trend": 0.07,
              "avg-foil": null,
              "low-foil": 0.05,
              "trend-foil": 0.28
            }
          ]
        }
        "#;

        Mock::given(method("GET"))
            .and(path("/price_guide.json"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(json_body, "application/json"))
            .mount(&mock_server)
            .await;

        let adapter =
            CardMarketCallerAdapter::new(format!("{}/price_guide.json", mock_server.uri()));

        // Act
        let result = adapter.get_price_guides().await;

        // Assert
        assert!(result.is_ok());

        let (date, price_guides) = result.unwrap();
        // Assertions racine
        assert_eq!(date, NaiveDate::from_ymd_opt(2025, 12, 23).unwrap(),);

        let actual: Vec<FullPriceGuide> = price_guides;

        // Assertions collection
        assert_eq!(actual.len(), 2);

        let first = &actual[0];
        assert_eq!(first.id_product, 1);
        assert_eq!(first.normal.avg, Some(0.06).into());
        assert_eq!(first.normal.low, Some(0.02).into());
        assert_eq!(first.normal.trend, Some(0.09).into());
        assert_eq!(first.foil.avg, Some(0.5).into());
        assert_eq!(first.foil.low, Some(0.04).into());
        assert_eq!(first.foil.trend, Some(0.42).into());

        let second = &actual[1];
        assert_eq!(second.id_product, 2);
        assert_eq!(second.foil.avg, None::<f32>.into());
        assert_eq!(second.normal.trend, Some(0.07).into());
    }

    #[tokio::test]
    async fn should_return_error_when_http_fails() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        let adapter = CardMarketCallerAdapter::new(mock_server.uri());

        let result = adapter.get_price_guides().await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn should_fail_when_json_is_invalid() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw("this is not json", "application/json"),
            )
            .mount(&mock_server)
            .await;

        let adapter = CardMarketCallerAdapter::new(mock_server.uri());

        let result = adapter.get_price_guides().await;

        assert!(result.is_err());
    }
}
