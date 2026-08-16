use crate::application::error::AppError;
use crate::application::repository::TradingBinderRepository;
use crate::domain::user::UserId;
use async_trait::async_trait;
use sqlx::{Pool, Postgres};

pub struct TradingBindersRepositoryAdapter {
    pool: Pool<Postgres>,
}

impl TradingBindersRepositoryAdapter {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TradingBinderRepository for TradingBindersRepositoryAdapter {
    async fn list(&self, user_id: &UserId) -> Result<Vec<String>, AppError> {
        let rows = sqlx::query!(
            r#"
            SELECT binder_name
            FROM trading_binders
            WHERE user_id = $1
            ORDER BY binder_name
            "#,
            user_id.as_str()
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.binder_name).collect())
    }

    async fn add(&self, user_id: &UserId, binder_name: &str) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO trading_binders (user_id, binder_name)
            VALUES ($1, $2)
            ON CONFLICT (user_id, binder_name) DO NOTHING
            "#,
            user_id.as_str(),
            binder_name
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn remove(&self, user_id: &UserId, binder_name: &str) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            DELETE FROM trading_binders
            WHERE user_id = $1 AND binder_name = $2
            "#,
            user_id.as_str(),
            binder_name
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn binder_exists(&self, user_id: &UserId, binder_name: &str) -> Result<bool, AppError> {
        let row = sqlx::query!(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM collection_entry
                WHERE user_id = $1 AND binder_name = $2
            ) AS "exists!"
            "#,
            user_id.as_str(),
            binder_name
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.exists)
    }

    async fn purge_missing(&self, user_id: &UserId) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            DELETE FROM trading_binders
            WHERE user_id = $1
              AND binder_name NOT IN (
                  SELECT DISTINCT binder_name
                  FROM collection_entry
                  WHERE user_id = $1 AND binder_name IS NOT NULL
              )
            "#,
            user_id.as_str()
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::adapter_out::repository::common_repository_tests::{
        insert_card_without_cardmarket_id, insert_collection_entry_with_binder, insert_set,
        insert_user,
    };
    use chrono::Utc;
    use sqlx::PgPool;

    #[sqlx::test]
    async fn list_is_empty_for_user_without_selection(pool: PgPool) {
        insert_user(&pool, "user-1", "User1").await;

        let adapter = TradingBindersRepositoryAdapter::new(pool);
        let result = adapter.list(&UserId::new("user-1")).await.unwrap();

        assert!(result.is_empty());
    }

    #[sqlx::test]
    async fn add_then_list_returns_the_binder(pool: PgPool) {
        insert_user(&pool, "user-1", "User1").await;

        let adapter = TradingBindersRepositoryAdapter::new(pool);
        adapter
            .add(&UserId::new("user-1"), "Trade Binder")
            .await
            .unwrap();

        let result = adapter.list(&UserId::new("user-1")).await.unwrap();
        assert_eq!(result, vec!["Trade Binder".to_string()]);
    }

    #[sqlx::test]
    async fn add_is_idempotent(pool: PgPool) {
        insert_user(&pool, "user-1", "User1").await;

        let adapter = TradingBindersRepositoryAdapter::new(pool);
        adapter
            .add(&UserId::new("user-1"), "Trade Binder")
            .await
            .unwrap();
        adapter
            .add(&UserId::new("user-1"), "Trade Binder")
            .await
            .unwrap();

        let result = adapter.list(&UserId::new("user-1")).await.unwrap();
        assert_eq!(result, vec!["Trade Binder".to_string()]);
    }

    #[sqlx::test]
    async fn remove_deselects_the_binder(pool: PgPool) {
        insert_user(&pool, "user-1", "User1").await;

        let adapter = TradingBindersRepositoryAdapter::new(pool);
        adapter
            .add(&UserId::new("user-1"), "Trade Binder")
            .await
            .unwrap();
        adapter
            .remove(&UserId::new("user-1"), "Trade Binder")
            .await
            .unwrap();

        let result = adapter.list(&UserId::new("user-1")).await.unwrap();
        assert!(result.is_empty());
    }

    #[sqlx::test]
    async fn remove_of_unselected_binder_is_a_no_op(pool: PgPool) {
        insert_user(&pool, "user-1", "User1").await;

        let adapter = TradingBindersRepositoryAdapter::new(pool);
        let result = adapter
            .remove(&UserId::new("user-1"), "Unknown Binder")
            .await;

        assert!(result.is_ok());
    }

    #[sqlx::test]
    async fn list_only_returns_the_caller_selections(pool: PgPool) {
        insert_user(&pool, "user-1", "User1").await;
        insert_user(&pool, "user-2", "User2").await;

        let adapter = TradingBindersRepositoryAdapter::new(pool);
        adapter
            .add(&UserId::new("user-1"), "Trade Binder")
            .await
            .unwrap();
        adapter.add(&UserId::new("user-2"), "Bulk").await.unwrap();

        let result = adapter.list(&UserId::new("user-1")).await.unwrap();
        assert_eq!(result, vec!["Trade Binder".to_string()]);
    }

    #[sqlx::test]
    async fn binder_exists_is_true_when_present_in_collection(pool: PgPool) {
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
            Some("Trade Binder"),
        )
        .await;

        let adapter = TradingBindersRepositoryAdapter::new(pool);
        let result = adapter
            .binder_exists(&UserId::new("user-1"), "Trade Binder")
            .await
            .unwrap();

        assert!(result);
    }

    #[sqlx::test]
    async fn binder_exists_is_false_for_unknown_name(pool: PgPool) {
        insert_user(&pool, "user-1", "User1").await;

        let adapter = TradingBindersRepositoryAdapter::new(pool);
        let result = adapter
            .binder_exists(&UserId::new("user-1"), "Unknown")
            .await
            .unwrap();

        assert!(!result);
    }

    #[sqlx::test]
    async fn binder_exists_is_false_for_another_users_binder(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card_without_cardmarket_id(&pool, "TST", "1", "en", false, "Card A").await;
        insert_user(&pool, "user-other", "UserOther").await;
        insert_user(&pool, "user-1", "User1").await;
        insert_collection_entry_with_binder(
            &pool,
            "TST",
            "1",
            "en",
            false,
            "user-other",
            2,
            100,
            Utc::now(),
            Some("Trade Binder"),
        )
        .await;

        let adapter = TradingBindersRepositoryAdapter::new(pool);
        let result = adapter
            .binder_exists(&UserId::new("user-1"), "Trade Binder")
            .await
            .unwrap();

        assert!(!result);
    }

    #[sqlx::test]
    async fn purge_missing_removes_orphaned_selections(pool: PgPool) {
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
            Some("Bulk"),
        )
        .await;

        let adapter = TradingBindersRepositoryAdapter::new(pool);
        adapter.add(&UserId::new("user-1"), "Bulk").await.unwrap();
        adapter.add(&UserId::new("user-1"), "Decks").await.unwrap();

        adapter.purge_missing(&UserId::new("user-1")).await.unwrap();

        let result = adapter.list(&UserId::new("user-1")).await.unwrap();
        assert_eq!(result, vec!["Bulk".to_string()]);
    }

    #[sqlx::test]
    async fn purge_missing_does_not_touch_other_users(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card_without_cardmarket_id(&pool, "TST", "1", "en", false, "Card A").await;
        insert_user(&pool, "user-1", "User1").await;
        insert_user(&pool, "user-2", "User2").await;
        insert_collection_entry_with_binder(
            &pool,
            "TST",
            "1",
            "en",
            false,
            "user-2",
            2,
            100,
            Utc::now(),
            Some("Bulk"),
        )
        .await;

        let adapter = TradingBindersRepositoryAdapter::new(pool);
        adapter.add(&UserId::new("user-2"), "Bulk").await.unwrap();
        adapter.add(&UserId::new("user-2"), "Decks").await.unwrap();

        adapter.purge_missing(&UserId::new("user-1")).await.unwrap();

        let result = adapter.list(&UserId::new("user-2")).await.unwrap();
        assert_eq!(result, vec!["Bulk".to_string(), "Decks".to_string()]);
    }
}
