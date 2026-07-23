use crate::application::error::AppError;
use crate::application::repository::UserRepository;
use crate::domain::error::FunctionalError;
use crate::domain::user::{User, UserId};
use crate::infrastructure::adapter_out::repository::entities::UserEntity;
use async_trait::async_trait;
use sqlx::{Pool, Postgres};

pub struct UserRepositoryAdapter {
    pool: Pool<Postgres>,
}

impl UserRepositoryAdapter {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for UserRepositoryAdapter {
    async fn upsert(&self, user: &User) -> Result<(), AppError> {
        let username = user.username.clone().ok_or_else(|| {
            FunctionalError::WrongFormat("Missing username claim in token".to_string())
        })?;

        sqlx::query!(
            r#"INSERT INTO users (id, username)
                VALUES ($1, $2)
                ON CONFLICT (id)
                    DO UPDATE
                    SET username = $2"#,
            user.id.as_str(),
            username,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, AppError> {
        let row = sqlx::query_as!(
            UserEntity,
            "SELECT id, username FROM users WHERE id = $1",
            id.as_str()
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(User::from))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    fn make_user(id: &str, username: &str) -> User {
        User::new(id.to_string(), None, Some(username.to_string()))
    }

    #[sqlx::test]
    async fn should_insert_new_user(pool: PgPool) {
        let adapter = UserRepositoryAdapter::new(pool.clone());

        let result = adapter.upsert(&make_user("user_1", "alice")).await;

        assert!(result.is_ok());
        let user = adapter
            .find_by_id(&UserId::new("user_1"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(user.id, UserId::new("user_1"));
        assert_eq!(user.username, Some("alice".to_string()));
    }

    #[sqlx::test]
    async fn should_update_username_on_conflict_without_duplicating(pool: PgPool) {
        let adapter = UserRepositoryAdapter::new(pool.clone());

        adapter.upsert(&make_user("user_2", "bob")).await.unwrap();
        adapter
            .upsert(&make_user("user_2", "bob-updated"))
            .await
            .unwrap();

        let user = adapter
            .find_by_id(&UserId::new("user_2"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(user.username, Some("bob-updated".to_string()));
    }

    #[sqlx::test]
    async fn should_return_wrong_format_error_when_username_missing(pool: PgPool) {
        let adapter = UserRepositoryAdapter::new(pool);
        let user = User::new("user_3".to_string(), None, None);

        let result = adapter.upsert(&user).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Functional(FunctionalError::WrongFormat(msg)) => {
                assert_eq!(msg, "Missing username claim in token")
            }
            _ => panic!("Expected WrongFormat"),
        }
    }
}
