use crate::application::caller::{GathererCaller, GathererLookup, GathererMiss};
use crate::application::error::{AppError, InfraError};
use crate::domain::card_image::CardImages;
use crate::domain::language_code::LanguageCode;
use crate::domain::set_name::SetCode;
use crate::infrastructure::adapter_out::caller::http;
use async_trait::async_trait;
use ratelimit::Ratelimiter;
use serde_json::Value;

pub struct GathererCallerAdapter {
    client: reqwest::Client,
    gatherer_base_url: String,
    ratelimiter: Ratelimiter,
}

impl GathererCallerAdapter {
    pub fn new(gatherer_base_url: impl Into<String>) -> Self {
        Self {
            client: http::client(),
            gatherer_base_url: gatherer_base_url.into(),
            ratelimiter: Ratelimiter::builder(2).max_tokens(2).build().unwrap(),
        }
    }

    async fn get_image(&self, url: &str) -> Result<Option<Vec<u8>>, AppError> {
        http::throttle(&self.ratelimiter, "Gatherer").await?;
        http::get_bytes_unless_not_found(&self.client, url).await
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

/// The image URLs a Gatherer card page shows.
#[derive(Debug, PartialEq, Eq)]
struct GathererPage {
    /// The `og:image`, the front face.
    front_url: String,
    back: Back,
}

#[derive(Debug, PartialEq, Eq)]
enum Back {
    /// Not a double-faced card (split, adventure and flip cards included).
    None,
    Url(String),
    /// A double-faced card whose back image the page does not give.
    Missing,
    /// The card is not in the page data, so whether it has a back is unknown. Treated as a
    /// technical error: guessing "single-faced" would store a front alone and never retry it.
    Unreadable,
}

/// Reads the front image from the `og:image` meta tag and, for a double-faced card, the back
/// image from the page's data, or `None` if the page has no `og:image`.
fn parse_page(html: &str) -> Option<GathererPage> {
    let document = scraper::Html::parse_document(html);
    let og_image = scraper::Selector::parse(r#"meta[property="og:image"]"#).unwrap();
    let front_url = document
        .select(&og_image)
        .next()?
        .value()
        .attr("content")?
        .to_string();

    let script = scraper::Selector::parse("script").unwrap();
    let payload: String = document
        .select(&script)
        .filter_map(|element| flight_chunk(&element.text().collect::<String>()))
        .collect();
    let back = find_back(&payload, &front_url);

    Some(GathererPage { front_url, back })
}

/// The page data is streamed by Next.js as `self.__next_f.push([1,"<chunk>"])` scripts; their
/// concatenated chunks form lines of `<hex id>:<JSON>`.
fn flight_chunk(script: &str) -> Option<String> {
    let arguments = script
        .trim()
        .strip_prefix("self.__next_f.push(")?
        .strip_suffix(')')?;
    match serde_json::from_str::<Value>(arguments).ok()? {
        Value::Array(items) if items.first() == Some(&Value::from(1)) => {
            items.get(1)?.as_str().map(str::to_string)
        }
        _ => None,
    }
}

/// Finds, in the page data, the card whose front image is `front_url` (the page also lists other
/// printings) and reads its back from its `compositeCard`.
fn find_back(payload: &str, front_url: &str) -> Back {
    for line in payload.lines() {
        let Some((id, json)) = line.split_once(':') else {
            continue;
        };
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_hexdigit()) {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(json) else {
            continue;
        };
        if let Some(card) = find_card(&value, front_url) {
            let composite = &card["compositeCard"];
            if composite["compositeType"] != "Doublefaced" {
                return Back::None;
            }
            return match composite["imageUrls"]["medium"].as_str() {
                Some(url) => Back::Url(url.to_string()),
                None => Back::Missing,
            };
        }
    }
    Back::Unreadable
}

fn find_card<'a>(value: &'a Value, front_url: &str) -> Option<&'a Value> {
    match value {
        Value::Object(object) => {
            if object
                .get("imageUrls")
                .and_then(|urls| urls.get("medium"))
                .is_some_and(|medium| medium == front_url)
            {
                return Some(value);
            }
            object
                .values()
                .find_map(|child| find_card(child, front_url))
        }
        Value::Array(items) => items.iter().find_map(|child| find_card(child, front_url)),
        _ => None,
    }
}

#[async_trait]
impl GathererCaller for GathererCallerAdapter {
    #[tracing::instrument(name = "gatherer.get_card", skip_all, fields(sentry.op = "http.client"))]
    async fn get_card(
        &self,
        set_code: SetCode,
        collector_number: String,
        language_code: LanguageCode,
        name: String,
    ) -> Result<GathererLookup, AppError> {
        let url = format!(
            "{}/{}/{}/{}/{}",
            self.gatherer_base_url,
            set_code,
            language_code.gatherer_locale(),
            collector_number,
            slugify(&name),
        );

        http::throttle(&self.ratelimiter, "Gatherer").await?;
        let Some(response) = http::get_unless_not_found(&self.client, &url).await? else {
            tracing::debug!("Gatherer page not found for {url}");
            return Ok(GathererLookup::NotFound(GathererMiss::NoPage));
        };
        let html = response.text().await?;

        let Some(page) = parse_page(&html) else {
            tracing::warn!("Gatherer page for {url} has no og:image meta tag");
            return Ok(GathererLookup::NotFound(GathererMiss::NoImageOnPage));
        };

        let back_url = match page.back {
            Back::None => None,
            Back::Url(back_url) => Some(back_url),
            Back::Missing => {
                tracing::debug!("Gatherer page for {url} has no back image");
                return Ok(GathererLookup::NotFound(GathererMiss::NoBackOnPage));
            }
            Back::Unreadable => {
                return Err(InfraError::CallError(format!(
                    "Gatherer page for {url} has no data for its og:image card"
                ))
                .into());
            }
        };

        let Some(front) = self.get_image(&page.front_url).await? else {
            return Ok(GathererLookup::NotFound(GathererMiss::NoImage));
        };
        let back = match back_url {
            None => None,
            Some(back_url) => match self.get_image(&back_url).await? {
                Some(back) => Some(back),
                None => return Ok(GathererLookup::NotFound(GathererMiss::NoBack)),
            },
        };

        Ok(GathererLookup::Found(CardImages { front, back }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::path;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// A card page as Gatherer serves it: the `og:image` meta tag, and the page data streamed as
    /// Next.js flight chunks, listing the card (optionally double-faced) and another printing.
    fn page_html(front_url: &str, composite: Option<Value>) -> String {
        let card = serde_json::json!({
            "setCode": "ISD",
            "cardNumber": "51",
            "imageUrls": { "small": "ignored", "medium": front_url },
            "compositeCard": composite,
        });
        let other_printing = serde_json::json!({
            "setCode": "INR",
            "imageUrls": { "medium": "https://other/front.webp" },
            "compositeCard": {
                "compositeType": "Doublefaced",
                "imageUrls": { "medium": "https://other/back.webp" }
            },
        });
        let line = format!(
            "1a:[\"$\",\"div\",null,{}]\n",
            serde_json::json!({ "printings": [other_printing, card] })
        );
        // The line is split across two chunks, as Next.js does.
        let (first, second) = line.split_at(line.len() / 2);
        let script = |chunk: &str| {
            format!(
                "<script>self.__next_f.push([1,{}])</script>",
                serde_json::to_string(chunk).unwrap()
            )
        };
        format!(
            r#"<html><head><meta property="og:image" content="{front_url}"/></head><body>
            <script>self.__next_f.push([0])</script>{}{}</body></html>"#,
            script("0:{\"P\":null}\n"),
            script(first) + &script(second),
        )
    }

    fn double_faced(back_url: Option<&str>) -> Option<Value> {
        let mut composite = serde_json::json!({ "compositeType": "Doublefaced" });
        if let Some(back_url) = back_url {
            composite["imageUrls"] = serde_json::json!({ "medium": back_url });
        }
        Some(composite)
    }

    async fn mount(server: &MockServer, url_path: &str, response: ResponseTemplate) {
        Mock::given(path(url_path))
            .respond_with(response)
            .mount(server)
            .await;
    }

    async fn get_card(server: &MockServer, language: LanguageCode) -> GathererLookup {
        GathererCallerAdapter::new(server.uri())
            .get_card(
                SetCode::new("ISD"),
                "51".to_string(),
                language,
                "Delver of Secrets // Insectile Aberration".to_string(),
            )
            .await
            .unwrap()
    }

    fn found(lookup: GathererLookup) -> CardImages {
        match lookup {
            GathererLookup::Found(images) => images,
            GathererLookup::NotFound(miss) => panic!("expected a card, Gatherer missed: {miss}"),
        }
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
    fn parse_page_reads_the_back_of_the_card_itself_not_of_another_printing() {
        let html = page_html(
            "https://g/Cards/medium/FRONT.webp",
            double_faced(Some("https://g/Cards/medium/BACK.webp")),
        );

        assert_eq!(
            parse_page(&html),
            Some(GathererPage {
                front_url: "https://g/Cards/medium/FRONT.webp".to_string(),
                back: Back::Url("https://g/Cards/medium/BACK.webp".to_string()),
            })
        );
    }

    #[test]
    fn parse_page_gives_no_back_to_a_flip_card() {
        let composite = Some(serde_json::json!({
            "compositeType": "Flip",
            "imageUrls": { "medium": "https://g/Cards/medium/FLIPPED.webp" }
        }));
        let html = page_html("https://g/Cards/medium/FRONT.webp", composite);

        assert_eq!(parse_page(&html).unwrap().back, Back::None);
    }

    #[test]
    fn parse_page_without_data_for_the_og_image_card_is_unreadable() {
        let html = page_html("https://g/Cards/medium/FRONT.webp", None).replace(
            r#"content="https://g/Cards/medium/FRONT.webp""#,
            r#"content="https://g/Cards/medium/ELSEWHERE.webp""#,
        );

        assert_eq!(parse_page(&html).unwrap().back, Back::Unreadable);
    }

    #[test]
    fn find_back_skips_the_lines_that_are_not_card_data() {
        let card = serde_json::json!({
            "imageUrls": { "medium": "FRONT" },
            "compositeCard": { "compositeType": "Doublefaced", "imageUrls": { "medium": "BACK" } },
        });
        let payload = format!("no separator\nHL:[\"not an id\"]\n1b:not json\n1c:{card}\n");

        assert_eq!(find_back(&payload, "FRONT"), Back::Url("BACK".to_string()));
    }

    #[tokio::test]
    async fn get_card_fails_when_the_page_data_cannot_be_read() {
        let server = MockServer::start().await;
        let front_url = format!("{}/Cards/medium/FRONT.webp", server.uri());
        mount(
            &server,
            "/ISD/fr-fr/51/delver-of-secrets",
            ResponseTemplate::new(200).set_body_string(format!(
                r#"<html><head><meta property="og:image" content="{front_url}"/></head></html>"#
            )),
        )
        .await;

        let result = GathererCallerAdapter::new(server.uri())
            .get_card(
                SetCode::new("ISD"),
                "51".to_string(),
                LanguageCode::FR,
                "Delver of Secrets".to_string(),
            )
            .await;

        assert!(result.is_err());
    }

    #[test]
    fn parse_page_without_og_image_is_none() {
        assert_eq!(parse_page("<html></html>"), None);
    }

    #[tokio::test]
    async fn get_card_downloads_the_front_byte_for_byte() {
        let server = MockServer::start().await;
        let front_url = format!("{}/Cards/medium/ABC123.webp", server.uri());
        mount(
            &server,
            "/ISD/fr-fr/51/delver-of-secrets",
            ResponseTemplate::new(200).set_body_string(page_html(&front_url, None)),
        )
        .await;
        mount(
            &server,
            "/Cards/medium/ABC123.webp",
            ResponseTemplate::new(200).set_body_bytes(b"front webp".to_vec()),
        )
        .await;

        let card = get_card(&server, LanguageCode::FR).await;

        assert_eq!(
            card,
            GathererLookup::Found(CardImages {
                front: b"front webp".to_vec(),
                back: None,
            })
        );
    }

    #[tokio::test]
    async fn get_card_downloads_the_back_of_a_double_faced_card() {
        let server = MockServer::start().await;
        let front_url = format!("{}/Cards/medium/FRONT.webp", server.uri());
        let back_url = format!("{}/Cards/medium/BACK.webp", server.uri());
        mount(
            &server,
            "/ISD/en-us/51/delver-of-secrets",
            ResponseTemplate::new(200)
                .set_body_string(page_html(&front_url, double_faced(Some(&back_url)))),
        )
        .await;
        mount(
            &server,
            "/Cards/medium/FRONT.webp",
            ResponseTemplate::new(200).set_body_bytes(b"front".to_vec()),
        )
        .await;
        mount(
            &server,
            "/Cards/medium/BACK.webp",
            ResponseTemplate::new(200).set_body_bytes(b"back".to_vec()),
        )
        .await;

        let images = found(get_card(&server, LanguageCode::EN).await);

        assert_eq!(
            images,
            CardImages {
                front: b"front".to_vec(),
                back: Some(b"back".to_vec()),
            }
        );
    }

    #[tokio::test]
    async fn get_card_returns_small_images_as_they_are() {
        use crate::domain::card_image::{test_images::webp_of_width, webp_width};

        let server = MockServer::start().await;
        let front_url = format!("{}/Cards/medium/SMALL.webp", server.uri());
        mount(
            &server,
            "/ISD/fr-fr/51/delver-of-secrets",
            ResponseTemplate::new(200).set_body_string(page_html(&front_url, None)),
        )
        .await;
        mount(
            &server,
            "/Cards/medium/SMALL.webp",
            ResponseTemplate::new(200).set_body_bytes(webp_of_width(200)),
        )
        .await;

        let images = found(get_card(&server, LanguageCode::FR).await);

        assert_eq!(webp_width(&images.front), Some(200));
    }

    #[tokio::test]
    async fn get_card_is_not_found_when_the_back_is_missing_from_the_page() {
        let server = MockServer::start().await;
        let front_url = format!("{}/Cards/medium/FRONT.webp", server.uri());
        mount(
            &server,
            "/ISD/fr-fr/51/delver-of-secrets",
            ResponseTemplate::new(200).set_body_string(page_html(&front_url, double_faced(None))),
        )
        .await;
        mount(
            &server,
            "/Cards/medium/FRONT.webp",
            ResponseTemplate::new(200).set_body_bytes(b"front".to_vec()),
        )
        .await;

        assert_eq!(
            get_card(&server, LanguageCode::FR).await,
            GathererLookup::NotFound(GathererMiss::NoBackOnPage)
        );
    }

    #[tokio::test]
    async fn get_card_is_not_found_when_the_back_image_is_404() {
        let server = MockServer::start().await;
        let front_url = format!("{}/Cards/medium/FRONT.webp", server.uri());
        let back_url = format!("{}/Cards/medium/BACK.webp", server.uri());
        mount(
            &server,
            "/ISD/fr-fr/51/delver-of-secrets",
            ResponseTemplate::new(200)
                .set_body_string(page_html(&front_url, double_faced(Some(&back_url)))),
        )
        .await;
        mount(
            &server,
            "/Cards/medium/FRONT.webp",
            ResponseTemplate::new(200).set_body_bytes(b"front".to_vec()),
        )
        .await;
        mount(
            &server,
            "/Cards/medium/BACK.webp",
            ResponseTemplate::new(404),
        )
        .await;

        assert_eq!(
            get_card(&server, LanguageCode::FR).await,
            GathererLookup::NotFound(GathererMiss::NoBack)
        );
    }

    #[tokio::test]
    async fn get_card_is_not_found_when_the_front_image_is_404() {
        let server = MockServer::start().await;
        let front_url = format!("{}/Cards/medium/FRONT.webp", server.uri());
        mount(
            &server,
            "/ISD/fr-fr/51/delver-of-secrets",
            ResponseTemplate::new(200).set_body_string(page_html(&front_url, None)),
        )
        .await;
        mount(
            &server,
            "/Cards/medium/FRONT.webp",
            ResponseTemplate::new(404),
        )
        .await;

        assert_eq!(
            get_card(&server, LanguageCode::FR).await,
            GathererLookup::NotFound(GathererMiss::NoImage)
        );
    }

    #[tokio::test]
    async fn get_card_is_not_found_on_a_404_page() {
        let server = MockServer::start().await;
        mount(
            &server,
            "/ISD/fr-fr/51/delver-of-secrets",
            ResponseTemplate::new(404),
        )
        .await;

        assert_eq!(
            get_card(&server, LanguageCode::FR).await,
            GathererLookup::NotFound(GathererMiss::NoPage)
        );
    }

    #[tokio::test]
    async fn get_card_is_not_found_when_the_meta_tag_is_missing() {
        let server = MockServer::start().await;
        mount(
            &server,
            "/ISD/fr-fr/51/delver-of-secrets",
            ResponseTemplate::new(200).set_body_string("<html></html>"),
        )
        .await;

        assert_eq!(
            get_card(&server, LanguageCode::FR).await,
            GathererLookup::NotFound(GathererMiss::NoImageOnPage)
        );
    }

    #[tokio::test]
    async fn get_card_fails_on_a_server_error() {
        let server = MockServer::start().await;
        mount(
            &server,
            "/ISD/fr-fr/51/delver-of-secrets",
            ResponseTemplate::new(503),
        )
        .await;

        let result = GathererCallerAdapter::new(server.uri())
            .get_card(
                SetCode::new("ISD"),
                "51".to_string(),
                LanguageCode::FR,
                "Delver of Secrets".to_string(),
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn get_card_fails_when_the_image_download_fails() {
        let server = MockServer::start().await;
        let front_url = format!("{}/Cards/medium/FRONT.webp", server.uri());
        mount(
            &server,
            "/ISD/fr-fr/51/delver-of-secrets",
            ResponseTemplate::new(200).set_body_string(page_html(&front_url, None)),
        )
        .await;
        mount(
            &server,
            "/Cards/medium/FRONT.webp",
            ResponseTemplate::new(500),
        )
        .await;

        let result = GathererCallerAdapter::new(server.uri())
            .get_card(
                SetCode::new("ISD"),
                "51".to_string(),
                LanguageCode::FR,
                "Delver of Secrets".to_string(),
            )
            .await;

        assert!(result.is_err());
    }
}
