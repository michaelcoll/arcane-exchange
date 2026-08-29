use crate::application::caller::EdhRecCaller;
use crate::application::error::{AppError, InfraError};
use crate::domain::card::CardInfo;
use crate::infrastructure::adapter_out::caller::dto::EdhRecCardInfo;
use async_trait::async_trait;
use chrono::{Duration, NaiveDateTime, Utc};
use std::option::Option;
use tokio::sync::RwLock;

struct BuildIdCache {
    id: Option<String>,
    last_updated: Option<NaiveDateTime>,
}

pub struct EdhRecCallerAdapter {
    client: reqwest::Client,
    edh_rec_base_url: String,
    cache: RwLock<BuildIdCache>,
}

impl EdhRecCallerAdapter {
    pub fn new(edh_rec_base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            edh_rec_base_url: edh_rec_base_url.into(),
            cache: RwLock::new(BuildIdCache {
                id: None,
                last_updated: None,
            }),
        }
    }

    async fn get_build_id(&self) -> Result<String, AppError> {
        let url = self.edh_rec_base_url.clone() + "/faq";
        println!("Fetching build ID from {}", url);

        let html = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| InfraError::CallError(format!("edhrec request error: {e}")))?
            .text()
            .await
            .map_err(|e| InfraError::CallError(format!("edhrec response read error: {e}")))?;

        let document = scraper::Html::parse_document(&html);
        let selector = scraper::Selector::parse(r#"script#__NEXT_DATA__"#)
            .map_err(|e| InfraError::CallError(format!("invalid selector: {e}")))?;

        let json_text = document
            .select(&selector)
            .next()
            .and_then(|el| el.text().next())
            .ok_or_else(|| {
                InfraError::CallError("unable to find __NEXT_DATA__ script".to_string())
            })?;

        let v: serde_json::Value = serde_json::from_str(json_text)
            .map_err(|e| InfraError::CallError(format!("__NEXT_DATA__ is not valid json: {e}")))?;

        let build_id = v
            .get("buildId")
            .and_then(|x| x.as_str())
            .ok_or_else(|| InfraError::CallError("buildId not found in __NEXT_DATA__".to_string()))?
            .to_string();

        println!("Build ID: {build_id}");

        Ok(build_id)
    }

    async fn update_build_id(&self) -> Result<(), AppError> {
        let now = Utc::now().naive_utc();

        {
            let cache = self.cache.read().await;
            if let Some(last) = cache.last_updated
                && (now - last) < Duration::hours(24)
            {
                return Ok(());
            }
        }

        let new_id = self.get_build_id().await?;

        {
            let mut cache = self.cache.write().await;
            cache.id = Some(new_id.clone());
            cache.last_updated = Some(now);
        }

        Ok(())
    }

    fn get_card_id_from_name(&self, name: &str) -> String {
        name.replace(' ', "-")
            .replace("'", "")
            .replace(",", "")
            .to_lowercase()
    }
}

#[async_trait]
impl EdhRecCaller for EdhRecCallerAdapter {
    async fn get_card_info(&self, card_name: String) -> Result<CardInfo, AppError> {
        self.update_build_id().await?;

        let url = format!(
            "{}/_next/data/{}/cards/{}.json",
            self.edh_rec_base_url,
            self.cache.read().await.id.as_ref().unwrap(),
            self.get_card_id_from_name(&card_name)
        );

        println!("Fetching cardinfo from {}", url);

        let card_info: EdhRecCardInfo = self.client.get(url).send().await?.json().await?;

        let edh_rec_card = card_info.page_props.data.container.json_dict.card;

        println!("edh rec card info: {:?}", edh_rec_card);

        Ok(CardInfo {
            inclusion: edh_rec_card.inclusion as u32,
            total_decks: edh_rec_card.potential_decks as u32,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::path;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn create_adapter(edh_rec_base_url: impl Into<String>) -> EdhRecCallerAdapter {
        EdhRecCallerAdapter::new(edh_rec_base_url)
    }

    #[tokio::test]
    async fn get_build_id_returns_valid_build_id() {
        let mock_server = MockServer::start().await;

        let html_response = r#"
        <html>
            <script id="__NEXT_DATA__">
            {"buildId":"abc123def456","other":"data"}
            </script>
        </html>
        "#;

        Mock::given(path("/faq"))
            .respond_with(ResponseTemplate::new(200).set_body_string(html_response))
            .mount(&mock_server)
            .await;

        let adapter = create_adapter(mock_server.uri());
        let result = adapter.get_build_id().await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "abc123def456");
    }

    #[tokio::test]
    async fn get_build_id_returns_error_when_script_not_found() {
        let mock_server = MockServer::start().await;

        let html_response = r#"<html><body>No script tag</body></html>"#;

        Mock::given(path("/faq"))
            .respond_with(ResponseTemplate::new(200).set_body_string(html_response))
            .mount(&mock_server)
            .await;

        let adapter = create_adapter(mock_server.uri());
        let result = adapter.get_build_id().await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Infra(InfraError::CallError(msg)) => assert!(msg.contains("unable to find")),
            _ => panic!("Expected CallError"),
        }
    }

    #[tokio::test]
    async fn get_build_id_returns_error_when_build_id_missing() {
        let mock_server = MockServer::start().await;

        let html_response = r#"
        <html>
            <script id="__NEXT_DATA__">
            {"other":"data"}
            </script>
        </html>
        "#;

        Mock::given(path("/faq"))
            .respond_with(ResponseTemplate::new(200).set_body_string(html_response))
            .mount(&mock_server)
            .await;

        let adapter = create_adapter(mock_server.uri());
        let result = adapter.get_build_id().await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Infra(InfraError::CallError(msg)) => {
                assert!(msg.contains("buildId not found"))
            }
            _ => panic!("Expected CallError"),
        }
    }

    #[tokio::test]
    async fn get_build_id_returns_error_on_http_failure() {
        let mock_server = MockServer::start().await;

        Mock::given(path("/faq"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        let adapter = create_adapter(mock_server.uri());
        let result = adapter.get_build_id().await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Infra(InfraError::CallError(_)) => (),
            _ => panic!("Expected CallError"),
        }
    }

    #[tokio::test]
    async fn get_build_id_returns_error_on_invalid_json() {
        let mock_server = MockServer::start().await;

        let html_response = r#"
        <html>
            <script id="__NEXT_DATA__">
            {invalid json
            </script>
        </html>
        "#;

        Mock::given(path("/faq"))
            .respond_with(ResponseTemplate::new(200).set_body_string(html_response))
            .mount(&mock_server)
            .await;

        let adapter = EdhRecCallerAdapter::new(mock_server.uri());
        let result = adapter.get_build_id().await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Infra(InfraError::CallError(msg)) => {
                assert!(msg.contains("is not valid json"))
            }
            _ => panic!("Expected CallError"),
        }
    }

    #[tokio::test]
    async fn update_build_id_fetches_and_caches_when_cache_empty() {
        let mock_server = MockServer::start().await;

        let html_response = r#"
        <html>
            <script id="__NEXT_DATA__">
            {"buildId":"fresh-build-id","other":"data"}
            </script>
        </html>
        "#;

        Mock::given(path("/faq"))
            .respond_with(ResponseTemplate::new(200).set_body_string(html_response))
            .mount(&mock_server)
            .await;

        let adapter = create_adapter(mock_server.uri());
        let result = adapter.update_build_id().await;

        assert!(result.is_ok());
        let cache = adapter.cache.read().await;
        assert_eq!(cache.id.as_ref().unwrap(), "fresh-build-id");
        assert!(cache.last_updated.is_some());
    }

    #[tokio::test]
    async fn update_build_id_does_not_fetch_when_cache_valid() {
        let mock_server = MockServer::start().await;

        let html_response = r#"
        <html>
            <script id="__NEXT_DATA__">
            {"buildId":"new-build-id","other":"data"}
            </script>
        </html>
        "#;

        Mock::given(path("/faq"))
            .respond_with(ResponseTemplate::new(200).set_body_string(html_response))
            .mount(&mock_server)
            .await;

        let adapter = create_adapter(mock_server.uri());

        {
            let mut cache = adapter.cache.write().await;
            cache.id = Some("cached-build-id".to_string());
            cache.last_updated = Some(Utc::now().naive_utc());
        }

        let result = adapter.update_build_id().await;

        assert!(result.is_ok());
        let cache = adapter.cache.read().await;
        assert_eq!(cache.id.as_ref().unwrap(), "cached-build-id");
    }

    #[tokio::test]
    async fn update_build_id_refetches_when_cache_expired() {
        let mock_server = MockServer::start().await;

        let html_response = r#"
        <html>
            <script id="__NEXT_DATA__">
            {"buildId":"refreshed-build-id","other":"data"}
            </script>
        </html>
        "#;

        Mock::given(path("/faq"))
            .respond_with(ResponseTemplate::new(200).set_body_string(html_response))
            .mount(&mock_server)
            .await;

        let adapter = create_adapter(mock_server.uri());

        {
            let mut cache = adapter.cache.write().await;
            cache.id = Some("old-build-id".to_string());
            cache.last_updated = Some(Utc::now().naive_utc() - Duration::hours(25));
        }

        let result = adapter.update_build_id().await;

        assert!(result.is_ok());
        let cache = adapter.cache.read().await;
        assert_eq!(cache.id.as_ref().unwrap(), "refreshed-build-id");
    }

    #[tokio::test]
    async fn update_build_id_returns_error_when_get_build_id_fails() {
        let mock_server = MockServer::start().await;

        Mock::given(path("/faq"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        let adapter = create_adapter(mock_server.uri());
        let result = adapter.update_build_id().await;

        assert!(result.is_err());
        let cache = adapter.cache.read().await;
        assert!(cache.id.is_none());
    }

    #[tokio::test]
    async fn update_build_id_updates_last_updated_timestamp() {
        let mock_server = MockServer::start().await;

        let html_response = r#"
        <html>
            <script id="__NEXT_DATA__">
            {"buildId":"test-build-id","other":"data"}
            </script>
        </html>
        "#;

        Mock::given(path("/faq"))
            .respond_with(ResponseTemplate::new(200).set_body_string(html_response))
            .mount(&mock_server)
            .await;

        let adapter = create_adapter(mock_server.uri());
        let before = Utc::now().naive_utc();

        let result = adapter.update_build_id().await;

        let after = Utc::now().naive_utc();
        assert!(result.is_ok());
        let cache = adapter.cache.read().await;
        let updated = cache.last_updated.unwrap();
        assert!(updated >= before && updated <= after);
    }

    #[test]
    fn get_card_id_from_name_replaces_spaces_with_hyphens() {
        let adapter = create_adapter("");
        let result = adapter.get_card_id_from_name("Jace the Mind Sculptor");
        assert_eq!(result, "jace-the-mind-sculptor");
    }

    #[test]
    fn get_card_id_from_name_replaces_colons_with_hyphens() {
        let adapter = create_adapter("");
        let result = adapter.get_card_id_from_name("Atraxa, Praetors' Voice");
        assert_eq!(result, "atraxa-praetors-voice");
    }

    #[test]
    fn get_card_id_from_name_replaces_spaces_and_colons() {
        let adapter = create_adapter("");
        let result = adapter.get_card_id_from_name("Y'shtola, Night's Blessed");
        assert_eq!(result, "yshtola-nights-blessed");
    }

    #[test]
    fn get_card_id_from_name_empty_string() {
        let adapter = create_adapter("");
        let result = adapter.get_card_id_from_name("");
        assert_eq!(result, "");
    }

    #[tokio::test]
    async fn get_card_info_returns_card_info_successfully() {
        let mock_server = MockServer::start().await;

        let build_id_response = r#"
        <html>
            <script id="__NEXT_DATA__">
            {"buildId":"test-build-id","other":"data"}
            </script>
        </html>
        "#;

        let card_info_response = r#"
        {
            "pageProps": {
                "data": {
                    "container": {
                        "json_dict": {
                            "card": {
                                "inclusion": 1500,
                                "potential_decks": 50000
                            }
                        }
                    }
                }
            }
        }
        "#;

        Mock::given(path("/faq"))
            .respond_with(ResponseTemplate::new(200).set_body_string(build_id_response))
            .mount(&mock_server)
            .await;

        Mock::given(path(
            "/_next/data/test-build-id/cards/jace-the-mind-sculptor.json",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(card_info_response))
        .mount(&mock_server)
        .await;

        let adapter = create_adapter(mock_server.uri());
        let card_name = "Jace the Mind Sculptor";
        let result = adapter.get_card_info(card_name.to_string()).await;

        assert!(result.is_ok());
        let card_info = result.unwrap();
        assert_eq!(card_info.inclusion, 1500);
        assert_eq!(card_info.total_decks, 50000);
    }

    #[tokio::test]
    async fn get_card_info_returns_error_when_build_id_update_fails() {
        let mock_server = MockServer::start().await;

        Mock::given(path("/faq"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        let adapter = create_adapter(mock_server.uri());
        let card_name = "Jace the Mind Sculptor";
        let result = adapter.get_card_info(card_name.to_string()).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Infra(InfraError::CallError(msg)) => {
                assert_eq!(msg, "unable to find __NEXT_DATA__ script");
            }
            _ => panic!("Expected CallError"),
        }
    }

    #[tokio::test]
    async fn get_card_info_returns_error_when_card_info_request_fails() {
        let mock_server = MockServer::start().await;

        let build_id_response = r#"
        <html>
            <script id="__NEXT_DATA__">
            {"buildId":"test-build-id","other":"data"}
            </script>
        </html>
        "#;

        Mock::given(path("/faq"))
            .respond_with(ResponseTemplate::new(200).set_body_string(build_id_response))
            .mount(&mock_server)
            .await;

        Mock::given(path(
            "/_next/data/test-build-id/cards/jace-the-mind-sculptor.json",
        ))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

        let adapter = create_adapter(mock_server.uri());
        let card_name = "Jace the Mind Sculptor";
        let result = adapter.get_card_info(card_name.to_string()).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Infra(InfraError::CallError(msg)) => {
                assert!(msg.contains("error decoding response body"))
            }
            _ => panic!("Expected CallError"),
        }
    }

    #[tokio::test]
    async fn get_card_info_returns_error_when_card_info_json_invalid() {
        let mock_server = MockServer::start().await;

        let build_id_response = r#"
        <html>
            <script id="__NEXT_DATA__">
            {"buildId":"test-build-id","other":"data"}
            </script>
        </html>
        "#;

        Mock::given(path("/faq"))
            .respond_with(ResponseTemplate::new(200).set_body_string(build_id_response))
            .mount(&mock_server)
            .await;

        Mock::given(path(
            "/_next/data/test-build-id/cards/jace-the-mind-sculptor.json",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string("{invalid json"))
        .mount(&mock_server)
        .await;

        let adapter = create_adapter(mock_server.uri());
        let card_name = "Jace the Mind Sculptor";
        let result = adapter.get_card_info(card_name.to_string()).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Infra(InfraError::CallError(msg)) => {
                assert!(msg.contains("error decoding response body"))
            }
            _ => panic!("Expected CallError"),
        }
    }

    #[tokio::test]
    async fn get_card_info_transforms_card_name_to_card_id() {
        let mock_server = MockServer::start().await;

        let build_id_response = r#"
        <html>
            <script id="__NEXT_DATA__">
            {"buildId":"test-build-id","other":"data"}
            </script>
        </html>
        "#;

        let card_info_response = r#"
        {
            "pageProps": {
                "data": {
                    "container": {
                        "json_dict": {
                            "card": {
                                "inclusion": 100,
                                "potential_decks": 1000
                            }
                        }
                    }
                }
            }
        }
        "#;

        Mock::given(path("/faq"))
            .respond_with(ResponseTemplate::new(200).set_body_string(build_id_response))
            .mount(&mock_server)
            .await;

        // Test avec un nom contenant des espaces et des apostrophes
        Mock::given(path("/_next/data/test-build-id/cards/test-card-name.json"))
            .respond_with(ResponseTemplate::new(200).set_body_string(card_info_response))
            .mount(&mock_server)
            .await;

        let adapter = create_adapter(mock_server.uri());
        let card_name = "Test Card Name";
        let result = adapter.get_card_info(card_name.to_string()).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn get_card_info_reuses_cached_build_id() {
        let mock_server = MockServer::start().await;

        let build_id_response = r#"
            <html>
                <script id="__NEXT_DATA__">
                {"buildId":"test-build-id","other":"data"}
                </script>
            </html>
            "#;

        let card_info_response = r#"
            {
                "pageProps": {
                    "data": {
                        "container": {
                            "json_dict": {
                                "card": {
                                    "inclusion": 100,
                                    "potential_decks": 1000
                                }
                            }
                        }
                    }
                }
            }
            "#;

        Mock::given(path("/faq"))
            .respond_with(ResponseTemplate::new(200).set_body_string(build_id_response))
            .expect(1)
            .mount(&mock_server)
            .await;

        Mock::given(path(
            "/_next/data/test-build-id/cards/jace-the-mind-sculptor.json",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(card_info_response))
        .mount(&mock_server)
        .await;

        Mock::given(path("/_next/data/test-build-id/cards/lightning-bolt.json"))
            .respond_with(ResponseTemplate::new(200).set_body_string(card_info_response))
            .mount(&mock_server)
            .await;

        let adapter = create_adapter(mock_server.uri());

        let card_name1 = "Jace the Mind Sculptor";
        let result1 = adapter.get_card_info(card_name1.to_string()).await;
        assert!(result1.is_ok());

        let card_name2 = "Lightning Bolt";
        let result2 = adapter.get_card_info(card_name2.to_string()).await;
        assert!(result2.is_ok());
    }

    #[tokio::test]
    async fn get_card_info_with_zero_inclusion_and_decks() {
        let mock_server = MockServer::start().await;

        let build_id_response = r#"
            <html>
                <script id="__NEXT_DATA__">
                {"buildId":"test-build-id","other":"data"}
                </script>
            </html>
            "#;

        let card_info_response = r#"
            {
                "pageProps": {
                    "data": {
                        "container": {
                            "json_dict": {
                                "card": {
                                    "inclusion": 0,
                                    "potential_decks": 0
                                }
                            }
                        }
                    }
                }
            }
            "#;

        Mock::given(path("/faq"))
            .respond_with(ResponseTemplate::new(200).set_body_string(build_id_response))
            .mount(&mock_server)
            .await;

        Mock::given(path("/_next/data/test-build-id/cards/test-card.json"))
            .respond_with(ResponseTemplate::new(200).set_body_string(card_info_response))
            .mount(&mock_server)
            .await;

        let adapter = create_adapter(mock_server.uri());
        let card_name = "Test Card";
        let result = adapter.get_card_info(card_name.to_string()).await;

        assert!(result.is_ok());
        let card_info = result.unwrap();
        assert_eq!(card_info.inclusion, 0);
        assert_eq!(card_info.total_decks, 0);
    }
}
