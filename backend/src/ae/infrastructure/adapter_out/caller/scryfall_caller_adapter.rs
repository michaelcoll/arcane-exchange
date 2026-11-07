use crate::application::caller::{ScryfallBulkCard, ScryfallBulkCardStream, ScryfallCaller};
use crate::application::error::{AppError, InfraError};
use crate::domain::card_image::CardImages;
use crate::infrastructure::adapter_out::caller::dto::{
    ScryfallBulkCardLine, ScryfallBulkDataList, ScryfallCardInfo,
};
use crate::infrastructure::adapter_out::caller::http;
use async_trait::async_trait;
use flate2::read::GzDecoder;
use futures::{StreamExt, TryStreamExt};
use ratelimit::Ratelimiter;
use std::io::{BufRead, BufReader, Read};
use tokio_util::io::{StreamReader, SyncIoBridge};
use uuid::Uuid;

/// Quality of the lossy WebP re-encoding of Scryfall JPEG images (0-100).
const WEBP_QUALITY: f32 = 90.0;

/// The `type` of the `bulk-data` entry holding every card in every language.
const ALL_CARDS_BULK_TYPE: &str = "all_cards";

/// Decoded cards waiting for the consumer while the download goes on.
const BULK_CHANNEL_CAPACITY: usize = 1024;

pub struct ScryfallCallerAdapter {
    client: reqwest::Client,
    /// For the bulk download only: see `http::streaming_client`.
    bulk_client: reqwest::Client,
    scryfall_base_url: String,
    ratelimiter: Ratelimiter,
}

impl ScryfallCallerAdapter {
    pub fn new(scryfall_base_url: impl Into<String>, rate_limit_tokens: u32) -> Self {
        let rate_limit_tokens = u64::from(rate_limit_tokens);
        Self {
            client: http::client(),
            bulk_client: http::streaming_client(),
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

    #[tracing::instrument(name = "scryfall.stream_bulk_cards", skip_all, fields(sentry.op = "http.client"))]
    async fn stream_bulk_cards(&self) -> Result<ScryfallBulkCardStream, AppError> {
        http::throttle(&self.ratelimiter, "Scryfall").await?;
        let bulk_data: ScryfallBulkDataList = self
            .client
            .get(format!("{}/bulk-data", self.scryfall_base_url))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let download_uri = bulk_data
            .data
            .into_iter()
            .find(|bulk| bulk.kind == ALL_CARDS_BULK_TYPE)
            .and_then(|bulk| bulk.jsonl_download_uri)
            .ok_or_else(|| {
                InfraError::CallError("Scryfall bulk-data has no downloadable all_cards".into())
            })?;

        let response = self
            .bulk_client
            .get(download_uri)
            .send()
            .await?
            .error_for_status()?;
        let body = StreamReader::new(
            response
                .bytes_stream()
                .map_err(|e| std::io::Error::other(e.to_string())),
        );

        Ok(bulk_card_stream(move |sender| {
            read_bulk(SyncIoBridge::new(body), sender)
        }))
    }
}

/// Decompressing and parsing are blocking work: `read` runs on a blocking thread and hands the
/// cards over a bounded channel, so a slow consumer slows the download down instead of filling the
/// memory. The stream ends when `read` returns; a `read` that panics ends it with an error, so that
/// a reading cut short is never taken for a complete bulk.
fn bulk_card_stream(
    read: impl FnOnce(tokio::sync::mpsc::Sender<Result<ScryfallBulkCard, AppError>>) + Send + 'static,
) -> ScryfallBulkCardStream {
    let (sender, mut receiver) = tokio::sync::mpsc::channel(BULK_CHANNEL_CAPACITY);
    let reader = tokio::task::spawn_blocking(move || read(sender));

    let cards = futures::stream::poll_fn(move |cx| receiver.poll_recv(cx));
    let reader_failure = futures::stream::once(reader).filter_map(|joined| async move {
        joined.err().map(|e| {
            Err(InfraError::CallError(format!("the Scryfall bulk reader failed: {e}")).into())
        })
    });
    Box::pin(cards.chain(reader_failure))
}

/// Reads the gzipped JSON lines of the bulk, sending each decoded card, or the error that ends the
/// reading. Stops silently when the receiver is gone.
fn read_bulk(
    body: impl Read,
    sender: tokio::sync::mpsc::Sender<Result<ScryfallBulkCard, AppError>>,
) {
    for line in BufReader::new(GzDecoder::new(body)).lines() {
        let item = line
            .map_err(|e| InfraError::CallError(format!("cannot read the Scryfall bulk: {e}")))
            .and_then(|line| {
                serde_json::from_str::<ScryfallBulkCardLine>(&line).map_err(|e| {
                    InfraError::CallError(format!("cannot decode a Scryfall bulk card: {e}"))
                })
            })
            .map(ScryfallBulkCard::from)
            .map_err(AppError::from);
        let failed = item.is_err();
        if sender.blocking_send(item).is_err() || failed {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::caller::{ScryfallBulkCard, ScryfallBulkCardFace};
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

    const BULK_EXCERPT: &str = include_str!("testdata/scryfall_bulk_excerpt.jsonl");

    fn gzip(content: &str) -> Vec<u8> {
        use std::io::Write;
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(content.as_bytes()).unwrap();
        encoder.finish().unwrap()
    }

    /// A Scryfall serving `bulk` as its `all_cards` file, next to a decoy `default_cards` one.
    async fn server_with_bulk(bulk: Vec<u8>) -> MockServer {
        let server = MockServer::start().await;
        Mock::given(path("/bulk-data"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "has_more": false,
                "data": [
                    {
                        "object": "bulk_data",
                        "type": "default_cards",
                        "jsonl_download_uri": format!("{}/default-cards.jsonl.gz", server.uri())
                    },
                    {
                        "object": "bulk_data",
                        "type": "all_cards",
                        "jsonl_download_uri": format!("{}/all-cards.jsonl.gz", server.uri())
                    }
                ]
            })))
            .mount(&server)
            .await;
        Mock::given(path("/all-cards.jsonl.gz"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(bulk))
            .mount(&server)
            .await;
        server
    }

    async fn read_bulk(
        adapter: &ScryfallCallerAdapter,
    ) -> Result<Vec<Result<ScryfallBulkCard, AppError>>, AppError> {
        use futures::StreamExt;
        Ok(adapter.stream_bulk_cards().await?.collect().await)
    }

    fn face(name: &str, printed_name: Option<&str>) -> ScryfallBulkCardFace {
        ScryfallBulkCardFace {
            name: name.to_string(),
            printed_name: printed_name.map(str::to_string),
        }
    }

    #[tokio::test]
    async fn stream_bulk_cards_decodes_a_french_card_transforms_and_a_token() {
        let server = server_with_bulk(gzip(BULK_EXCERPT)).await;

        let cards: Vec<ScryfallBulkCard> = read_bulk(&ScryfallCallerAdapter::new(server.uri(), 8))
            .await
            .unwrap()
            .into_iter()
            .collect::<Result<_, _>>()
            .unwrap();

        assert_eq!(
            cards,
            vec![
                ScryfallBulkCard {
                    scryfall_id: "f0b00465-03ba-42ee-b2dc-319443044df6".parse().unwrap(),
                    set_code: "mkm".to_string(),
                    set_name: "Murders at Karlov Manor".to_string(),
                    set_type: "expansion".to_string(),
                    collector_number: "12".to_string(),
                    language: "fr".to_string(),
                    layout: "normal".to_string(),
                    rarity: "mythic".to_string(),
                    name: "Delney, Streetwise Lookout".to_string(),
                    printed_name: Some("Delney, vigie débrouillarde".to_string()),
                    faces: vec![],
                    cardmarket_id: None,
                    digital: false,
                },
                ScryfallBulkCard {
                    scryfall_id: "54d4e7c3-294d-4900-8b70-faafda17cc33".parse().unwrap(),
                    set_code: "mid".to_string(),
                    set_name: "Innistrad: Midnight Hunt".to_string(),
                    set_type: "expansion".to_string(),
                    collector_number: "2".to_string(),
                    language: "en".to_string(),
                    layout: "transform".to_string(),
                    rarity: "uncommon".to_string(),
                    name: "Ambitious Farmhand // Seasoned Cathar".to_string(),
                    printed_name: None,
                    faces: vec![
                        face("Ambitious Farmhand", None),
                        face("Seasoned Cathar", None),
                    ],
                    cardmarket_id: Some(574956),
                    digital: false,
                },
                ScryfallBulkCard {
                    scryfall_id: "94153f04-c319-4f03-b324-84919e794a9e".parse().unwrap(),
                    set_code: "mid".to_string(),
                    set_name: "Innistrad: Midnight Hunt".to_string(),
                    set_type: "expansion".to_string(),
                    collector_number: "2".to_string(),
                    language: "fr".to_string(),
                    layout: "transform".to_string(),
                    rarity: "uncommon".to_string(),
                    name: "Ambitious Farmhand // Seasoned Cathar".to_string(),
                    // Scryfall leaves it null on a multi-face object: it is on the faces.
                    printed_name: None,
                    faces: vec![
                        face("Ambitious Farmhand", Some("Fermière ambitieuse")),
                        face("Seasoned Cathar", Some("Cathare aguerrie")),
                    ],
                    cardmarket_id: None,
                    digital: false,
                },
                ScryfallBulkCard {
                    scryfall_id: "f4588570-bde4-4c2f-8469-81a3e15fb57b".parse().unwrap(),
                    set_code: "tmkm".to_string(),
                    set_name: "Murders at Karlov Manor Tokens".to_string(),
                    set_type: "token".to_string(),
                    collector_number: "12".to_string(),
                    language: "en".to_string(),
                    layout: "token".to_string(),
                    rarity: "common".to_string(),
                    name: "Spirit".to_string(),
                    printed_name: None,
                    faces: vec![],
                    cardmarket_id: Some(754218),
                    digital: false,
                },
            ]
        );
    }

    #[tokio::test]
    async fn stream_bulk_cards_fails_without_an_all_cards_file() {
        let server = MockServer::start().await;
        Mock::given(path("/bulk-data"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [{ "type": "oracle_cards", "jsonl_download_uri": "http://unused" }]
            })))
            .mount(&server)
            .await;

        let result = ScryfallCallerAdapter::new(server.uri(), 8)
            .stream_bulk_cards()
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn stream_bulk_cards_fails_when_the_download_fails() {
        let server = server_with_bulk(Vec::new()).await;
        server.reset().await;
        Mock::given(path("/bulk-data"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [{
                    "type": "all_cards",
                    "jsonl_download_uri": format!("{}/all-cards.jsonl.gz", server.uri())
                }]
            })))
            .mount(&server)
            .await;
        Mock::given(path("/all-cards.jsonl.gz"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;

        let result = ScryfallCallerAdapter::new(server.uri(), 8)
            .stream_bulk_cards()
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn stream_bulk_cards_ends_with_an_error_on_a_truncated_file() {
        let mut truncated = gzip(BULK_EXCERPT);
        truncated.truncate(truncated.len() / 2);
        let server = server_with_bulk(truncated).await;

        let items = read_bulk(&ScryfallCallerAdapter::new(server.uri(), 8))
            .await
            .unwrap();

        assert!(items.last().is_some_and(|item| item.is_err()));
    }

    #[tokio::test]
    async fn stream_bulk_cards_ends_with_an_error_on_a_malformed_line() {
        let server = server_with_bulk(gzip("{\"object\":\"card\"\nnot json\n")).await;

        let items = read_bulk(&ScryfallCallerAdapter::new(server.uri(), 8))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert!(items[0].is_err());
    }

    #[tokio::test]
    async fn a_bulk_reader_that_panics_ends_the_stream_with_an_error_rather_than_quietly() {
        let items: Vec<_> = bulk_card_stream(|_sender| panic!("reader failure"))
            .collect()
            .await;

        assert_eq!(items.len(), 1);
        assert!(items[0].is_err());
    }

    #[tokio::test]
    async fn a_bulk_reader_that_returns_ends_the_stream_without_error() {
        let items: Vec<_> = bulk_card_stream(|_sender| ()).collect().await;

        assert!(items.is_empty());
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
