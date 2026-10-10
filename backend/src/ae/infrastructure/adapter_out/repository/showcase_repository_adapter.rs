use crate::application::error::AppError;
use crate::application::repository::ShowcaseRepository;
use crate::domain::showcase::{SHOWCASE_SIZE, ShowcaseCard};
use crate::infrastructure::adapter_out::repository::entities::ShowcaseCardEntity;
use async_trait::async_trait;
use sqlx::{Pool, Postgres};

pub struct ShowcaseRepositoryAdapter {
    pool: Pool<Postgres>,
}

impl ShowcaseRepositoryAdapter {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ShowcaseRepository for ShowcaseRepositoryAdapter {
    #[tracing::instrument(name = "showcase_repo.find_most_expensive", skip_all, fields(sentry.op = "db"))]
    async fn find_most_expensive(&self) -> Result<Vec<ShowcaseCard>, AppError> {
        let entities = sqlx::query_as!(
            ShowcaseCardEntity,
            // `mv_card_prices` has one row per card, finish and player, of every collection
            // whatever its visibility. The inner query keeps the most expensive copy of each set
            // and collector number; the languages of a card always tie, the price being the same
            // in every language, so the first language code stands for the card.
            r#"SELECT
                 best.set_code AS "set_code!",
                 best.collector_number AS "collector_number!",
                 best.language_code AS "language_code!",
                 best.image_source AS "image_source!",
                 best.image_has_back AS "image_has_back!"
               FROM (
                 SELECT DISTINCT ON (cp.set_code, cp.collector_number)
                   cp.set_code,
                   cp.collector_number,
                   cp.language_code,
                   cp.trend,
                   c.image_source,
                   c.image_has_back
                 FROM mv_card_prices cp
                 -- Read from `card` rather than the view, which is only refreshed with the
                 -- prices: an image shows up as soon as it is downloaded.
                 JOIN card c ON (c.set_code, c.collector_number, c.language_code) =
                                (cp.set_code, cp.collector_number, cp.language_code)
                 WHERE cp.trend IS NOT NULL
                   AND c.image_source IS NOT NULL
                 ORDER BY cp.set_code, cp.collector_number, cp.trend DESC, cp.language_code
               ) best
               ORDER BY best.trend DESC, best.set_code, best.collector_number
               LIMIT $1"#,
            i64::from(SHOWCASE_SIZE),
        )
        .fetch_all(&self.pool)
        .await?;

        entities
            .into_iter()
            .map(|entity| ShowcaseCard::try_from(entity).map_err(AppError::from))
            .collect()
    }
}
