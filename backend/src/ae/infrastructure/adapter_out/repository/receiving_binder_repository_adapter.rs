use crate::application::error::AppError;
use crate::application::repository::ReceivingBinderRepository;
use crate::domain::binder::{BinderId, ReceivingBinder};
use crate::domain::error::FunctionalError;
use crate::domain::user::UserId;
use async_trait::async_trait;
use sqlx::{Pool, Postgres};

pub struct ReceivingBinderRepositoryAdapter {
    pool: Pool<Postgres>,
}

impl ReceivingBinderRepositoryAdapter {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ReceivingBinderRepository for ReceivingBinderRepositoryAdapter {
    #[tracing::instrument(name = "receiving_binder_repo.find", skip_all, fields(sentry.op = "db"))]
    async fn find(&self, user_id: &UserId) -> Result<Option<ReceivingBinder>, AppError> {
        let row = sqlx::query!(
            r#"SELECT b.id, b.name
                FROM users u
                         JOIN binder b ON b.id = u.receiving_binder_id
                WHERE u.id = $1"#,
            user_id.as_str(),
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| ReceivingBinder {
            id: BinderId(row.id),
            name: row.name,
        }))
    }

    #[tracing::instrument(name = "receiving_binder_repo.set", skip_all, fields(sentry.op = "db"))]
    async fn set(
        &self,
        user_id: &UserId,
        binder_id: Option<BinderId>,
    ) -> Result<Option<ReceivingBinder>, AppError> {
        sqlx::query!(
            "UPDATE users SET receiving_binder_id = $2 WHERE id = $1",
            user_id.as_str(),
            binder_id.map(|id| id.0),
        )
        .execute(&self.pool)
        .await
        .map_err(|err| match err {
            // The binder is gone: absent from the start, or deleted since the service looked.
            sqlx::Error::Database(db_err) if db_err.is_foreign_key_violation() => {
                FunctionalError::BinderNotFound.into()
            }
            other => AppError::from(other),
        })?;

        self.find(user_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::adapter_out::repository::common_repository_tests::{
        insert_binder, insert_user,
    };
    use sqlx::PgPool;
    use uuid::Uuid;

    fn alice() -> UserId {
        UserId::new("user_a")
    }

    async fn seed(pool: &PgPool) {
        insert_user(pool, "user_a", "alice").await;
        insert_user(pool, "user_b", "bob").await;
    }

    #[sqlx::test]
    async fn there_is_no_receiving_binder_until_one_is_chosen(pool: PgPool) {
        seed(&pool).await;
        let repository = ReceivingBinderRepositoryAdapter::new(pool);

        assert_eq!(repository.find(&alice()).await.unwrap(), None);
    }

    #[sqlx::test]
    async fn choosing_a_binder_returns_and_stores_it(pool: PgPool) {
        seed(&pool).await;
        let binder = BinderId(insert_binder(&pool, "user_a", "Reçus").await);
        let repository = ReceivingBinderRepositoryAdapter::new(pool);
        let expected = Some(ReceivingBinder {
            id: binder,
            name: "Reçus".to_string(),
        });

        assert_eq!(
            repository.set(&alice(), Some(binder)).await.unwrap(),
            expected
        );
        assert_eq!(repository.find(&alice()).await.unwrap(), expected);
    }

    #[sqlx::test]
    async fn an_absent_binder_goes_back_to_the_default(pool: PgPool) {
        seed(&pool).await;
        let binder = BinderId(insert_binder(&pool, "user_a", "Reçus").await);
        let repository = ReceivingBinderRepositoryAdapter::new(pool);
        repository.set(&alice(), Some(binder)).await.unwrap();

        assert_eq!(repository.set(&alice(), None).await.unwrap(), None);
        assert_eq!(repository.find(&alice()).await.unwrap(), None);
    }

    #[sqlx::test]
    async fn an_unknown_binder_is_binder_not_found_and_changes_nothing(pool: PgPool) {
        seed(&pool).await;
        let own = BinderId(insert_binder(&pool, "user_a", "Reçus").await);
        let repository = ReceivingBinderRepositoryAdapter::new(pool);
        repository.set(&alice(), Some(own)).await.unwrap();

        let error = repository
            .set(&alice(), Some(BinderId(Uuid::new_v4())))
            .await
            .unwrap_err();

        assert_eq!(error.code(), "binder_not_found");
        assert_eq!(repository.find(&alice()).await.unwrap().unwrap().id, own);
    }

    #[sqlx::test]
    async fn deleting_the_binder_resets_the_choice(pool: PgPool) {
        seed(&pool).await;
        let binder = BinderId(insert_binder(&pool, "user_a", "Reçus").await);
        let repository = ReceivingBinderRepositoryAdapter::new(pool.clone());
        repository.set(&alice(), Some(binder)).await.unwrap();

        sqlx::query("DELETE FROM binder WHERE id = $1")
            .bind(binder.0)
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(repository.find(&alice()).await.unwrap(), None);
    }

    #[sqlx::test]
    async fn renaming_the_binder_keeps_the_choice(pool: PgPool) {
        seed(&pool).await;
        let binder = BinderId(insert_binder(&pool, "user_a", "Reçus").await);
        let repository = ReceivingBinderRepositoryAdapter::new(pool.clone());
        repository.set(&alice(), Some(binder)).await.unwrap();

        sqlx::query("UPDATE binder SET name = 'Renommé' WHERE id = $1")
            .bind(binder.0)
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(
            repository.find(&alice()).await.unwrap(),
            Some(ReceivingBinder {
                id: binder,
                name: "Renommé".to_string()
            })
        );
    }
}
