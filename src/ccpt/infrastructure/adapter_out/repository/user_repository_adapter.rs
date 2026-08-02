use crate::application::error::AppError;
use crate::application::repository::UserRepository;
use crate::domain::error::FunctionalError;
use crate::domain::user::{User, UserId, UserSuggestion};
use crate::infrastructure::adapter_out::repository::entities::{UserEntity, UserSuggestionEntity};
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

    async fn autocomplete(&self, query: &str, limit: i64) -> Result<Vec<UserSuggestion>, AppError> {
        let rows = sqlx::query_as!(
            UserSuggestionEntity,
            r#"
            SELECT u.username,
                   COALESCE(SUM(ce.quantity), 0)::BIGINT AS "card_count!"
            FROM users u
            LEFT JOIN collection_entry ce ON ce.user_id = u.id
            WHERE LOWER(u.username) ILIKE '%' || LOWER($1) || '%'
               OR LOWER($1) <% LOWER(u.username)
            GROUP BY u.id, u.username
            ORDER BY word_similarity(LOWER($1), LOWER(u.username)) DESC
            LIMIT $2
            "#,
            query,
            limit,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(UserSuggestion::from).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::adapter_out::repository::common_repository_tests::{
        insert_card_without_cardmarket_id, insert_collection_entry, insert_set, insert_user,
    };
    use chrono::Utc;
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

    // --- autocomplete ---

    #[sqlx::test]
    async fn autocomplete_returns_users_matching_substring(pool: PgPool) {
        insert_user(&pool, "user_alice", "alice").await;
        insert_user(&pool, "user_bob", "bob").await;

        let adapter = UserRepositoryAdapter::new(pool);
        let result = adapter.autocomplete("ali", 10).await.unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].username, "alice");
    }

    #[sqlx::test]
    async fn autocomplete_is_case_insensitive(pool: PgPool) {
        insert_user(&pool, "user_alice", "Alice").await;

        let adapter = UserRepositoryAdapter::new(pool);

        let upper = adapter.autocomplete("ALI", 10).await.unwrap();
        assert_eq!(upper.len(), 1);
        assert_eq!(upper[0].username, "Alice");

        let lower = adapter.autocomplete("ali", 10).await.unwrap();
        assert_eq!(lower.len(), 1);
        assert_eq!(lower[0].username, "Alice");
    }

    #[sqlx::test]
    async fn autocomplete_orders_results_by_similarity(pool: PgPool) {
        insert_user(&pool, "user_alice", "alice").await;
        insert_user(&pool, "user_malice", "malice").await;

        let adapter = UserRepositoryAdapter::new(pool);
        let result = adapter.autocomplete("alice", 10).await.unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].username, "alice");
    }

    #[sqlx::test]
    async fn autocomplete_returns_empty_for_no_match(pool: PgPool) {
        insert_user(&pool, "user_alice", "alice").await;

        let adapter = UserRepositoryAdapter::new(pool);
        let result = adapter.autocomplete("xyz", 10).await.unwrap();

        assert!(result.is_empty());
    }

    #[sqlx::test]
    async fn autocomplete_respects_limit(pool: PgPool) {
        for i in 0..15 {
            insert_user(&pool, &format!("user_ali_{i}"), &format!("ali_{i}")).await;
        }

        let adapter = UserRepositoryAdapter::new(pool);
        let result = adapter.autocomplete("ali", 10).await.unwrap();

        assert_eq!(result.len(), 10);
    }

    #[sqlx::test]
    async fn autocomplete_sums_quantities_across_multiple_cards(pool: PgPool) {
        insert_user(&pool, "user_alice", "alice").await;
        insert_set(&pool, "TST").await;
        insert_card_without_cardmarket_id(&pool, "TST", "1", "en", false, "Card A").await;
        insert_card_without_cardmarket_id(&pool, "TST", "2", "en", false, "Card B").await;
        insert_collection_entry(
            &pool,
            "TST",
            "1",
            "en",
            false,
            "user_alice",
            3,
            100,
            Utc::now(),
        )
        .await;
        insert_collection_entry(
            &pool,
            "TST",
            "2",
            "en",
            false,
            "user_alice",
            2,
            200,
            Utc::now(),
        )
        .await;

        let adapter = UserRepositoryAdapter::new(pool);
        let result = adapter.autocomplete("ali", 10).await.unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].card_count, 5);
    }

    #[sqlx::test]
    async fn autocomplete_returns_zero_card_count_for_user_without_cards(pool: PgPool) {
        insert_user(&pool, "user_alice", "alice").await;

        let adapter = UserRepositoryAdapter::new(pool);
        let result = adapter.autocomplete("ali", 10).await.unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].card_count, 0);
    }
}
