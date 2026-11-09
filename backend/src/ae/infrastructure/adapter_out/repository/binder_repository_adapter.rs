use crate::application::error::AppError;
use crate::application::repository::BinderRepository;
use crate::domain::binder::{Binder, BinderId, BinderName};
use crate::domain::error::FunctionalError;
use crate::domain::user::UserId;
use async_trait::async_trait;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

/// Postgres error code for `unique_violation`.
const UNIQUE_VIOLATION: &str = "23505";

pub struct BinderRepositoryAdapter {
    pool: Pool<Postgres>,
}

impl BinderRepositoryAdapter {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    /// `user_id`'s binders sorted by name: only `only`, if given.
    async fn fetch(&self, user_id: &UserId, only: Option<Uuid>) -> Result<Vec<Binder>, AppError> {
        let rows = sqlx::query!(
            r#"
            SELECT b.id,
                   b.name,
                   COALESCE(SUM(ce.quantity), 0)::BIGINT AS "copies!",
                   (tb.binder_id IS NOT NULL)            AS "is_open!"
            FROM binder b
                     LEFT JOIN collection_entry ce ON ce.binder_id = b.id
                     LEFT JOIN trading_binders tb ON tb.binder_id = b.id
            WHERE b.user_id = $1
              AND ($2::UUID IS NULL OR b.id = $2)
            GROUP BY b.id, b.name, tb.binder_id
            ORDER BY b.name
            "#,
            user_id.as_str(),
            only
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| Binder {
                id: BinderId(row.id),
                name: row.name,
                copies: row.copies.max(0) as u64,
                is_open: row.is_open,
            })
            .collect())
    }
}

/// Reads a write on `binder_user_name_uk` as the name being taken.
fn name_taken_on_unique_violation(err: sqlx::Error) -> AppError {
    match err {
        sqlx::Error::Database(db_err) if db_err.code().as_deref() == Some(UNIQUE_VIOLATION) => {
            FunctionalError::BinderNameTaken.into()
        }
        other => other.into(),
    }
}

#[async_trait]
impl BinderRepository for BinderRepositoryAdapter {
    #[tracing::instrument(name = "binder_repo.list", skip_all, fields(sentry.op = "db"))]
    async fn list(&self, user_id: &UserId) -> Result<Vec<Binder>, AppError> {
        self.fetch(user_id, None).await
    }

    #[tracing::instrument(name = "binder_repo.create", skip_all, fields(sentry.op = "db"))]
    async fn create(&self, user_id: &UserId, name: &BinderName) -> Result<Binder, AppError> {
        let id = sqlx::query_scalar!(
            r#"INSERT INTO binder (user_id, name) VALUES ($1, $2) RETURNING id"#,
            user_id.as_str(),
            name.as_str()
        )
        .fetch_one(&self.pool)
        .await
        .map_err(name_taken_on_unique_violation)?;

        // The binder has just been created: it holds nothing and is closed to trade.
        Ok(Binder {
            id: BinderId(id),
            name: name.as_str().to_string(),
            copies: 0,
            is_open: false,
        })
    }

    #[tracing::instrument(name = "binder_repo.rename", skip_all, fields(sentry.op = "db"))]
    async fn rename(
        &self,
        user_id: &UserId,
        id: BinderId,
        name: &BinderName,
    ) -> Result<Binder, AppError> {
        let updated = sqlx::query!(
            r#"UPDATE binder SET name = $3 WHERE id = $2 AND user_id = $1"#,
            user_id.as_str(),
            id.0,
            name.as_str()
        )
        .execute(&self.pool)
        .await
        .map_err(name_taken_on_unique_violation)?;
        if updated.rows_affected() == 0 {
            return Err(FunctionalError::BinderNotFound.into());
        }

        self.fetch(user_id, Some(id.0))
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| FunctionalError::BinderNotFound.into())
    }

    #[tracing::instrument(name = "binder_repo.delete", skip_all, fields(sentry.op = "db"))]
    async fn delete(&self, user_id: &UserId, id: BinderId) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;

        // The lock keeps a concurrent write into the binder from slipping in between the check
        // and the delete: such a write takes a key share lock on the same row.
        let found = sqlx::query_scalar!(
            r#"SELECT id FROM binder WHERE id = $1 AND user_id = $2 FOR UPDATE"#,
            id.0,
            user_id.as_str()
        )
        .fetch_optional(&mut *tx)
        .await?;
        if found.is_none() {
            return Err(FunctionalError::BinderNotFound.into());
        }

        let holds_copies = sqlx::query_scalar!(
            r#"SELECT EXISTS (SELECT 1 FROM collection_entry WHERE binder_id = $1) AS "exists!""#,
            id.0
        )
        .fetch_one(&mut *tx)
        .await?;
        if holds_copies {
            return Err(FunctionalError::BinderNotEmpty.into());
        }

        // Its trade opening goes with it (`ON DELETE CASCADE`).
        sqlx::query!(r#"DELETE FROM binder WHERE id = $1"#, id.0)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::adapter_out::repository::common_repository_tests::{
        insert_binder, insert_card_without_cardmarket_id, insert_collection_entry_with_binder,
        insert_trading_binder, insert_user,
    };
    use chrono::Utc;
    use sqlx::PgPool;

    fn name(value: &str) -> BinderName {
        BinderName::try_new(value).unwrap()
    }

    fn user(id: &str) -> UserId {
        UserId::new(id)
    }

    async fn trading_binder_count(pool: &PgPool) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM trading_binders")
            .fetch_one(pool)
            .await
            .unwrap()
    }

    async fn setup(pool: &PgPool) -> BinderRepositoryAdapter {
        insert_user(pool, "user-1", "User1").await;
        insert_user(pool, "user-2", "User2").await;
        BinderRepositoryAdapter::new(pool.clone())
    }

    async fn store_copies(pool: &PgPool, user_id: &str, binder: &str, quantity: i32) {
        insert_card_without_cardmarket_id(pool, "FDN", "1", "FR", "Card").await;
        insert_collection_entry_with_binder(
            pool,
            "FDN",
            "1",
            "FR",
            false,
            user_id,
            quantity,
            100,
            Utc::now(),
            Some(binder),
        )
        .await;
    }

    #[sqlx::test]
    async fn create_makes_a_binder_closed_to_trade(pool: PgPool) {
        let repository = setup(&pool).await;

        let binder = repository
            .create(&user("user-1"), &name("Trade"))
            .await
            .unwrap();

        assert_eq!(binder.name, "Trade");
        assert_eq!(binder.copies, 0);
        assert!(!binder.is_open);
        assert_eq!(trading_binder_count(&pool).await, 0);
        let listed = repository.list(&user("user-1")).await.unwrap();
        assert_eq!(listed, vec![binder]);
    }

    #[sqlx::test]
    async fn create_refuses_a_name_the_player_already_uses(pool: PgPool) {
        let repository = setup(&pool).await;
        repository
            .create(&user("user-1"), &name("Trade"))
            .await
            .unwrap();

        let result = repository.create(&user("user-1"), &name("Trade")).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::BinderNameTaken))
        ));
    }

    #[sqlx::test]
    async fn two_players_can_use_the_same_name(pool: PgPool) {
        let repository = setup(&pool).await;
        repository
            .create(&user("user-1"), &name("Trade"))
            .await
            .unwrap();

        let result = repository.create(&user("user-2"), &name("Trade")).await;

        assert!(result.is_ok());
    }

    #[sqlx::test]
    async fn list_shows_empty_binders_copies_and_trade_opening_of_the_player_only(pool: PgPool) {
        let repository = setup(&pool).await;
        store_copies(&pool, "user-1", "Full", 3).await;
        insert_trading_binder(&pool, "user-1", "Full").await;
        insert_binder(&pool, "user-1", "Empty").await;
        insert_binder(&pool, "user-2", "Other").await;

        let binders = repository.list(&user("user-1")).await.unwrap();

        let summary: Vec<_> = binders
            .iter()
            .map(|b| (b.name.as_str(), b.copies, b.is_open))
            .collect();
        assert_eq!(summary, vec![("Empty", 0, false), ("Full", 3, true)]);
    }

    #[sqlx::test]
    async fn rename_keeps_the_trade_opening_and_the_copies(pool: PgPool) {
        let repository = setup(&pool).await;
        store_copies(&pool, "user-1", "Old", 2).await;
        insert_trading_binder(&pool, "user-1", "Old").await;
        let id = repository.list(&user("user-1")).await.unwrap()[0].id;

        let renamed = repository
            .rename(&user("user-1"), id, &name("New"))
            .await
            .unwrap();

        assert_eq!(renamed.id, id);
        assert_eq!(renamed.name, "New");
        assert_eq!(renamed.copies, 2);
        assert!(renamed.is_open);
        assert_eq!(trading_binder_count(&pool).await, 1);
    }

    #[sqlx::test]
    async fn rename_refuses_a_name_another_binder_of_the_player_has(pool: PgPool) {
        let repository = setup(&pool).await;
        repository
            .create(&user("user-1"), &name("A"))
            .await
            .unwrap();
        let b = repository
            .create(&user("user-1"), &name("B"))
            .await
            .unwrap();

        let result = repository.rename(&user("user-1"), b.id, &name("A")).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::BinderNameTaken))
        ));
    }

    #[sqlx::test]
    async fn rename_to_its_own_name_is_accepted(pool: PgPool) {
        let repository = setup(&pool).await;
        let a = repository
            .create(&user("user-1"), &name("A"))
            .await
            .unwrap();

        let result = repository.rename(&user("user-1"), a.id, &name("A")).await;

        assert!(result.is_ok());
    }

    #[sqlx::test]
    async fn a_player_cannot_rename_the_binder_of_another(pool: PgPool) {
        let repository = setup(&pool).await;
        let theirs = repository
            .create(&user("user-2"), &name("Theirs"))
            .await
            .unwrap();

        let result = repository
            .rename(&user("user-1"), theirs.id, &name("Mine"))
            .await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::BinderNotFound))
        ));
        assert_eq!(
            repository.list(&user("user-2")).await.unwrap()[0].name,
            "Theirs"
        );
    }

    #[sqlx::test]
    async fn delete_refuses_a_binder_that_holds_copies(pool: PgPool) {
        let repository = setup(&pool).await;
        store_copies(&pool, "user-1", "Full", 1).await;
        let id = repository.list(&user("user-1")).await.unwrap()[0].id;

        let result = repository.delete(&user("user-1"), id).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::BinderNotEmpty))
        ));
        assert_eq!(repository.list(&user("user-1")).await.unwrap().len(), 1);
    }

    #[sqlx::test]
    async fn delete_removes_an_empty_binder_with_its_trade_opening(pool: PgPool) {
        let repository = setup(&pool).await;
        insert_trading_binder(&pool, "user-1", "Empty").await;
        let id = repository.list(&user("user-1")).await.unwrap()[0].id;

        repository.delete(&user("user-1"), id).await.unwrap();

        assert!(repository.list(&user("user-1")).await.unwrap().is_empty());
        assert_eq!(trading_binder_count(&pool).await, 0);
    }

    #[sqlx::test]
    async fn a_player_cannot_delete_the_binder_of_another(pool: PgPool) {
        let repository = setup(&pool).await;
        let theirs = repository
            .create(&user("user-2"), &name("Theirs"))
            .await
            .unwrap();

        let result = repository.delete(&user("user-1"), theirs.id).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::BinderNotFound))
        ));
        assert_eq!(repository.list(&user("user-2")).await.unwrap().len(), 1);
    }

    #[sqlx::test]
    async fn delete_of_an_unknown_binder_is_not_found(pool: PgPool) {
        let repository = setup(&pool).await;

        let result = repository
            .delete(&user("user-1"), BinderId(Uuid::new_v4()))
            .await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::BinderNotFound))
        ));
    }
}
