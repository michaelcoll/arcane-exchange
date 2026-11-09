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
    #[tracing::instrument(name = "trading_binders_repo.list", skip_all, fields(sentry.op = "db"))]
    async fn list(&self, user_id: &UserId) -> Result<Vec<String>, AppError> {
        let rows = sqlx::query!(
            r#"
            SELECT b.name
            FROM trading_binders tb
                     JOIN binder b ON b.id = tb.binder_id
            WHERE b.user_id = $1
            ORDER BY b.name
            "#,
            user_id.as_str()
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.name).collect())
    }

    #[tracing::instrument(name = "trading_binders_repo.add", skip_all, fields(sentry.op = "db"))]
    async fn add(&self, user_id: &UserId, binder_name: &str) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO trading_binders (binder_id)
            SELECT id
            FROM binder
            WHERE user_id = $1 AND name = $2
            ON CONFLICT (binder_id) DO NOTHING
            "#,
            user_id.as_str(),
            binder_name
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    #[tracing::instrument(name = "trading_binders_repo.remove", skip_all, fields(sentry.op = "db"))]
    async fn remove(&self, user_id: &UserId, binder_name: &str) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            DELETE FROM trading_binders
            WHERE binder_id IN (SELECT id FROM binder WHERE user_id = $1 AND name = $2)
            "#,
            user_id.as_str(),
            binder_name
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    #[tracing::instrument(name = "trading_binders_repo.binder_exists", skip_all, fields(sentry.op = "db"))]
    async fn binder_exists(&self, user_id: &UserId, binder_name: &str) -> Result<bool, AppError> {
        let row = sqlx::query!(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM binder
                WHERE user_id = $1 AND name = $2
            ) AS "exists!"
            "#,
            user_id.as_str(),
            binder_name
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.exists)
    }

    #[tracing::instrument(name = "trading_binders_repo.purge_missing", skip_all, fields(sentry.op = "db"))]
    async fn purge_missing(&self, user_id: &UserId) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            DELETE FROM binder b
            WHERE b.user_id = $1
              AND NOT EXISTS (SELECT 1 FROM collection_entry ce WHERE ce.binder_id = b.id)
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
        insert_binder, insert_card_without_cardmarket_id, insert_collection_entry_with_binder,
        insert_set, insert_user,
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
        insert_binder(&pool, "user-1", "Trade Binder").await;

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
        insert_binder(&pool, "user-1", "Trade Binder").await;

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
        insert_binder(&pool, "user-1", "Trade Binder").await;

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
        insert_binder(&pool, "user-1", "Trade Binder").await;
        insert_binder(&pool, "user-2", "Bulk").await;

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
        insert_card_without_cardmarket_id(&pool, "TST", "1", "en", "Card A").await;
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
            Some("Bulk"),
        )
        .await;
        insert_binder(&pool, "user-1", "Decks").await;

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
        insert_card_without_cardmarket_id(&pool, "TST", "1", "en", "Card A").await;
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
        insert_binder(&pool, "user-2", "Decks").await;

        let adapter = TradingBindersRepositoryAdapter::new(pool);
        adapter.add(&UserId::new("user-2"), "Bulk").await.unwrap();
        adapter.add(&UserId::new("user-2"), "Decks").await.unwrap();

        adapter.purge_missing(&UserId::new("user-1")).await.unwrap();

        let result = adapter.list(&UserId::new("user-2")).await.unwrap();
        assert_eq!(result, vec!["Bulk".to_string(), "Decks".to_string()]);
    }

    #[sqlx::test(migrations = false)]
    async fn migration_attaches_each_entry_and_opening_to_its_binder(pool: PgPool) {
        let migrator = sqlx::migrate!("./migrations");
        migrator.run_to(34, &pool).await.unwrap();
        insert_set(&pool, "TST").await;
        insert_card_without_cardmarket_id(&pool, "TST", "1", "en", "Card A").await;
        insert_card_without_cardmarket_id(&pool, "TST", "2", "en", "Card B").await;
        insert_user(&pool, "user-1", "User1").await;
        insert_user(&pool, "user-2", "User2").await;
        for (user, number, binder) in [
            ("user-1", "1", Some("Bulk")),
            ("user-1", "2", Some("Bulk")),
            ("user-1", "1", None),
            ("user-2", "1", Some("Bulk")),
        ] {
            sqlx::query(
                r#"INSERT INTO collection_entry (set_code, collector_number, language_code, foil,
                       user_id, quantity, purchase_price, binder_name)
                   VALUES ('TST', $1, 'en', FALSE, $2, 1, 100, $3)"#,
            )
            .bind(number)
            .bind(user)
            .bind(binder)
            .execute(&pool)
            .await
            .unwrap();
        }
        sqlx::query(
            r#"INSERT INTO trading_binders (user_id, binder_name) VALUES ('user-1', 'Bulk'), ('user-1', 'Empty')"#,
        )
        .execute(&pool)
        .await
        .unwrap();

        migrator.run(&pool).await.unwrap();

        let binders: Vec<(String, String)> =
            sqlx::query_as("SELECT user_id, name FROM binder ORDER BY user_id, name")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(
            binders,
            vec![
                ("user-1".to_string(), "Bulk".to_string()),
                ("user-1".to_string(), "Empty".to_string()),
                ("user-2".to_string(), "Bulk".to_string()),
            ]
        );
        let entries: Vec<(String, String, Option<String>)> = sqlx::query_as(
            r#"SELECT ce.user_id, ce.collector_number, b.name
               FROM collection_entry ce LEFT JOIN binder b ON b.id = ce.binder_id
               ORDER BY ce.user_id, ce.collector_number, b.name"#,
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(
            entries,
            vec![
                (
                    "user-1".to_string(),
                    "1".to_string(),
                    Some("Bulk".to_string())
                ),
                ("user-1".to_string(), "1".to_string(), None),
                (
                    "user-1".to_string(),
                    "2".to_string(),
                    Some("Bulk".to_string())
                ),
                (
                    "user-2".to_string(),
                    "1".to_string(),
                    Some("Bulk".to_string())
                ),
            ]
        );
        let adapter = TradingBindersRepositoryAdapter::new(pool);
        assert_eq!(
            adapter.list(&UserId::new("user-1")).await.unwrap(),
            vec!["Bulk".to_string(), "Empty".to_string()]
        );
        assert!(
            adapter
                .list(&UserId::new("user-2"))
                .await
                .unwrap()
                .is_empty()
        );
    }

    /// Inserts a legacy entry (before migration 0035): the binder is a name, `NULL` for none.
    async fn insert_legacy_entry(
        pool: &PgPool,
        number: &str,
        quantity: i32,
        purchase_price: i32,
        added_at: &str,
        binder_name: Option<&str>,
    ) {
        sqlx::query(
            r#"INSERT INTO collection_entry (set_code, collector_number, language_code, foil,
                   user_id, quantity, purchase_price, added_at, binder_name)
               VALUES ('TST', $1, 'en', FALSE, 'user-1', $2, $3, $4::timestamptz, $5)"#,
        )
        .bind(number)
        .bind(quantity)
        .bind(purchase_price)
        .bind(added_at)
        .bind(binder_name)
        .execute(pool)
        .await
        .unwrap();
    }

    #[sqlx::test(migrations = false)]
    async fn migration_merges_the_binder_names_that_only_differ_by_spaces(pool: PgPool) {
        let migrator = sqlx::migrate!("./migrations");
        migrator.run_to(34, &pool).await.unwrap();
        insert_set(&pool, "TST").await;
        for (number, name) in [("1", "Card A"), ("2", "Card B"), ("3", "Card C")] {
            insert_card_without_cardmarket_id(&pool, "TST", number, "en", name).await;
        }
        insert_user(&pool, "user-1", "User1").await;
        // "A" and "A " are two binders to the old import, which never trimmed names.
        insert_legacy_entry(&pool, "1", 2, 1000, "2026-01-10T00:00:00Z", Some("A")).await;
        insert_legacy_entry(&pool, "1", 1, 400, "2026-01-03T00:00:00Z", Some("A ")).await;
        // A name made of spaces only is no binder: it merges with the unbinned entry.
        insert_legacy_entry(&pool, "2", 1, 500, "2026-01-05T00:00:00Z", Some("   ")).await;
        insert_legacy_entry(&pool, "2", 3, 0, "2026-01-02T00:00:00Z", None).await;
        // The merged quantity stays within the 255 an entry holds.
        insert_legacy_entry(&pool, "3", 200, 100, "2026-01-01T00:00:00Z", Some("B")).await;
        insert_legacy_entry(&pool, "3", 100, 100, "2026-01-01T00:00:00Z", Some("B ")).await;
        sqlx::query(
            r#"INSERT INTO trading_binders (user_id, binder_name)
               VALUES ('user-1', 'A'), ('user-1', 'A '), ('user-1', '   ')"#,
        )
        .execute(&pool)
        .await
        .unwrap();

        migrator.run(&pool).await.unwrap();

        let binders: Vec<String> =
            sqlx::query_scalar("SELECT name FROM binder WHERE user_id = 'user-1' ORDER BY name")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(binders, vec!["A".to_string(), "B".to_string()]);
        // Collector number, binder name, quantity, purchase price, added day.
        type MergedEntry = (String, Option<String>, i32, Option<i32>, String);
        let entries: Vec<MergedEntry> = sqlx::query_as(
            r#"SELECT ce.collector_number, b.name, ce.quantity, ce.purchase_price,
                      to_char(ce.added_at AT TIME ZONE 'UTC', 'YYYY-MM-DD')
               FROM collection_entry ce LEFT JOIN binder b ON b.id = ce.binder_id
               ORDER BY ce.collector_number"#,
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(
            entries,
            vec![
                (
                    "1".to_string(),
                    Some("A".to_string()),
                    3,
                    Some(800),
                    "2026-01-03".to_string()
                ),
                (
                    "2".to_string(),
                    None,
                    4,
                    Some(500),
                    "2026-01-02".to_string()
                ),
                (
                    "3".to_string(),
                    Some("B".to_string()),
                    255,
                    Some(100),
                    "2026-01-01".to_string()
                ),
            ]
        );
        // One opening for "A"; the blank one is dropped with the binder it cannot be.
        let adapter = TradingBindersRepositoryAdapter::new(pool);
        assert_eq!(
            adapter.list(&UserId::new("user-1")).await.unwrap(),
            vec!["A".to_string()]
        );
    }

    #[sqlx::test(migrations = false)]
    async fn migration_makes_a_purchase_price_of_zero_absent(pool: PgPool) {
        let migrator = sqlx::migrate!("./migrations");
        migrator.run_to(35, &pool).await.unwrap();
        insert_set(&pool, "TST").await;
        for (number, name) in [("1", "Card A"), ("2", "Card B")] {
            insert_card_without_cardmarket_id(&pool, "TST", number, "en", name).await;
        }
        insert_user(&pool, "user-1", "User1").await;
        for (number, price) in [("1", 0), ("2", 250)] {
            sqlx::query(
                r#"INSERT INTO collection_entry (set_code, collector_number, language_code, foil,
                       user_id, quantity, purchase_price)
                   VALUES ('TST', $1, 'en', FALSE, 'user-1', 1, $2)"#,
            )
            .bind(number)
            .bind(price)
            .execute(&pool)
            .await
            .unwrap();
        }

        migrator.run(&pool).await.unwrap();

        let prices: Vec<(String, Option<i32>)> = sqlx::query_as(
            "SELECT collector_number, purchase_price FROM collection_entry ORDER BY collector_number",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(
            prices,
            vec![("1".to_string(), None), ("2".to_string(), Some(250))]
        );
    }
}
