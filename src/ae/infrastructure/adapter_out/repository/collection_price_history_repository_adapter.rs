use crate::application::error::AppError;
use crate::application::repository::CollectionPriceHistoryRepository;
use crate::domain::price::PriceHistoryEntry;
use crate::domain::user::{User, UserId};
use crate::infrastructure::adapter_out::repository::entities::CollectionPriceHistoryEntity;
use async_trait::async_trait;
use chrono::NaiveDate;
use sqlx::{Pool, Postgres};

pub struct CollectionPriceHistoryRepositoryAdapter {
    pool: Pool<Postgres>,
}

impl CollectionPriceHistoryRepositoryAdapter {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CollectionPriceHistoryRepository for CollectionPriceHistoryRepositoryAdapter {
    #[tracing::instrument(name = "collection_price_history_repo.get_date_and_user_to_update", skip_all, fields(sentry.op = "db"))]
    async fn get_date_and_user_to_update(&self) -> Result<Vec<(NaiveDate, User)>, AppError> {
        let rows = sqlx::query!(
            r#"SELECT dates.date, users.user_id
                FROM (SELECT DISTINCT user_id FROM collection_entry) AS users
                         CROSS JOIN (SELECT DISTINCT date FROM cardmarket_price) AS dates"#
        )
        .fetch_all(&self.pool)
        .await?;

        let result = rows
            .into_iter()
            .map(|row| (row.date, User::from_id(UserId::new(row.user_id))))
            .collect();

        Ok(result)
    }

    #[tracing::instrument(name = "collection_price_history_repo.update_for_date_and_user", skip_all, fields(sentry.op = "db"))]
    async fn update_for_date_and_user(&self, date: NaiveDate, user: User) -> Result<(), AppError> {
        sqlx::query!(
            r#"INSERT INTO collection_price_history (date, user_id, low, trend, avg)
                SELECT prices.date,
                       prices.user_id,
                       SUM(prices.low)   AS low,
                       SUM(prices.trend) AS trend,
                       SUM(prices.avg)   AS avg

                FROM (SELECT ce.user_id,
                             ce.added_at,
                             cmp.date,
                             CASE WHEN c.foil THEN cmp.low_foil ELSE cmp.low END * ce.quantity     AS low,
                             CASE WHEN c.foil THEN cmp.avg_foil ELSE cmp.avg END * ce.quantity     AS avg,
                             CASE WHEN c.foil THEN cmp.trend_foil ELSE cmp.trend END * ce.quantity AS trend
                       FROM card c
                               JOIN collection_entry ce
                                    ON c.set_code = ce.set_code AND c.collector_number = ce.collector_number AND
                                       c.language_code = ce.language_code AND c.foil = ce.foil
                               JOIN cardmarket_price cmp ON cardmarket_id = cmp.id_produit) AS prices
                WHERE prices.user_id = $1
                  AND prices.date = $2
                  AND CAST(prices.added_at AS DATE) <= to_timestamp(prices.date::text, 'YYYY-MM-DD')
                GROUP BY prices.user_id, prices.date
                ON CONFLICT (date, user_id) DO UPDATE SET
                    low   = EXCLUDED.low,
                    trend = EXCLUDED.trend,
                    avg   = EXCLUDED.avg"#,
            user.id.as_str(),
            date,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    #[tracing::instrument(name = "collection_price_history_repo.get_price_history", skip_all, fields(sentry.op = "db"))]
    async fn get_price_history(
        &self,
        user_id: &UserId,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<PriceHistoryEntry>, AppError> {
        let entities = sqlx::query_as!(
            CollectionPriceHistoryEntity,
            r#"SELECT date, low, trend, avg
                FROM collection_price_history
                WHERE user_id = $1
                  AND date >= $2
                  AND date <= $3
                ORDER BY date"#,
            user_id.as_str(),
            start_date,
            end_date,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(entities.into_iter().map(PriceHistoryEntry::from).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::adapter_out::repository::common_repository_tests::{
        fetch_collection_price_history, insert_card, insert_collection_entry,
        insert_collection_price_history, insert_price, insert_set, insert_user,
    };
    use crate::infrastructure::adapter_out::repository::entities::{
        CardMarketPriceEntity, PriceGuideEntity,
    };
    use chrono::{NaiveDate, Utc};
    use sqlx::PgPool;

    #[sqlx::test]
    async fn get_date_and_user_to_update_returns_empty_when_no_data(pool: PgPool) {
        let adapter = CollectionPriceHistoryRepositoryAdapter::new(pool);
        let result = adapter.get_date_and_user_to_update().await.unwrap();

        assert!(result.is_empty());
    }

    #[sqlx::test]
    async fn get_date_and_user_to_update_returns_combinations_not_in_history(pool: PgPool) {
        let adapter = CollectionPriceHistoryRepositoryAdapter::new(pool.clone());

        insert_set(&pool, "SET1").await;
        insert_card(&pool, "SET1", "1", "EN", false, "Test Card", 1).await;
        insert_user(&pool, "user1", "User1").await;
        insert_collection_entry(&pool, "SET1", "1", "EN", false, "user1", 1, 100, Utc::now()).await;
        insert_price(
            &pool,
            CardMarketPriceEntity {
                id_produit: 1,
                date: NaiveDate::from_ymd_opt(2025, 12, 25).unwrap(),
                normal: PriceGuideEntity {
                    low: Some(10),
                    avg: Some(20),
                    trend: Some(15),
                },
                foil: PriceGuideEntity::empty(),
            },
        )
        .await;

        let result = adapter.get_date_and_user_to_update().await.unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, NaiveDate::from_ymd_opt(2025, 12, 25).unwrap());
        assert_eq!(result[0].1.id, UserId::new("user1"));
    }

    #[sqlx::test]
    async fn get_date_and_user_to_update_returns_combinations_already_in_history(pool: PgPool) {
        let adapter = CollectionPriceHistoryRepositoryAdapter::new(pool.clone());

        let date = NaiveDate::from_ymd_opt(2025, 12, 25).unwrap();

        insert_set(&pool, "SET2").await;
        insert_card(&pool, "SET2", "1", "EN", false, "Test Card", 1).await;
        insert_user(&pool, "user1", "User1").await;
        insert_collection_entry(&pool, "SET2", "1", "EN", false, "user1", 1, 100, Utc::now()).await;
        insert_price(
            &pool,
            CardMarketPriceEntity {
                id_produit: 1,
                date,
                normal: PriceGuideEntity {
                    low: Some(10),
                    avg: Some(20),
                    trend: Some(15),
                },
                foil: PriceGuideEntity::empty(),
            },
        )
        .await;

        insert_collection_price_history(&pool, date, "user1", 100, 200, 150).await;

        let result = adapter.get_date_and_user_to_update().await.unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, date);
        assert_eq!(result[0].1.id, UserId::new("user1"));
    }

    #[sqlx::test]
    async fn get_date_and_user_to_update_returns_multiple_combinations(pool: PgPool) {
        let adapter = CollectionPriceHistoryRepositoryAdapter::new(pool.clone());

        let date1 = NaiveDate::from_ymd_opt(2025, 12, 25).unwrap();
        let date2 = NaiveDate::from_ymd_opt(2025, 12, 26).unwrap();

        insert_set(&pool, "SET3").await;
        insert_card(&pool, "SET3", "1", "EN", false, "Test Card", 1).await;
        insert_user(&pool, "user1", "User1").await;
        insert_user(&pool, "user2", "User2").await;
        insert_collection_entry(&pool, "SET3", "1", "EN", false, "user1", 1, 100, Utc::now()).await;
        insert_collection_entry(&pool, "SET3", "1", "EN", false, "user2", 2, 200, Utc::now()).await;
        insert_price(
            &pool,
            CardMarketPriceEntity {
                id_produit: 1,
                date: date1,
                normal: PriceGuideEntity {
                    low: Some(10),
                    avg: Some(20),
                    trend: Some(15),
                },
                foil: PriceGuideEntity::empty(),
            },
        )
        .await;
        insert_price(
            &pool,
            CardMarketPriceEntity {
                id_produit: 1,
                date: date2,
                normal: PriceGuideEntity {
                    low: Some(12),
                    avg: Some(22),
                    trend: Some(17),
                },
                foil: PriceGuideEntity::empty(),
            },
        )
        .await;

        let result = adapter.get_date_and_user_to_update().await.unwrap();

        assert_eq!(result.len(), 4);
    }

    #[sqlx::test]
    async fn update_for_date_and_user_filters_by_user(pool: PgPool) {
        let adapter = CollectionPriceHistoryRepositoryAdapter::new(pool.clone());
        let user1 = User::from_id(UserId::new("user1"));
        let date = NaiveDate::from_ymd_opt(2025, 12, 25).unwrap();

        let added_at = date.and_hms_opt(0, 0, 0).unwrap().and_utc();

        insert_set(&pool, "SET6").await;
        insert_card(&pool, "SET6", "1", "EN", false, "Card 1", 5).await;
        insert_user(&pool, "user1", "User1").await;
        insert_user(&pool, "user2", "User2").await;
        insert_collection_entry(&pool, "SET6", "1", "EN", false, "user1", 2, 100, added_at).await;
        insert_collection_entry(&pool, "SET6", "1", "EN", false, "user2", 3, 150, added_at).await;
        insert_price(
            &pool,
            CardMarketPriceEntity {
                id_produit: 5,
                date,
                normal: PriceGuideEntity {
                    low: Some(10),
                    avg: Some(20),
                    trend: Some(15),
                },
                foil: PriceGuideEntity::empty(),
            },
        )
        .await;

        adapter
            .update_for_date_and_user(date, user1.clone())
            .await
            .unwrap();

        let rows_user1 = fetch_collection_price_history(&pool, date, "user1").await;
        let rows_user2 = fetch_collection_price_history(&pool, date, "user2").await;

        assert_eq!(rows_user1.len(), 1);
        assert_eq!(rows_user1[0].low, 20i32); // 10 * 2
        assert_eq!(rows_user2.len(), 0);
    }
}
