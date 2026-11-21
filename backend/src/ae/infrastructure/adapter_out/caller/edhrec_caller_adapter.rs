use crate::application::caller::EdhRecCaller;
use crate::application::error::{AppError, InfraError};
use crate::domain::inclusion_rate::InclusionRate;
use crate::infrastructure::adapter_out::caller::dto::{EdhRecCardPage, EdhRecPage};
use crate::infrastructure::adapter_out::caller::http;
use async_trait::async_trait;
use reqwest::StatusCode;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::Instant;

/// The separator between the faces of a double-faced card's name.
const FACE_SEPARATOR: &str = " // ";

/// Reads the inclusion rate on EDHREC's public JSON pages, one request per card, never closer
/// than `min_interval` to the previous one: EDHREC is free and has no official API. After a 429
/// or a 503 the interval becomes `backoff_interval`, for the rest of the process.
pub struct EdhRecCallerAdapter {
    client: reqwest::Client,
    edh_rec_base_url: String,
    backoff_interval: Duration,
    schedule: Mutex<Schedule>,
}

struct Schedule {
    interval: Duration,
    /// When the next request may start.
    next_request_at: Instant,
}

impl EdhRecCallerAdapter {
    pub fn new(
        edh_rec_base_url: impl Into<String>,
        min_interval: Duration,
        backoff_interval: Duration,
    ) -> Self {
        Self {
            client: http::client(),
            edh_rec_base_url: edh_rec_base_url.into(),
            backoff_interval,
            schedule: Mutex::new(Schedule {
                interval: min_interval,
                next_request_at: Instant::now(),
            }),
        }
    }

    /// Waits for this request's turn. The lock is held while waiting, so concurrent callers are
    /// served one interval apart.
    async fn wait_for_turn(&self) {
        let mut schedule = self.schedule.lock().await;
        tokio::time::sleep_until(schedule.next_request_at).await;
        schedule.next_request_at = Instant::now() + schedule.interval;
    }

    /// EDHREC refuses to serve us at this pace: slows every following request down.
    async fn back_off(&self, status: StatusCode, url: &str) {
        let mut schedule = self.schedule.lock().await;
        schedule.interval = self.backoff_interval;
        schedule.next_request_at = Instant::now() + self.backoff_interval;
        tracing::warn!(
            "EDHREC answered {status} on {url}: backing off to one request every {:?}.",
            self.backoff_interval
        );
    }
}

/// The pages EDHREC may give a card, in the order to try them. A card with several faces is the
/// page of its front face, except split cards ("Fire // Ice"), which EDHREC names after their
/// full name.
fn card_slugs(card_name: &str) -> Vec<String> {
    match card_name.split_once(FACE_SEPARATOR) {
        Some((front_face, _)) => vec![
            card_slug(front_face),
            card_slug(&card_name.replace(FACE_SEPARATOR, " ")),
        ],
        None => vec![card_slug(card_name)],
    }
}

/// The page name EDHREC gives a card or a face: lowercase, words joined by hyphens, accents
/// folded and other punctuation dropped.
fn card_slug(name: &str) -> String {
    let mut slug = String::new();
    for c in name.chars().flat_map(char::to_lowercase) {
        match c {
            'a'..='z' | '0'..='9' => slug.push(c),
            ' ' | '-' => slug.push('-'),
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => slug.push('a'),
            'æ' => slug.push_str("ae"),
            'ç' => slug.push('c'),
            'è' | 'é' | 'ê' | 'ë' => slug.push('e'),
            'ì' | 'í' | 'î' | 'ï' => slug.push('i'),
            'ñ' => slug.push('n'),
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' => slug.push('o'),
            'ù' | 'ú' | 'û' | 'ü' => slug.push('u'),
            _ => {}
        }
    }
    slug
}

#[async_trait]
impl EdhRecCaller for EdhRecCallerAdapter {
    #[tracing::instrument(name = "edhrec.get_inclusion_rate", skip_all, fields(sentry.op = "http.client"))]
    async fn get_inclusion_rate(
        &self,
        card_name: String,
    ) -> Result<Option<InclusionRate>, AppError> {
        for slug in card_slugs(&card_name) {
            let url = format!("{}/pages/cards/{slug}.json", self.edh_rec_base_url);
            let Some(page) = self.get_page::<EdhRecPage>(&url, &card_name).await? else {
                continue;
            };
            let card_page = match page {
                EdhRecPage::Card(card_page) => card_page,
                // A single redirect is followed: the page it points to is a card page or an error.
                EdhRecPage::Redirect { redirect } => {
                    let url = format!("{}/pages{redirect}.json", self.edh_rec_base_url);
                    match self.get_page::<EdhRecCardPage>(&url, &card_name).await? {
                        Some(card_page) => card_page,
                        None => continue,
                    }
                }
            };
            let card = card_page.container.json_dict.card;
            return Ok(Some(InclusionRate {
                num_decks: card.num_decks,
                potential_decks: card.potential_decks,
            }));
        }
        Ok(None)
    }
}

impl EdhRecCallerAdapter {
    /// Waits for its turn, then reads the page at `url`; `None` when EDHREC does not know it.
    async fn get_page<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        card_name: &str,
    ) -> Result<Option<T>, AppError> {
        self.wait_for_turn().await;
        let response = self.client.get(url).send().await?;
        let status = response.status();
        if matches!(
            status,
            StatusCode::TOO_MANY_REQUESTS | StatusCode::SERVICE_UNAVAILABLE
        ) {
            self.back_off(status, url).await;
        }
        let Some(response) = http::check_status(url, response)? else {
            return Ok(None);
        };
        let page = response.json().await.map_err(|e| {
            InfraError::CallError(format!("unexpected EDHREC page for '{card_name}': {e}"))
        })?;
        Ok(Some(page))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use wiremock::matchers::path;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn create_adapter(edh_rec_base_url: impl Into<String>) -> EdhRecCallerAdapter {
        EdhRecCallerAdapter::new(edh_rec_base_url, Duration::ZERO, Duration::ZERO)
    }

    fn page(num_decks: u32, potential_decks: u32) -> String {
        format!(
            r#"{{"container": {{"json_dict": {{"card": {{
                "name": "Sol Ring", "num_decks": {num_decks}, "potential_decks": {potential_decks}
            }}}}}}}}"#
        )
    }

    async fn serve(server: &MockServer, page_path: &str, response: ResponseTemplate) {
        Mock::given(path(page_path))
            .respond_with(response)
            .mount(server)
            .await;
    }

    #[tokio::test]
    async fn a_card_page_gives_the_raw_deck_counts() {
        let server = MockServer::start().await;
        serve(
            &server,
            "/pages/cards/sol-ring.json",
            ResponseTemplate::new(200).set_body_string(page(1_500, 50_000)),
        )
        .await;

        let rate = create_adapter(server.uri())
            .get_inclusion_rate("Sol Ring".to_string())
            .await
            .unwrap();

        assert_eq!(
            rate,
            Some(InclusionRate {
                num_decks: 1_500,
                potential_decks: 50_000
            })
        );
    }

    #[tokio::test]
    async fn an_unknown_card_page_is_not_found() {
        let server = MockServer::start().await;
        serve(
            &server,
            "/pages/cards/sol-ring.json",
            ResponseTemplate::new(404),
        )
        .await;

        let rate = create_adapter(server.uri())
            .get_inclusion_rate("Sol Ring".to_string())
            .await
            .unwrap();

        assert_eq!(rate, None);
    }

    #[tokio::test]
    async fn a_server_error_is_an_error() {
        let server = MockServer::start().await;
        serve(
            &server,
            "/pages/cards/sol-ring.json",
            ResponseTemplate::new(503),
        )
        .await;

        let result = create_adapter(server.uri())
            .get_inclusion_rate("Sol Ring".to_string())
            .await;

        assert!(matches!(
            result,
            Err(AppError::Infra(InfraError::CallError(_)))
        ));
    }

    #[tokio::test]
    async fn an_unexpected_page_is_an_error() {
        let server = MockServer::start().await;
        serve(
            &server,
            "/pages/cards/sol-ring.json",
            ResponseTemplate::new(200).set_body_string(r#"{"pageProps": {}}"#),
        )
        .await;

        let result = create_adapter(server.uri())
            .get_inclusion_rate("Sol Ring".to_string())
            .await;

        assert!(matches!(
            result,
            Err(AppError::Infra(InfraError::CallError(_)))
        ));
    }

    fn redirect(slug: &str) -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_string(format!(r#"{{"redirect": "/cards/{slug}"}}"#))
    }

    #[tokio::test]
    async fn a_double_faced_card_whose_front_face_page_redirects_is_read_on_the_target_page() {
        let server = MockServer::start().await;
        serve(
            &server,
            "/pages/cards/restricted-office.json",
            redirect("restricted-office-lecture-hall"),
        )
        .await;
        serve(
            &server,
            "/pages/cards/restricted-office-lecture-hall.json",
            ResponseTemplate::new(200).set_body_string(page(7, 70)),
        )
        .await;

        let rate = create_adapter(server.uri())
            .get_inclusion_rate("Restricted Office // Lecture Hall".to_string())
            .await
            .unwrap();

        assert_eq!(rate.map(|r| r.num_decks), Some(7));
    }

    #[tokio::test]
    async fn a_redirect_on_a_redirected_page_is_an_error() {
        let server = MockServer::start().await;
        serve(&server, "/pages/cards/a.json", redirect("b")).await;
        serve(&server, "/pages/cards/b.json", redirect("c")).await;

        let result = create_adapter(server.uri())
            .get_inclusion_rate("A".to_string())
            .await;

        assert!(matches!(
            result,
            Err(AppError::Infra(InfraError::CallError(_)))
        ));
    }

    #[tokio::test]
    async fn a_redirect_to_an_unknown_page_is_not_found() {
        let server = MockServer::start().await;
        serve(&server, "/pages/cards/a.json", redirect("b")).await;
        serve(&server, "/pages/cards/b.json", ResponseTemplate::new(404)).await;

        let rate = create_adapter(server.uri())
            .get_inclusion_rate("A".to_string())
            .await
            .unwrap();

        assert_eq!(rate, None);
    }

    #[tokio::test]
    async fn a_redirect_costs_one_interval() {
        let server = MockServer::start().await;
        serve(&server, "/pages/cards/a.json", redirect("b")).await;
        serve(
            &server,
            "/pages/cards/b.json",
            ResponseTemplate::new(200).set_body_string(page(1, 2)),
        )
        .await;
        let adapter = EdhRecCallerAdapter::new(
            server.uri(),
            Duration::from_millis(300),
            Duration::from_secs(5),
        );

        let started = std::time::Instant::now();
        adapter.get_inclusion_rate("A".to_string()).await.unwrap();

        assert!(started.elapsed() >= Duration::from_millis(300));
    }

    #[tokio::test]
    async fn a_split_card_whose_front_face_page_redirects_is_read_on_its_full_name_page() {
        let server = MockServer::start().await;
        serve(&server, "/pages/cards/fire.json", redirect("fire-ice")).await;
        serve(
            &server,
            "/pages/cards/fire-ice.json",
            ResponseTemplate::new(200).set_body_string(page(10, 100)),
        )
        .await;

        let rate = create_adapter(server.uri())
            .get_inclusion_rate("Fire // Ice".to_string())
            .await
            .unwrap();

        assert_eq!(rate.map(|r| r.num_decks), Some(10));
    }

    #[tokio::test]
    async fn a_double_faced_card_is_read_on_its_front_face_page() {
        let server = MockServer::start().await;
        serve(
            &server,
            "/pages/cards/delver-of-secrets.json",
            ResponseTemplate::new(200).set_body_string(page(10, 100)),
        )
        .await;

        let rate = create_adapter(server.uri())
            .get_inclusion_rate("Delver of Secrets // Insectile Aberration".to_string())
            .await
            .unwrap();

        assert_eq!(rate.map(|r| r.num_decks), Some(10));
    }

    #[tokio::test]
    async fn a_split_card_without_a_front_face_page_is_read_on_its_full_name_page() {
        let server = MockServer::start().await;
        serve(
            &server,
            "/pages/cards/fire.json",
            ResponseTemplate::new(404),
        )
        .await;
        serve(
            &server,
            "/pages/cards/fire-ice.json",
            ResponseTemplate::new(200).set_body_string(page(10, 100)),
        )
        .await;

        let rate = create_adapter(server.uri())
            .get_inclusion_rate("Fire // Ice".to_string())
            .await
            .unwrap();

        assert_eq!(rate.map(|r| r.num_decks), Some(10));
    }

    #[tokio::test]
    async fn a_double_faced_card_unknown_on_both_pages_is_not_found() {
        let server = MockServer::start().await;
        serve(
            &server,
            "/pages/cards/fire.json",
            ResponseTemplate::new(404),
        )
        .await;
        serve(
            &server,
            "/pages/cards/fire-ice.json",
            ResponseTemplate::new(404),
        )
        .await;

        let rate = create_adapter(server.uri())
            .get_inclusion_rate("Fire // Ice".to_string())
            .await
            .unwrap();

        assert_eq!(rate, None);
    }

    #[tokio::test]
    async fn the_full_name_page_is_requested_one_interval_after_the_front_face_page() {
        let server = MockServer::start().await;
        serve(
            &server,
            "/pages/cards/fire.json",
            ResponseTemplate::new(404),
        )
        .await;
        serve(
            &server,
            "/pages/cards/fire-ice.json",
            ResponseTemplate::new(200).set_body_string(page(1, 2)),
        )
        .await;
        let adapter = EdhRecCallerAdapter::new(
            server.uri(),
            Duration::from_millis(300),
            Duration::from_secs(5),
        );

        let started = std::time::Instant::now();
        adapter
            .get_inclusion_rate("Fire // Ice".to_string())
            .await
            .unwrap();

        assert!(started.elapsed() >= Duration::from_millis(300));
    }

    #[tokio::test]
    async fn requests_are_spaced_by_the_minimum_interval() {
        let server = MockServer::start().await;
        serve(
            &server,
            "/pages/cards/sol-ring.json",
            ResponseTemplate::new(200).set_body_string(page(1, 2)),
        )
        .await;
        let adapter = EdhRecCallerAdapter::new(
            server.uri(),
            Duration::from_millis(300),
            Duration::from_secs(5),
        );

        let started = std::time::Instant::now();
        for _ in 0..3 {
            adapter
                .get_inclusion_rate("Sol Ring".to_string())
                .await
                .unwrap();
        }

        // The first request is not delayed: two intervals separate the three.
        assert!(started.elapsed() >= Duration::from_millis(600));
    }

    #[tokio::test]
    async fn requests_from_concurrent_callers_are_spaced_by_the_minimum_interval() {
        let server = MockServer::start().await;
        serve(
            &server,
            "/pages/cards/sol-ring.json",
            ResponseTemplate::new(200).set_body_string(page(1, 2)),
        )
        .await;
        let adapter = EdhRecCallerAdapter::new(
            server.uri(),
            Duration::from_millis(200),
            Duration::from_secs(5),
        );

        let starts = Arc::new(std::sync::Mutex::new(Vec::new()));
        let adapter = &adapter;
        let calls = (0..4).map(|_| {
            let starts = starts.clone();
            async move {
                let result = adapter.get_inclusion_rate("Sol Ring".to_string()).await;
                starts.lock().unwrap().push(std::time::Instant::now());
                result
            }
        });
        for result in futures::future::join_all(calls).await {
            result.unwrap();
        }

        // Completions follow the starts in order; a response takes about the same time each.
        let starts = starts.lock().unwrap();
        for pair in starts.windows(2) {
            assert!(pair[1] - pair[0] >= Duration::from_millis(150));
        }
    }

    async fn assert_status_slows_the_next_request_down(status: u16) {
        let server = MockServer::start().await;
        serve(
            &server,
            "/pages/cards/sol-ring.json",
            ResponseTemplate::new(status),
        )
        .await;
        let adapter =
            EdhRecCallerAdapter::new(server.uri(), Duration::ZERO, Duration::from_millis(300));

        let result = adapter.get_inclusion_rate("Sol Ring".to_string()).await;
        assert!(matches!(
            result,
            Err(AppError::Infra(InfraError::CallError(_)))
        ));

        let started = std::time::Instant::now();
        let _ = adapter.get_inclusion_rate("Sol Ring".to_string()).await;
        assert!(started.elapsed() >= Duration::from_millis(300));

        // The back-off is kept: the next request is spaced by it too.
        let started = std::time::Instant::now();
        let _ = adapter.get_inclusion_rate("Sol Ring".to_string()).await;
        assert!(started.elapsed() >= Duration::from_millis(300));
    }

    #[tokio::test]
    async fn a_429_is_an_error_and_slows_the_next_requests_down() {
        assert_status_slows_the_next_request_down(429).await;
    }

    #[tokio::test]
    async fn a_503_is_an_error_and_slows_the_next_requests_down() {
        assert_status_slows_the_next_request_down(503).await;
    }

    #[tokio::test]
    async fn a_500_does_not_slow_the_next_requests_down() {
        let server = MockServer::start().await;
        serve(
            &server,
            "/pages/cards/sol-ring.json",
            ResponseTemplate::new(500),
        )
        .await;
        let adapter =
            EdhRecCallerAdapter::new(server.uri(), Duration::ZERO, Duration::from_secs(5));

        let _ = adapter.get_inclusion_rate("Sol Ring".to_string()).await;

        let started = std::time::Instant::now();
        let _ = adapter.get_inclusion_rate("Sol Ring".to_string()).await;
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn the_slug_drops_commas_and_apostrophes_and_joins_words_with_hyphens() {
        assert_eq!(
            card_slug("Atraxa, Praetors' Voice"),
            "atraxa-praetors-voice"
        );
    }

    #[test]
    fn the_slug_folds_accents() {
        assert_eq!(card_slug("Lim-Dûl's Vault"), "lim-duls-vault");
        assert_eq!(card_slug("Jötun Grunt"), "jotun-grunt");
    }

    #[test]
    fn a_single_faced_card_has_one_slug() {
        assert_eq!(card_slugs("Sol Ring"), vec!["sol-ring"]);
    }

    #[test]
    fn a_double_faced_card_has_its_front_face_slug_then_its_full_name_slug() {
        assert_eq!(
            card_slugs("Fire // Ice"),
            vec!["fire".to_string(), "fire-ice".to_string()]
        );
        assert_eq!(
            card_slugs("Delver of Secrets // Insectile Aberration"),
            vec![
                "delver-of-secrets".to_string(),
                "delver-of-secrets-insectile-aberration".to_string()
            ]
        );
    }
}
