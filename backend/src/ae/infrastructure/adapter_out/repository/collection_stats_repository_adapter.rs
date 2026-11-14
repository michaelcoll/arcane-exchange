use crate::application::error::AppError;
use crate::application::repository::CollectionStatsRepository;
use crate::domain::collection_stats::{BinderInfo, CollectionStats};
use crate::domain::price::Price;
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
    #[tracing::instrument(name = "collection_stats_repo.get_collection_stats", skip_all, fields(sentry.op = "db"))]
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

        let binders = sqlx::query!(
            r#"
            SELECT b.name AS "binder_name!", SUM(ce.quantity)::BIGINT AS "card_count!"
            FROM collection_entry ce
            JOIN binder b ON b.id = ce.binder_id
            WHERE ce.user_id = $1
            GROUP BY b.id, b.name
            ORDER BY SUM(ce.quantity) DESC, b.name
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
            binders: binders
                .into_iter()
                .map(|r| BinderInfo {
                    name: r.binder_name,
                    card_count: r.card_count as u64,
                })
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::adapter_out::repository::common_repository_tests::{
        insert_card, insert_card_without_cardmarket_id, insert_collection_entry,
        insert_collection_entry_with_binder, insert_price, insert_set, insert_user, refresh_view,
    };
    use crate::infrastructure::adapter_out::repository::entities::CardMarketPriceEntity;
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
        assert!(stats.binders.is_empty());
    }

    #[sqlx::test]
    async fn returns_correct_totals(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card_without_cardmarket_id(&pool, "TST", "1", "en", "Card A").await;
        insert_card_without_cardmarket_id(&pool, "TST", "2", "en", "Card B").await;
        insert_user(&pool, "user-1", "User1").await;
        insert_collection_entry(&pool, "TST", "1", "en", false, "user-1", 3, 100, Utc::now()).await;
        insert_collection_entry(&pool, "TST", "2", "en", false, "user-1", 2, 200, Utc::now()).await;

        let adapter = CollectionStatsRepositoryAdapter::new(pool);
        let result = adapter.get_collection_stats(&UserId::new("user-1")).await;
        assert!(result.is_ok());
        let stats = result.unwrap();
        assert_eq!(stats.total_cards, 5);
        assert_eq!(stats.unique_cards, 2);
    }

    #[sqlx::test]
    async fn a_card_nobody_owns_does_not_change_the_collection_value(pool: PgPool) {
        insert_set(&pool, "OWN").await;
        insert_card(&pool, "OWN", "1", "en", "Owned Card", 1).await;
        insert_set(&pool, "ORP").await;
        insert_card(&pool, "ORP", "1", "en", "Orphan Card", 2).await;
        insert_user(&pool, "user-1", "User1").await;
        insert_collection_entry(&pool, "OWN", "1", "en", false, "user-1", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        insert_price(&pool, CardMarketPriceEntity::simple(2, 99_999)).await;
        refresh_view(&pool).await;

        let stats = CollectionStatsRepositoryAdapter::new(pool)
            .get_collection_stats(&UserId::new("user-1"))
            .await
            .unwrap();

        assert_eq!(stats.total_cards, 1);
        assert_eq!(stats.unique_cards, 1);
        assert_eq!(stats.price_trend_max.value, Some(100));
    }

    #[sqlx::test]
    async fn does_not_return_other_users_cards(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card_without_cardmarket_id(&pool, "TST", "1", "en", "Card A").await;
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
        insert_card_without_cardmarket_id(&pool, "TST", "1", "en", "Card A").await;
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

    #[sqlx::test]
    async fn binders_are_ordered_by_card_count_descending(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card_without_cardmarket_id(&pool, "TST", "1", "en", "Card A").await;
        insert_card_without_cardmarket_id(&pool, "TST", "2", "en", "Card B").await;
        insert_user(&pool, "user-1", "User1").await;
        insert_collection_entry_with_binder(
            &pool,
            "TST",
            "1",
            "en",
            false,
            "user-1",
            4,
            100,
            Utc::now(),
            Some("Trade Binder"),
        )
        .await;
        insert_collection_entry_with_binder(
            &pool,
            "TST",
            "2",
            "en",
            false,
            "user-1",
            2,
            200,
            Utc::now(),
            Some("Bulk"),
        )
        .await;

        let adapter = CollectionStatsRepositoryAdapter::new(pool);
        let stats = adapter
            .get_collection_stats(&UserId::new("user-1"))
            .await
            .unwrap();

        assert_eq!(stats.binders.len(), 2);
        assert_eq!(stats.binders[0].name, "Trade Binder");
        assert_eq!(stats.binders[0].card_count, 4);
        assert_eq!(stats.binders[1].name, "Bulk");
        assert_eq!(stats.binders[1].card_count, 2);
    }

    #[sqlx::test]
    async fn entries_without_binder_are_excluded_from_binders(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card_without_cardmarket_id(&pool, "TST", "1", "en", "Card A").await;
        insert_user(&pool, "user-1", "User1").await;
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
            None,
        )
        .await;

        let adapter = CollectionStatsRepositoryAdapter::new(pool);
        let stats = adapter
            .get_collection_stats(&UserId::new("user-1"))
            .await
            .unwrap();

        assert!(stats.binders.is_empty());
        assert_eq!(stats.total_cards, 3);
        assert_eq!(stats.unique_cards, 1);
    }

    #[sqlx::test]
    async fn binders_do_not_leak_between_users(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card_without_cardmarket_id(&pool, "TST", "1", "en", "Card A").await;
        insert_user(&pool, "user-other", "UserOther").await;
        insert_collection_entry_with_binder(
            &pool,
            "TST",
            "1",
            "en",
            false,
            "user-other",
            5,
            100,
            Utc::now(),
            Some("Trade Binder"),
        )
        .await;

        let adapter = CollectionStatsRepositoryAdapter::new(pool);
        let stats = adapter
            .get_collection_stats(&UserId::new("user-1"))
            .await
            .unwrap();

        assert!(stats.binders.is_empty());
    }
}
