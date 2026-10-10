use crate::application::error::{AppError, InfraError};
use crate::application::repository::StatsRepository;
use crate::infrastructure::adapter_out::repository::entities::{
    CountEntity, SizeEntity, invalid_db_value,
};
use async_trait::async_trait;
use chrono::NaiveDate;
use sqlx::{Pool, Postgres};

pub struct StatsRepositoryAdapter {
    pool: Pool<Postgres>,
}

impl StatsRepositoryAdapter {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl StatsRepository for StatsRepositoryAdapter {
    #[tracing::instrument(name = "stats_repo.get_card_number", skip_all, fields(sentry.op = "db"))]
    async fn get_card_number(&self) -> Result<u32, AppError> {
        Ok(
            sqlx::query_as!(CountEntity, "SELECT count(*) AS count FROM card")
                .fetch_one(&self.pool)
                .await?
                .count
                .unwrap() as u32,
        )
    }

    #[tracing::instrument(name = "stats_repo.get_card_price_number", skip_all, fields(sentry.op = "db"))]
    async fn get_card_price_number(&self) -> Result<u32, AppError> {
        Ok(sqlx::query_as!(
            CountEntity,
            "SELECT count(*) AS count FROM cardmarket_price"
        )
        .fetch_one(&self.pool)
        .await?
        .count
        .unwrap() as u32)
    }

    #[tracing::instrument(name = "stats_repo.get_db_size", skip_all, fields(sentry.op = "db"))]
    async fn get_db_size(&self) -> Result<u16, AppError> {
        Ok(sqlx::query_as!(
            SizeEntity,
            "SELECT pg_database_size(current_database()) / 1024 / 1024 as size"
        )
        .fetch_one(&self.pool)
        .await?
        .size
        .unwrap() as u16)
    }

    #[tracing::instrument(name = "stats_repo.get_last_price_date", skip_all, fields(sentry.op = "db"))]
    async fn get_last_price_date(&self) -> Result<Option<NaiveDate>, AppError> {
        Ok(
            sqlx::query_scalar!("SELECT max(date) AS last_price_date FROM cardmarket_price")
                .fetch_one(&self.pool)
                .await?,
        )
    }

    /// Read from `v_tradable_entry`, the single source of a card's exposure to a third party:
    /// visibility, trading binders and rarity filters are applied there, not here.
    #[tracing::instrument(name = "stats_repo.get_proposed_copy_number", skip_all, fields(sentry.op = "db"))]
    async fn get_proposed_copy_number(&self) -> Result<u32, AppError> {
        let total = sqlx::query_scalar!(
            r#"SELECT COALESCE(SUM(proposed_quantity), 0)::BIGINT AS "total!" FROM v_tradable_entry"#
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(proposed_copy_number_from_db(total)?)
    }
}

fn proposed_copy_number_from_db(total: i64) -> Result<u32, InfraError> {
    u32::try_from(total).map_err(|_| invalid_db_value("proposed copy number", &total.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::adapter_out::repository::common_repository_tests::{
        insert_card_without_cardmarket_id, insert_price, insert_set,
    };
    use crate::infrastructure::adapter_out::repository::entities::CardMarketPriceEntity;
    use sqlx::PgPool;

    #[test]
    fn a_proposed_copy_total_out_of_range_is_a_repository_error() {
        assert_eq!(proposed_copy_number_from_db(1248), Ok(1248));
        assert_eq!(
            proposed_copy_number_from_db(4_294_967_296),
            Err(InfraError::RepositoryError(
                "invalid proposed copy number from database: 4294967296".to_string()
            ))
        );
        assert_eq!(
            proposed_copy_number_from_db(-1),
            Err(InfraError::RepositoryError(
                "invalid proposed copy number from database: -1".to_string()
            ))
        );
    }

    #[sqlx::test]
    async fn should_return_card_count(pool: PgPool) {
        let adapter = StatsRepositoryAdapter::new(pool.clone());

        insert_set(&pool, "TST").await;
        insert_card_without_cardmarket_id(&pool, "TST", "1", "en", "Test Card").await;
        insert_card_without_cardmarket_id(&pool, "TST", "2", "en", "Another Card").await;

        let result = adapter.get_card_number().await;

        assert!(result.is_ok());
        let count = result.unwrap();
        assert_eq!(count, 2, "should return count of cards in the database");
    }

    #[sqlx::test]
    async fn should_return_zero_card_count_when_no_cards_exist(pool: PgPool) {
        let adapter = StatsRepositoryAdapter::new(pool.clone());

        sqlx::query("TRUNCATE card CASCADE")
            .execute(&pool)
            .await
            .unwrap();

        let result = adapter.get_card_number().await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[sqlx::test]
    async fn should_return_card_price_number(pool: PgPool) {
        let adapter = StatsRepositoryAdapter::new(pool);
        let result = adapter.get_card_price_number().await;

        assert!(result.is_ok());
        let _count = result.unwrap();
    }

    #[sqlx::test]
    async fn should_return_zero_card_price_count_when_no_prices_exist(pool: PgPool) {
        let adapter = StatsRepositoryAdapter::new(pool.clone());

        sqlx::query("TRUNCATE cardmarket_price")
            .execute(&pool)
            .await
            .unwrap();

        let result = adapter.get_card_price_number().await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[sqlx::test]
    async fn should_return_the_most_recent_price_date(pool: PgPool) {
        let adapter = StatsRepositoryAdapter::new(pool.clone());
        let older = NaiveDate::from_ymd_opt(2026, 9, 30).unwrap();
        let latest = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        insert_price(&pool, CardMarketPriceEntity::simple_at(1, latest, 100)).await;
        insert_price(&pool, CardMarketPriceEntity::simple_at(2, older, 100)).await;

        let result = adapter.get_last_price_date().await;

        assert_eq!(result.unwrap(), Some(latest));
    }

    #[sqlx::test]
    async fn should_return_no_price_date_when_no_prices_exist(pool: PgPool) {
        let adapter = StatsRepositoryAdapter::new(pool.clone());

        sqlx::query("TRUNCATE cardmarket_price")
            .execute(&pool)
            .await
            .unwrap();

        let result = adapter.get_last_price_date().await;

        assert_eq!(result.unwrap(), None);
    }

    #[sqlx::test]
    async fn should_return_db_size_in_megabytes(pool: PgPool) {
        let adapter = StatsRepositoryAdapter::new(pool);
        let result = adapter.get_db_size().await;

        assert!(result.is_ok());
        let _size = result.unwrap();
    }

    #[sqlx::test]
    async fn should_return_consistent_card_count_on_multiple_calls(pool: PgPool) {
        let adapter = StatsRepositoryAdapter::new(pool);

        let first_call = adapter.get_card_number().await.unwrap();
        let second_call = adapter.get_card_number().await.unwrap();

        assert_eq!(
            first_call, second_call,
            "multiple calls should return the same count without mutations"
        );
    }

    #[sqlx::test]
    async fn should_return_independent_stat_values(pool: PgPool) {
        let adapter = StatsRepositoryAdapter::new(pool);

        let _card_count = adapter.get_card_number().await.unwrap();
        let _price_count = adapter.get_card_price_number().await.unwrap();
        let _db_size = adapter.get_db_size().await.unwrap();
    }
}
