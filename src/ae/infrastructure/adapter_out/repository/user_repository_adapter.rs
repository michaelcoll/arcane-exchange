use crate::application::error::AppError;
use crate::application::repository::UserRepository;
use crate::domain::error::FunctionalError;
use crate::domain::user::{CollectionVisibility, User, UserId, UserSuggestion};
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
            r#"INSERT INTO users (id, username, image_url)
                VALUES ($1, $2, $3)
                ON CONFLICT (id)
                    DO UPDATE
                    SET username = $2,
                        image_url = COALESCE($3, users.image_url)"#,
            user.id.as_str(),
            username,
            user.avatar_url,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, AppError> {
        let row = sqlx::query_as!(
            UserEntity,
            "SELECT id, username, image_url FROM users WHERE id = $1",
            id.as_str()
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(User::from))
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>, AppError> {
        let row = sqlx::query_as!(
            UserEntity,
            "SELECT id, username, image_url FROM users WHERE LOWER(username) = LOWER($1)",
            username
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(User::from))
    }

    async fn autocomplete(&self, query: &str, limit: i64) -> Result<Vec<UserSuggestion>, AppError> {
        // Inner join on `v_tradable_entry`: a user who offers nothing to trade (private,
        // no binder selected, every rarity closed) is never suggested — see
        // `.agents/database-schema.instructions.md`.
        let rows = sqlx::query_as!(
            UserSuggestionEntity,
            r#"
            SELECT u.username,
                   SUM(t.proposed_quantity)::BIGINT AS "card_count!"
            FROM users u
            JOIN v_tradable_entry t ON t.user_id = u.id
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

    async fn get_visibility(&self, id: &UserId) -> Result<Option<CollectionVisibility>, AppError> {
        let row = sqlx::query!("SELECT visibility FROM users WHERE id = $1", id.as_str())
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| CollectionVisibility::from_db_str(&r.visibility)))
    }

    async fn set_visibility(
        &self,
        id: &UserId,
        visibility: CollectionVisibility,
    ) -> Result<bool, AppError> {
        let result = sqlx::query!(
            "UPDATE users SET visibility = $2 WHERE id = $1",
            id.as_str(),
            visibility.as_db_str(),
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::adapter_out::repository::common_repository_tests::{
        insert_card_with_rarity, insert_card_without_cardmarket_id, insert_collection_entry,
        insert_collection_entry_with_binder, insert_rarity_filter, insert_set,
        insert_trading_binder, insert_user, insert_user_with_visibility,
    };
    use chrono::Utc;
    use sqlx::PgPool;

    fn make_user(id: &str, username: &str) -> User {
        User::new(id.to_string(), None, Some(username.to_string()), None)
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
    async fn upsert_persists_avatar_and_keeps_existing_value_when_claim_missing(pool: PgPool) {
        let adapter = UserRepositoryAdapter::new(pool.clone());

        let with_avatar = User::new(
            "user_4".to_string(),
            None,
            Some("carol".to_string()),
            Some("https://img.example.com/avatar.png".to_string()),
        );
        adapter.upsert(&with_avatar).await.unwrap();

        let user = adapter
            .find_by_id(&UserId::new("user_4"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            user.avatar_url,
            Some("https://img.example.com/avatar.png".to_string())
        );

        // Login sans claim image_url : la valeur existante est conservée.
        adapter.upsert(&make_user("user_4", "carol")).await.unwrap();
        let user = adapter
            .find_by_id(&UserId::new("user_4"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            user.avatar_url,
            Some("https://img.example.com/avatar.png".to_string())
        );

        // Un nouveau claim remplace la valeur.
        let new_avatar = User::new(
            "user_4".to_string(),
            None,
            Some("carol".to_string()),
            Some("https://img.example.com/new-avatar.png".to_string()),
        );
        adapter.upsert(&new_avatar).await.unwrap();
        let user = adapter
            .find_by_id(&UserId::new("user_4"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            user.avatar_url,
            Some("https://img.example.com/new-avatar.png".to_string())
        );
    }

    #[sqlx::test]
    async fn should_return_wrong_format_error_when_username_missing(pool: PgPool) {
        let adapter = UserRepositoryAdapter::new(pool);
        let user = User::new("user_3".to_string(), None, None, None);

        let result = adapter.upsert(&user).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Functional(FunctionalError::WrongFormat(msg)) => {
                assert_eq!(msg, "Missing username claim in token")
            }
            _ => panic!("Expected WrongFormat"),
        }
    }

    // --- find_by_username ---

    #[sqlx::test]
    async fn find_by_username_returns_user_on_exact_case_match(pool: PgPool) {
        insert_user(&pool, "user_bob", "bob").await;

        let adapter = UserRepositoryAdapter::new(pool);
        let result = adapter.find_by_username("bob").await.unwrap().unwrap();

        assert_eq!(result.id, UserId::new("user_bob"));
    }

    #[sqlx::test]
    async fn find_by_username_is_case_insensitive(pool: PgPool) {
        insert_user(&pool, "user_bob", "bob").await;

        let adapter = UserRepositoryAdapter::new(pool);
        let result = adapter.find_by_username("Bob").await.unwrap().unwrap();

        assert_eq!(result.id, UserId::new("user_bob"));
    }

    #[sqlx::test]
    async fn find_by_username_returns_none_when_not_found(pool: PgPool) {
        let adapter = UserRepositoryAdapter::new(pool);
        let result = adapter.find_by_username("nobody").await.unwrap();

        assert!(result.is_none());
    }

    // --- autocomplete ---

    #[sqlx::test]
    async fn autocomplete_returns_users_matching_substring(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card_without_cardmarket_id(&pool, "TST", "1", "en", false, "Card A").await;
        insert_user_with_visibility(&pool, "user_alice", "alice", "public").await;
        insert_collection_entry(
            &pool,
            "TST",
            "1",
            "en",
            false,
            "user_alice",
            1,
            100,
            Utc::now(),
        )
        .await;
        insert_user(&pool, "user_bob", "bob").await;

        let adapter = UserRepositoryAdapter::new(pool);
        let result = adapter.autocomplete("ali", 10).await.unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].username, "alice");
    }

    #[sqlx::test]
    async fn autocomplete_is_case_insensitive(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card_without_cardmarket_id(&pool, "TST", "1", "en", false, "Card A").await;
        insert_user_with_visibility(&pool, "user_alice", "Alice", "public").await;
        insert_collection_entry(
            &pool,
            "TST",
            "1",
            "en",
            false,
            "user_alice",
            1,
            100,
            Utc::now(),
        )
        .await;

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
        insert_set(&pool, "TST").await;
        insert_card_without_cardmarket_id(&pool, "TST", "1", "en", false, "Card A").await;
        insert_user_with_visibility(&pool, "user_alice", "alice", "public").await;
        insert_user_with_visibility(&pool, "user_malice", "malice", "public").await;
        insert_collection_entry(
            &pool,
            "TST",
            "1",
            "en",
            false,
            "user_alice",
            1,
            100,
            Utc::now(),
        )
        .await;
        insert_collection_entry(
            &pool,
            "TST",
            "1",
            "en",
            false,
            "user_malice",
            1,
            100,
            Utc::now(),
        )
        .await;

        let adapter = UserRepositoryAdapter::new(pool);
        let result = adapter.autocomplete("alice", 10).await.unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].username, "alice");
    }

    #[sqlx::test]
    async fn autocomplete_returns_empty_for_no_match(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card_without_cardmarket_id(&pool, "TST", "1", "en", false, "Card A").await;
        insert_user_with_visibility(&pool, "user_alice", "alice", "public").await;
        insert_collection_entry(
            &pool,
            "TST",
            "1",
            "en",
            false,
            "user_alice",
            1,
            100,
            Utc::now(),
        )
        .await;

        let adapter = UserRepositoryAdapter::new(pool);
        let result = adapter.autocomplete("xyz", 10).await.unwrap();

        assert!(result.is_empty());
    }

    #[sqlx::test]
    async fn autocomplete_respects_limit(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card_without_cardmarket_id(&pool, "TST", "1", "en", false, "Card A").await;
        for i in 0..15 {
            let id = format!("user_ali_{i}");
            let username = format!("ali_{i}");
            insert_user_with_visibility(&pool, &id, &username, "public").await;
            insert_collection_entry(&pool, "TST", "1", "en", false, &id, 1, 100, Utc::now()).await;
        }

        let adapter = UserRepositoryAdapter::new(pool);
        let result = adapter.autocomplete("ali", 10).await.unwrap();

        assert_eq!(result.len(), 10);
    }

    #[sqlx::test]
    async fn autocomplete_sums_quantities_across_multiple_cards(pool: PgPool) {
        insert_user_with_visibility(&pool, "user_alice", "alice", "public").await;
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
    async fn autocomplete_excludes_user_without_any_tradable_card(pool: PgPool) {
        insert_user_with_visibility(&pool, "user_alice", "alice", "public").await;

        let adapter = UserRepositoryAdapter::new(pool);
        let result = adapter.autocomplete("ali", 10).await.unwrap();

        assert!(result.is_empty());
    }

    #[sqlx::test]
    async fn autocomplete_excludes_private_user(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card_without_cardmarket_id(&pool, "TST", "1", "en", false, "Card A").await;
        insert_user_with_visibility(&pool, "user_bob", "bob", "private").await;
        insert_collection_entry(
            &pool,
            "TST",
            "1",
            "en",
            false,
            "user_bob",
            40,
            100,
            Utc::now(),
        )
        .await;

        let adapter = UserRepositoryAdapter::new(pool);
        let result = adapter.autocomplete("bob", 10).await.unwrap();

        assert!(result.is_empty());
    }

    #[sqlx::test]
    async fn autocomplete_excludes_trade_user_without_binder_selected(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card_without_cardmarket_id(&pool, "TST", "1", "en", false, "Card A").await;
        insert_user_with_visibility(&pool, "user_bob", "bob", "trade").await;
        insert_collection_entry(
            &pool,
            "TST",
            "1",
            "en",
            false,
            "user_bob",
            40,
            100,
            Utc::now(),
        )
        .await;

        let adapter = UserRepositoryAdapter::new(pool);
        let result = adapter.autocomplete("bob", 10).await.unwrap();

        assert!(result.is_empty());
    }

    #[sqlx::test]
    async fn autocomplete_trade_user_card_count_is_the_proposed_quantity(pool: PgPool) {
        // Bob owns 40 copies total, but only 9 sit in his selected "Trade Binder" (the rest
        // are in an unselected binder) and he keeps 4 of that rarity — proposed = 9 - 4 = 5.
        insert_set(&pool, "TST").await;
        insert_card_with_rarity(&pool, "TST", "1", "EN", false, "Card A", 1, "R").await;
        insert_user_with_visibility(&pool, "user_bob", "bob", "trade").await;
        insert_trading_binder(&pool, "user_bob", "Trade Binder").await;
        insert_rarity_filter(&pool, "user_bob", "R", true, 4).await;
        insert_collection_entry_with_binder(
            &pool,
            "TST",
            "1",
            "EN",
            false,
            "user_bob",
            9,
            100,
            Utc::now(),
            Some("Trade Binder"),
        )
        .await;
        insert_collection_entry_with_binder(
            &pool,
            "TST",
            "1",
            "EN",
            false,
            "user_bob",
            31,
            100,
            Utc::now(),
            Some("Bulk Binder"),
        )
        .await;

        let adapter = UserRepositoryAdapter::new(pool);
        let result = adapter.autocomplete("bob", 10).await.unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].card_count, 5);
    }

    #[sqlx::test]
    async fn autocomplete_public_user_card_count_is_the_total_owned(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card_without_cardmarket_id(&pool, "TST", "1", "en", false, "Card A").await;
        insert_user_with_visibility(&pool, "user_bob", "bob", "public").await;
        insert_collection_entry(
            &pool,
            "TST",
            "1",
            "en",
            false,
            "user_bob",
            40,
            100,
            Utc::now(),
        )
        .await;

        let adapter = UserRepositoryAdapter::new(pool);
        let result = adapter.autocomplete("bob", 10).await.unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].card_count, 40);
    }

    // --- get_visibility / set_visibility ---

    #[sqlx::test]
    async fn get_visibility_returns_default_private_for_newly_inserted_user(pool: PgPool) {
        insert_user(&pool, "user_alice", "alice").await;

        let adapter = UserRepositoryAdapter::new(pool);
        let result = adapter
            .get_visibility(&UserId::new("user_alice"))
            .await
            .unwrap();

        assert_eq!(result, Some(CollectionVisibility::Private));
    }

    #[sqlx::test]
    async fn get_visibility_returns_stored_value_after_update(pool: PgPool) {
        insert_user(&pool, "user_alice", "alice").await;

        let adapter = UserRepositoryAdapter::new(pool);
        adapter
            .set_visibility(&UserId::new("user_alice"), CollectionVisibility::Public)
            .await
            .unwrap();
        let result = adapter
            .get_visibility(&UserId::new("user_alice"))
            .await
            .unwrap();

        assert_eq!(result, Some(CollectionVisibility::Public));
    }

    #[sqlx::test]
    async fn get_visibility_returns_none_for_unknown_user(pool: PgPool) {
        let adapter = UserRepositoryAdapter::new(pool);
        let result = adapter
            .get_visibility(&UserId::new("nobody"))
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[sqlx::test]
    async fn set_visibility_returns_true_and_persists_new_value(pool: PgPool) {
        insert_user(&pool, "user_alice", "alice").await;

        let adapter = UserRepositoryAdapter::new(pool);
        let updated = adapter
            .set_visibility(&UserId::new("user_alice"), CollectionVisibility::Trade)
            .await
            .unwrap();

        assert!(updated);
        assert_eq!(
            adapter
                .get_visibility(&UserId::new("user_alice"))
                .await
                .unwrap(),
            Some(CollectionVisibility::Trade)
        );
    }

    #[sqlx::test]
    async fn set_visibility_returns_false_for_unknown_user(pool: PgPool) {
        let adapter = UserRepositoryAdapter::new(pool);
        let updated = adapter
            .set_visibility(&UserId::new("nobody"), CollectionVisibility::Public)
            .await
            .unwrap();

        assert!(!updated);
    }
}
