use crate::application::caller::ScryfallCaller;
use crate::application::error::{AppError, InfraError};
use crate::domain::card_image::CardImages;
use crate::infrastructure::adapter_out::caller::dto::ScryfallCardInfo;
use crate::infrastructure::adapter_out::caller::http;
use async_trait::async_trait;
use ratelimit::Ratelimiter;
use uuid::Uuid;

/// Quality of the lossy WebP re-encoding of Scryfall JPEG images (0-100).
const WEBP_QUALITY: f32 = 90.0;

pub struct ScryfallCallerAdapter {
    client: reqwest::Client,
    scryfall_base_url: String,
    ratelimiter: Ratelimiter,
}

impl ScryfallCallerAdapter {
    pub fn new(scryfall_base_url: impl Into<String>, rate_limit_tokens: u32) -> Self {
        let rate_limit_tokens = u64::from(rate_limit_tokens);
        Self {
            client: http::client(),
            scryfall_base_url: scryfall_base_url.into(),
            ratelimiter: Ratelimiter::builder(rate_limit_tokens)
                .max_tokens(rate_limit_tokens)
                .build()
                .unwrap(),
        }
    }

    fn card_url(&self, id: Uuid) -> String {
        format!("{}/cards/{}?format=json", self.scryfall_base_url, id)
    }

    /// Downloads a JPEG image and re-encodes it as WebP, or `None` on a 404.
    async fn get_image_as_webp(&self, url: &str) -> Result<Option<Vec<u8>>, AppError> {
        http::throttle(&self.ratelimiter, "Scryfall").await?;
        let Some(jpeg) = http::get_bytes_unless_not_found(&self.client, url).await? else {
            return Ok(None);
        };
        let webp = tokio::task::spawn_blocking(move || jpeg_to_webp(&jpeg))
            .await
            .map_err(|e| InfraError::CallError(format!("WebP encoding task failed: {e}")))??;
        Ok(Some(webp))
    }
}

fn jpeg_to_webp(jpeg: &[u8]) -> Result<Vec<u8>, AppError> {
    let rgb = image::load_from_memory_with_format(jpeg, image::ImageFormat::Jpeg)
        .map_err(|e| InfraError::CallError(format!("invalid Scryfall JPEG image: {e}")))?
        .to_rgb8();
    let webp =
        webp::Encoder::from_rgb(rgb.as_raw(), rgb.width(), rgb.height()).encode(WEBP_QUALITY);
    Ok(webp.to_vec())
}

#[async_trait]
impl ScryfallCaller for ScryfallCallerAdapter {
    #[tracing::instrument(name = "scryfall.get_card_market_id", skip_all, fields(sentry.op = "http.client"))]
    async fn get_card_market_id(&self, id: Uuid) -> Result<Option<u32>, AppError> {
        http::throttle(&self.ratelimiter, "Scryfall").await?;

        let card_info: ScryfallCardInfo = self
            .client
            .get(self.card_url(id))
            .send()
            .await?
            .json()
            .await?;

        Ok(card_info.cardmarket_id.map(|id| id as u32))
    }

    #[tracing::instrument(name = "scryfall.get_card_images", skip_all, fields(sentry.op = "http.client"))]
    async fn get_card_images(&self, id: Uuid) -> Result<Option<CardImages>, AppError> {
        http::throttle(&self.ratelimiter, "Scryfall").await?;
        let Some(response) = http::get_unless_not_found(&self.client, &self.card_url(id)).await?
        else {
            return Ok(None);
        };
        let card_info: ScryfallCardInfo = response.json().await?;
        let Some((front_url, back_url)) = card_info.normal_image_urls() else {
            return Ok(None);
        };

        let Some(front) = self.get_image_as_webp(&front_url).await? else {
            return Ok(None);
        };
        let back = match back_url {
            None => None,
            Some(back_url) => match self.get_image_as_webp(&back_url).await? {
                Some(back) => Some(back),
                None => return Ok(None),
            },
        };
        Ok(Some(CardImages { front, back }))
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

    fn jpeg(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        image::RgbImage::from_pixel(width, height, image::Rgb([200, 30, 30]))
            .write_with_encoder(image::codecs::jpeg::JpegEncoder::new(&mut bytes))
            .unwrap();
        bytes
    }

    async fn mount_card(server: &MockServer, card: serde_json::Value) {
        Mock::given(path(format!("/cards/{}", Uuid::nil())))
            .respond_with(ResponseTemplate::new(200).set_body_json(card))
            .mount(server)
            .await;
    }

    async fn mount_jpeg(server: &MockServer, url_path: &str) {
        Mock::given(path(url_path))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(jpeg(488, 680)))
            .mount(server)
            .await;
    }

    #[tokio::test]
    async fn get_card_images_converts_the_normal_jpeg_to_webp() {
        use crate::domain::card_image::webp_width;

        let server = MockServer::start().await;
        mount_card(
            &server,
            serde_json::json!({
                "layout": "adventure",
                "image_uris": {
                    "normal": format!("{}/normal/front.jpg", server.uri()),
                    "large": format!("{}/large/front.jpg", server.uri())
                },
                "card_faces": [{ "name": "Creature" }, { "name": "Adventure" }]
            }),
        )
        .await;
        mount_jpeg(&server, "/normal/front.jpg").await;

        let images = ScryfallCallerAdapter::new(server.uri(), 8)
            .get_card_images(Uuid::nil())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(webp_width(&images.front), Some(488));
        assert_eq!(images.back, None);
    }

    #[tokio::test]
    async fn get_card_images_returns_both_faces_of_a_double_faced_card() {
        use crate::domain::card_image::webp_width;

        let server = MockServer::start().await;
        mount_card(
            &server,
            serde_json::json!({
                "layout": "transform",
                "card_faces": [
                    { "image_uris": { "normal": format!("{}/normal/front.jpg", server.uri()) } },
                    { "image_uris": { "normal": format!("{}/normal/back.jpg", server.uri()) } }
                ]
            }),
        )
        .await;
        mount_jpeg(&server, "/normal/front.jpg").await;
        mount_jpeg(&server, "/normal/back.jpg").await;

        let images = ScryfallCallerAdapter::new(server.uri(), 8)
            .get_card_images(Uuid::nil())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(webp_width(&images.front), Some(488));
        assert_eq!(images.back.as_deref().and_then(webp_width), Some(488));
    }

    #[tokio::test]
    async fn get_card_images_is_none_for_an_unknown_card() {
        let server = MockServer::start().await;
        Mock::given(path(format!("/cards/{}", Uuid::nil())))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let images = ScryfallCallerAdapter::new(server.uri(), 8)
            .get_card_images(Uuid::nil())
            .await
            .unwrap();

        assert_eq!(images, None);
    }

    #[tokio::test]
    async fn get_card_images_is_none_for_a_card_without_image() {
        let server = MockServer::start().await;
        mount_card(&server, serde_json::json!({ "layout": "normal" })).await;

        let images = ScryfallCallerAdapter::new(server.uri(), 8)
            .get_card_images(Uuid::nil())
            .await
            .unwrap();

        assert_eq!(images, None);
    }

    #[tokio::test]
    async fn get_card_images_is_none_when_the_front_image_is_404() {
        let server = MockServer::start().await;
        mount_card(
            &server,
            serde_json::json!({
                "image_uris": { "normal": format!("{}/normal/front.jpg", server.uri()) }
            }),
        )
        .await;
        Mock::given(path("/normal/front.jpg"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let images = ScryfallCallerAdapter::new(server.uri(), 8)
            .get_card_images(Uuid::nil())
            .await
            .unwrap();

        assert_eq!(images, None);
    }

    #[tokio::test]
    async fn get_card_images_fails_on_a_server_error() {
        let server = MockServer::start().await;
        Mock::given(path(format!("/cards/{}", Uuid::nil())))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;

        let result = ScryfallCallerAdapter::new(server.uri(), 8)
            .get_card_images(Uuid::nil())
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn get_card_images_fails_when_the_image_download_fails() {
        let server = MockServer::start().await;
        mount_card(
            &server,
            serde_json::json!({
                "image_uris": { "normal": format!("{}/normal/front.jpg", server.uri()) }
            }),
        )
        .await;
        Mock::given(path("/normal/front.jpg"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let result = ScryfallCallerAdapter::new(server.uri(), 8)
            .get_card_images(Uuid::nil())
            .await;

        assert!(result.is_err());
    }
}
