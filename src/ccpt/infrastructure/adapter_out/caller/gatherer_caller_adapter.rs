use crate::application::caller::GathererCaller;
use crate::application::error::{AppError, InfraError};
use crate::domain::language_code::LanguageCode;
use crate::domain::set_name::SetCode;
use async_trait::async_trait;
use ratelimit::{Ratelimiter, TryWaitError};

pub struct GathererCallerAdapter {
    client: reqwest::Client,
    gatherer_base_url: String,
    ratelimiter: Ratelimiter,
}

impl GathererCallerAdapter {
    pub fn new(gatherer_base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("reqwest")
                .build()
                .unwrap(),
            gatherer_base_url: gatherer_base_url.into(),
            ratelimiter: Ratelimiter::builder(2).max_tokens(2).build().unwrap(),
        }
    }
}

/// Converts a card name into the dash-separated slug used in Gatherer URLs
/// (e.g. "Felothar, Dawn of the Abzan" -> "felothar-dawn-of-the-abzan").
/// Double-faced / split card names ("Fire // Ice") only keep the first face.
fn slugify(name: &str) -> String {
    let name = name.split("//").next().unwrap_or(name);
    let mut slug = String::new();
    let mut last_was_dash = false;
    for c in name.chars() {
        if c == '\'' || c == '.' {
            continue;
        }
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
            last_was_dash = false;
        } else if !last_was_dash {
            slug.push('-');
            last_was_dash = true;
        }
    }
    slug.trim_matches('-').to_string()
}

/// Extracts the Gatherer image id from an `og:image` URL, e.g.
/// `https://gatherer-static.wizards.com/Cards/medium/<ID>.webp` -> `<ID>`.
fn extract_image_id(image_url: &str) -> Option<String> {
    let file_name = image_url.rsplit('/').next()?;
    let id = file_name.strip_suffix(".webp").unwrap_or(file_name);
    if id.is_empty() {
        None
    } else {
        Some(id.to_string())
    }
}

#[async_trait]
impl GathererCaller for GathererCallerAdapter {
    async fn get_gatherer_id(
        &self,
        set_code: SetCode,
        collector_number: String,
        language_code: LanguageCode,
        name: String,
    ) -> Result<Option<String>, AppError> {
        let url = format!(
            "{}/{}/{}/{}/{}",
            self.gatherer_base_url,
            set_code,
            language_code.gatherer_locale(),
            collector_number,
            slugify(&name),
        );

        if let Err(err) = self.ratelimiter.try_wait() {
            match err {
                TryWaitError::Insufficient(duration) => {
                    tokio::time::sleep(duration).await;
                }
                TryWaitError::ExceedsCapacity => {
                    return Err(InfraError::CallError(
                        "Gatherer rate limiter overflow".to_string(),
                    )
                    .into());
                }
                _ => {
                    return Err(
                        InfraError::CallError("Gatherer rate limiter error".to_string()).into(),
                    );
                }
            }
        }

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            tracing::warn!(
                "Gatherer page not found for {url} (status {})",
                response.status()
            );
            return Ok(None);
        }

        let html = response.text().await?;

        let document = scraper::Html::parse_document(&html);
        let selector = scraper::Selector::parse(r#"meta[property="og:image"]"#)
            .map_err(|e| InfraError::CallError(format!("invalid selector: {e}")))?;

        let Some(content) = document
            .select(&selector)
            .next()
            .and_then(|el| el.value().attr("content"))
        else {
            tracing::warn!("Gatherer page for {url} has no og:image meta tag");
            return Ok(None);
        };

        let Some(id) = extract_image_id(content) else {
            tracing::warn!("Gatherer og:image URL for {url} has no extractable id: {content}");
            return Ok(None);
        };

        Ok(Some(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::path;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn og_image_html(content: &str) -> String {
        format!(r#"<html><head><meta property="og:image" content="{content}"/></head></html>"#)
    }

    #[test]
    fn slugify_replaces_spaces_with_dashes() {
        assert_eq!(slugify("Wanderbrine Preacher"), "wanderbrine-preacher");
    }

    #[test]
    fn slugify_strips_punctuation() {
        assert_eq!(
            slugify("Felothar, Dawn of the Abzan"),
            "felothar-dawn-of-the-abzan"
        );
    }

    #[test]
    fn slugify_removes_apostrophes_without_inserting_dash() {
        assert_eq!(
            slugify("Y'shtola, Night's Blessed"),
            "yshtola-nights-blessed"
        );
    }

    #[test]
    fn slugify_removes_dots_without_inserting_dashes() {
        assert_eq!(slugify("S.H.I.E.L.D. Helicarrier"), "shield-helicarrier");
    }

    #[test]
    fn slugify_keeps_only_first_face_of_double_faced_card() {
        assert_eq!(
            slugify("Delver of Secrets // Insectile Aberration"),
            "delver-of-secrets"
        );
    }

    #[test]
    fn slugify_keeps_only_first_face_without_surrounding_spaces() {
        assert_eq!(slugify("Fire//Ice"), "fire");
    }

    #[test]
    fn extract_image_id_strips_path_and_extension() {
        let url = "https://gatherer-static.wizards.com/Cards/medium/530325A982E8ADA7B336093036D69C306198A8A1B1E36D11DE2F9FAEA7186FE5.webp";
        assert_eq!(
            extract_image_id(url),
            Some("530325A982E8ADA7B336093036D69C306198A8A1B1E36D11DE2F9FAEA7186FE5".to_string())
        );
    }

    #[test]
    fn extract_image_id_returns_none_for_empty_file_name() {
        assert_eq!(extract_image_id("https://example.com/"), None);
    }

    #[tokio::test]
    async fn get_gatherer_id_returns_id_from_og_image() {
        let mock_server = MockServer::start().await;

        Mock::given(path("/ECL/en-us/41/wanderbrine-preacher"))
            .respond_with(ResponseTemplate::new(200).set_body_string(og_image_html(
                "https://gatherer-static.wizards.com/Cards/medium/ABC123.webp",
            )))
            .mount(&mock_server)
            .await;

        let adapter = GathererCallerAdapter::new(mock_server.uri());
        let result = adapter
            .get_gatherer_id(
                SetCode::new("ECL"),
                "41".to_string(),
                LanguageCode::EN,
                "Wanderbrine Preacher".to_string(),
            )
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some("ABC123".to_string()));
    }

    #[tokio::test]
    async fn get_gatherer_id_returns_none_on_404() {
        let mock_server = MockServer::start().await;

        Mock::given(path("/ECL/en-us/41/unknown-card"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let adapter = GathererCallerAdapter::new(mock_server.uri());
        let result = adapter
            .get_gatherer_id(
                SetCode::new("ECL"),
                "41".to_string(),
                LanguageCode::EN,
                "Unknown Card".to_string(),
            )
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[tokio::test]
    async fn get_gatherer_id_returns_none_when_meta_tag_missing() {
        let mock_server = MockServer::start().await;

        Mock::given(path("/ECL/en-us/41/wanderbrine-preacher"))
            .respond_with(ResponseTemplate::new(200).set_body_string("<html></html>"))
            .mount(&mock_server)
            .await;

        let adapter = GathererCallerAdapter::new(mock_server.uri());
        let result = adapter
            .get_gatherer_id(
                SetCode::new("ECL"),
                "41".to_string(),
                LanguageCode::EN,
                "Wanderbrine Preacher".to_string(),
            )
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[tokio::test]
    async fn get_gatherer_id_builds_url_with_correct_locale_per_language() {
        let mock_server = MockServer::start().await;

        Mock::given(path("/FDN/fr-fr/1/goblin-boarders"))
            .respond_with(ResponseTemplate::new(200).set_body_string(og_image_html(
                "https://gatherer-static.wizards.com/Cards/medium/XYZ789.webp",
            )))
            .mount(&mock_server)
            .await;

        let adapter = GathererCallerAdapter::new(mock_server.uri());
        let result = adapter
            .get_gatherer_id(
                SetCode::new("FDN"),
                "1".to_string(),
                LanguageCode::FR,
                "Goblin Boarders".to_string(),
            )
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some("XYZ789".to_string()));
    }
}
