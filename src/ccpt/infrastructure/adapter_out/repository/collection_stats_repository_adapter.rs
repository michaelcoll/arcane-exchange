use crate::application::error::AppError;
use crate::application::repository::CollectionStatsRepository;
use crate::domain::collection_stats::CollectionStats;
use crate::domain::price::Price;
use crate::domain::set_name::{SetCode, SetName};
use crate::domain::user::UserId;
use async_trait::async_trait;
use sqlx::{Pool, Postgres};

pub struct CollectionStatsRepositoryAdapter {
    pool: Pool<Postgres>,
}

impl CollectionStatsRepositoryAdapter {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CollectionStatsRepository for CollectionStatsRepositoryAdapter {
    async fn get_collection_stats(&self, user_id: &UserId) -> Result<CollectionStats, AppError> {
        let totals = sqlx::query!(
            r#"
            SELECT
                COALESCE(SUM(ce.quantity), 0)::BIGINT AS "total_cards!",
                COUNT(DISTINCT (ce.set_code, ce.collector_number, ce.language_code, ce.foil))::BIGINT AS "unique_cards!"
            FROM collection_entry ce
            WHERE ce.user_id = $1
            "#,
            user_id.as_str()
        )
        .fetch_one(&self.pool)
        .await?;

        let prices = sqlx::query!(
            r#"
            SELECT
                MIN(cp.trend)::INT AS price_trend_min,
                MAX(cp.trend)::INT AS price_trend_max
            FROM collection_entry ce
            LEFT JOIN mv_card_prices cp
                ON  cp.set_code         = ce.set_code
                AND cp.collector_number = ce.collector_number
                AND cp.language_code    = ce.language_code
                AND cp.foil             = ce.foil
                AND cp.user_id          = ce.user_id
            WHERE ce.user_id = $1
            "#,
            user_id.as_str()
        )
        .fetch_one(&self.pool)
        .await?;

        let sets = sqlx::query!(
            r#"
            SELECT DISTINCT sn.set_code, sn.name
            FROM collection_entry ce
            JOIN card c
                ON  c.set_code         = ce.set_code
                AND c.collector_number = ce.collector_number
                AND c.language_code    = ce.language_code
                AND c.foil             = ce.foil
            JOIN set_name sn ON sn.set_code = c.set_code
            WHERE ce.user_id = $1
            ORDER BY sn.name
            "#,
            user_id.as_str()
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(CollectionStats {
            total_cards: totals.total_cards as u64,
            unique_cards: totals.unique_cards as u64,
            price_trend_min: prices
                .price_trend_min
                .map(|v| Price::from_cents(v as u32))
                .unwrap_or_else(Price::empty),
            price_trend_max: prices
                .price_trend_max
                .map(|v| Price::from_cents(v as u32))
                .unwrap_or_else(Price::empty),
            sets: sets
                .into_iter()
                .map(|r| SetName::new(SetCode::new(r.set_code), r.name))
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::adapter_out::repository::common_repository_tests::{
        insert_card_without_cardmarket_id, insert_collection_entry,
        insert_collection_entry_with_binder, insert_set, insert_user,
    };
    use chrono::Utc;
    use sqlx::PgPool;

    #[sqlx::test]
    async fn returns_zeros_for_empty_collection(pool: PgPool) {
        let adapter = CollectionStatsRepositoryAdapter::new(pool);
        let result = adapter
            .get_collection_stats(&UserId::new("unknown-user"))
            .await;
        assert!(result.is_ok());
        let stats = result.unwrap();
        assert_eq!(stats.total_cards, 0);
        assert_eq!(stats.unique_cards, 0);
        assert!(stats.price_trend_min.value.is_none());
        assert!(stats.price_trend_max.value.is_none());
        assert!(stats.sets.is_empty());
    }

    #[sqlx::test]
    async fn returns_correct_totals(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card_without_cardmarket_id(&pool, "TST", "1", "en", false, "Card A").await;
        insert_card_without_cardmarket_id(&pool, "TST", "2", "en", false, "Card B").await;
        insert_user(&pool, "user-1", "User1").await;
        insert_collection_entry(&pool, "TST", "1", "en", false, "user-1", 3, 100, Utc::now()).await;
        insert_collection_entry(&pool, "TST", "2", "en", false, "user-1", 2, 200, Utc::now()).await;

        let adapter = CollectionStatsRepositoryAdapter::new(pool);
        let result = adapter.get_collection_stats(&UserId::new("user-1")).await;
        assert!(result.is_ok());
        let stats = result.unwrap();
        assert_eq!(stats.total_cards, 5);
        assert_eq!(stats.unique_cards, 2);
        assert_eq!(stats.sets.len(), 1);
        assert_eq!(stats.sets[0].name, "Set TST");
        assert_eq!(stats.sets[0].code.to_string(), "TST");
    }

    #[sqlx::test]
    async fn does_not_return_other_users_cards(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card_without_cardmarket_id(&pool, "TST", "1", "en", false, "Card A").await;
        insert_user(&pool, "user-other", "UserOther").await;
        insert_collection_entry(
            &pool,
            "TST",
            "1",
            "en",
            false,
            "user-other",
            10,
            100,
            Utc::now(),
        )
        .await;

        let adapter = CollectionStatsRepositoryAdapter::new(pool);
        let result = adapter.get_collection_stats(&UserId::new("user-1")).await;
        assert!(result.is_ok());
        let stats = result.unwrap();
        assert_eq!(stats.total_cards, 0);
        assert_eq!(stats.unique_cards, 0);
    }

    #[sqlx::test]
    async fn counts_card_split_across_binders_once(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card_without_cardmarket_id(&pool, "TST", "1", "en", false, "Card A").await;
        insert_user(&pool, "user-1", "User1").await;
        insert_collection_entry_with_binder(
            &pool,
            "TST",
            "1",
            "en",
            false,
            "user-1",
            2,
            100,
            Utc::now(),
            Some("Binder A"),
        )
        .await;
        insert_collection_entry_with_binder(
            &pool,
            "TST",
            "1",
            "en",
            false,
            "user-1",
            3,
            100,
            Utc::now(),
            Some("Binder B"),
        )
        .await;

        let adapter = CollectionStatsRepositoryAdapter::new(pool);
        let stats = adapter
            .get_collection_stats(&UserId::new("user-1"))
            .await
            .unwrap();

        assert_eq!(stats.unique_cards, 1);
        assert_eq!(stats.total_cards, 5);
    }
}
