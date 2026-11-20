use crate::application::error::AppError;
use crate::application::repository::{InclusionRateLookup, InclusionRateRepository};
use crate::domain::card::CardId;
use crate::domain::inclusion_rate::InclusionRate;
use crate::infrastructure::adapter_out::repository::entities::{
    InclusionRateLookupEntity, deck_count_to_db,
};
use async_trait::async_trait;
use sqlx::{Pool, Postgres};

pub struct InclusionRateRepositoryAdapter {
    pool: Pool<Postgres>,
}

impl InclusionRateRepositoryAdapter {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl InclusionRateRepository for InclusionRateRepositoryAdapter {
    // A name is printed in many sets and languages: its smallest card stands for all of them.
    #[tracing::instrument(name = "inclusion_rate_repo.get_names_without_rate", skip_all, fields(sentry.op = "db"))]
    async fn get_names_without_rate(&self) -> Result<Vec<(CardId, InclusionRateLookup)>, AppError> {
        let entities = sqlx::query_as!(
            InclusionRateLookupEntity,
            "SELECT DISTINCT ON (c.name) c.set_code, c.collector_number, c.language_code, c.name
            FROM card c
            WHERE NOT EXISTS (SELECT 1 FROM edhrec_inclusion e WHERE e.card_name = c.name)
            ORDER BY c.name, c.set_code, c.collector_number, c.language_code"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(entities
            .into_iter()
            .map(<(CardId, InclusionRateLookup)>::try_from)
            .collect::<Result<_, _>>()?)
    }

    #[tracing::instrument(name = "inclusion_rate_repo.save", skip_all, fields(sentry.op = "db"))]
    async fn save(&self, card_name: String, rate: InclusionRate) -> Result<(), AppError> {
        sqlx::query!(
            "INSERT INTO edhrec_inclusion (card_name, num_decks, potential_decks, fetched_at)
            VALUES ($1, $2, $3, now())
            ON CONFLICT (card_name) DO UPDATE
                SET num_decks = EXCLUDED.num_decks,
                    potential_decks = EXCLUDED.potential_decks,
                    fetched_at = EXCLUDED.fetched_at",
            card_name,
            deck_count_to_db("num_decks", rate.num_decks)?,
            deck_count_to_db("potential_decks", rate.potential_decks)?,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::language_code::LanguageCode;
    use crate::infrastructure::adapter_out::repository::common_repository_tests::{
        insert_card, insert_set,
    };
    use sqlx::PgPool;

    fn rate(num_decks: u32, potential_decks: u32) -> InclusionRate {
        InclusionRate {
            num_decks,
            potential_decks,
        }
    }

    fn lookup(
        set_code: &str,
        collector_number: &str,
        card_name: &str,
    ) -> (CardId, InclusionRateLookup) {
        (
            CardId::new(set_code, collector_number, LanguageCode::EN),
            InclusionRateLookup {
                card_name: card_name.to_string(),
            },
        )
    }

    async fn stored_rate(pool: &PgPool, card_name: &str) -> Option<(i32, i32)> {
        sqlx::query_as::<_, (i32, i32)>(
            "SELECT num_decks, potential_decks FROM edhrec_inclusion WHERE card_name = $1",
        )
        .bind(card_name)
        .fetch_optional(pool)
        .await
        .unwrap()
    }

    #[sqlx::test]
    async fn names_without_rate_has_one_card_per_name_whatever_its_printings(pool: PgPool) {
        insert_set(&pool, "DSK").await;
        insert_card(&pool, "FDN", "12", "EN", "Sol Ring", 1).await;
        insert_card(&pool, "FDN", "12", "FR", "Sol Ring", 1).await;
        insert_card(&pool, "DSK", "7", "EN", "Sol Ring", 1).await;
        insert_card(&pool, "FDN", "3", "EN", "Counterspell", 2).await;

        let names = InclusionRateRepositoryAdapter::new(pool)
            .get_names_without_rate()
            .await
            .unwrap();

        assert_eq!(
            names,
            vec![
                lookup("FDN", "3", "Counterspell"),
                lookup("DSK", "7", "Sol Ring"),
            ]
        );
    }

    #[sqlx::test]
    async fn names_without_rate_skips_the_names_that_have_one(pool: PgPool) {
        insert_card(&pool, "FDN", "12", "EN", "Sol Ring", 1).await;
        insert_card(&pool, "FDN", "3", "EN", "Counterspell", 2).await;
        let repository = InclusionRateRepositoryAdapter::new(pool);
        repository
            .save("Sol Ring".to_string(), rate(10, 100))
            .await
            .unwrap();

        let names = repository.get_names_without_rate().await.unwrap();

        assert_eq!(names, vec![lookup("FDN", "3", "Counterspell")]);
    }

    #[sqlx::test]
    async fn save_stores_the_raw_counts_by_name(pool: PgPool) {
        InclusionRateRepositoryAdapter::new(pool.clone())
            .save("Sol Ring".to_string(), rate(1_500, 50_000))
            .await
            .unwrap();

        assert_eq!(stored_rate(&pool, "Sol Ring").await, Some((1_500, 50_000)));
    }

    #[sqlx::test]
    async fn save_replaces_the_previous_rate(pool: PgPool) {
        let repository = InclusionRateRepositoryAdapter::new(pool.clone());
        repository
            .save("Sol Ring".to_string(), rate(10, 100))
            .await
            .unwrap();

        repository
            .save("Sol Ring".to_string(), rate(20, 200))
            .await
            .unwrap();

        assert_eq!(stored_rate(&pool, "Sol Ring").await, Some((20, 200)));
    }
}
