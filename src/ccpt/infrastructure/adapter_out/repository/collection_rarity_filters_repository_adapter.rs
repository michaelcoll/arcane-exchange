use crate::application::error::AppError;
use crate::application::repository::RarityTradeFilterRepository;
use crate::domain::rarity_code::RarityCode;
use crate::domain::rarity_trade_filter::{RarityTradeFilter, RarityTradeFilterRule};
use crate::domain::user::UserId;
use async_trait::async_trait;
use sqlx::{Pool, Postgres};

pub struct CollectionRarityFiltersRepositoryAdapter {
    pool: Pool<Postgres>,
}

impl CollectionRarityFiltersRepositoryAdapter {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RarityTradeFilterRepository for CollectionRarityFiltersRepositoryAdapter {
    async fn list_with_counts(&self, user_id: &UserId) -> Result<Vec<RarityTradeFilter>, AppError> {
        let rows = sqlx::query!(
            r#"
            SELECT
                c.rarity AS "rarity!",
                COALESCE(f.is_open, FALSE) AS "is_open!",
                COALESCE(f.kept_copies, 0::SMALLINT) AS "kept_copies!",
                SUM(ce.quantity)::BIGINT AS "copies!",
                SUM(
                    CASE WHEN COALESCE(f.is_open, FALSE)
                        THEN GREATEST(ce.quantity - COALESCE(f.kept_copies, 0::SMALLINT)::INT, 0)
                        ELSE 0
                    END
                )::BIGINT AS "proposed!"
            FROM collection_entry ce
            JOIN card c
                ON  c.set_code         = ce.set_code
                AND c.collector_number = ce.collector_number
                AND c.language_code    = ce.language_code
                AND c.foil             = ce.foil
            JOIN trading_binders tb
                ON  tb.user_id     = ce.user_id
                AND tb.binder_name = ce.binder_name
            LEFT JOIN collection_rarity_filters f
                ON  f.user_id = ce.user_id
                AND f.rarity  = c.rarity
            WHERE ce.user_id = $1
            GROUP BY c.rarity, f.is_open, f.kept_copies
            ORDER BY CASE c.rarity
                WHEN 'M' THEN 1
                WHEN 'R' THEN 2
                WHEN 'U' THEN 3
                WHEN 'C' THEN 4
                WHEN 'S' THEN 5
                ELSE 6
            END
            "#,
            user_id.as_str()
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| {
                Ok(RarityTradeFilter {
                    rarity: RarityCode::try_new(&r.rarity)?,
                    is_open: r.is_open,
                    kept_copies: u8::try_from(r.kept_copies).unwrap_or(0),
                    copies: r.copies as u64,
                    proposed: r.proposed as u64,
                })
            })
            .collect()
    }

    async fn upsert(&self, user_id: &UserId, rule: &RarityTradeFilterRule) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO collection_rarity_filters (user_id, rarity, is_open, kept_copies)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (user_id, rarity) DO UPDATE
            SET is_open = EXCLUDED.is_open, kept_copies = EXCLUDED.kept_copies
            "#,
            user_id.as_str(),
            rule.rarity.to_string(),
            rule.is_open,
            i16::from(rule.kept_copies)
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
        insert_card_with_rarity, insert_collection_entry_with_binder, insert_set,
        insert_trading_binder, insert_user,
    };
    use chrono::Utc;
    use sqlx::PgPool;

    #[sqlx::test]
    async fn list_is_empty_for_empty_collection(pool: PgPool) {
        insert_user(&pool, "user-1", "User1").await;

        let adapter = CollectionRarityFiltersRepositoryAdapter::new(pool);
        let result = adapter
            .list_with_counts(&UserId::new("user-1"))
            .await
            .unwrap();

        assert!(result.is_empty());
    }

    #[sqlx::test]
    async fn list_is_empty_when_no_binder_is_selected(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card_with_rarity(&pool, "TST", "1", "en", false, "Card A", 1, "R").await;
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
            Some("Trade Binder"),
        )
        .await;

        let adapter = CollectionRarityFiltersRepositoryAdapter::new(pool);
        let result = adapter
            .list_with_counts(&UserId::new("user-1"))
            .await
            .unwrap();

        assert!(result.is_empty());
    }

    #[sqlx::test]
    async fn list_returns_owned_rarities_closed_by_default(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card_with_rarity(&pool, "TST", "1", "en", false, "Rare Card", 1, "R").await;
        insert_card_with_rarity(&pool, "TST", "2", "en", false, "Common Card", 2, "C").await;
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
            50,
            Utc::now(),
            Some("Trade Binder"),
        )
        .await;
        insert_trading_binder(&pool, "user-1", "Trade Binder").await;

        let adapter = CollectionRarityFiltersRepositoryAdapter::new(pool);
        let result = adapter
            .list_with_counts(&UserId::new("user-1"))
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].rarity, RarityCode::R);
        assert!(!result[0].is_open);
        assert_eq!(result[0].kept_copies, 0);
        assert_eq!(result[0].copies, 3);
        assert_eq!(result[0].proposed, 0);
        assert_eq!(result[1].rarity, RarityCode::C);
        assert_eq!(result[1].copies, 2);
        assert_eq!(result[1].proposed, 0);
    }

    #[sqlx::test]
    async fn list_computes_proposed_when_rarity_is_open(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card_with_rarity(&pool, "TST", "1", "en", false, "Rare Card A", 1, "R").await;
        insert_card_with_rarity(&pool, "TST", "2", "en", false, "Rare Card B", 2, "R").await;
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
            1,
            100,
            Utc::now(),
            Some("Trade Binder"),
        )
        .await;
        insert_trading_binder(&pool, "user-1", "Trade Binder").await;

        let adapter = CollectionRarityFiltersRepositoryAdapter::new(pool);
        adapter
            .upsert(
                &UserId::new("user-1"),
                &RarityTradeFilterRule {
                    rarity: RarityCode::R,
                    is_open: true,
                    kept_copies: 1,
                },
            )
            .await
            .unwrap();

        let result = adapter
            .list_with_counts(&UserId::new("user-1"))
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert!(result[0].is_open);
        assert_eq!(result[0].kept_copies, 1);
        assert_eq!(result[0].copies, 4);
        // 3 - 1 = 2 (kept once), 1 - 1 = 0 (nothing left) => 2
        assert_eq!(result[0].proposed, 2);
    }

    #[sqlx::test]
    async fn list_computes_zero_proposed_when_rarity_is_closed(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card_with_rarity(&pool, "TST", "1", "en", false, "Rare Card A", 1, "R").await;
        insert_card_with_rarity(&pool, "TST", "2", "en", false, "Rare Card B", 2, "R").await;
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
            1,
            100,
            Utc::now(),
            Some("Trade Binder"),
        )
        .await;
        insert_trading_binder(&pool, "user-1", "Trade Binder").await;

        let adapter = CollectionRarityFiltersRepositoryAdapter::new(pool);
        adapter
            .upsert(
                &UserId::new("user-1"),
                &RarityTradeFilterRule {
                    rarity: RarityCode::R,
                    is_open: false,
                    kept_copies: 1,
                },
            )
            .await
            .unwrap();

        let result = adapter
            .list_with_counts(&UserId::new("user-1"))
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert!(!result[0].is_open);
        assert_eq!(result[0].copies, 4);
        assert_eq!(result[0].proposed, 0);
    }

    #[sqlx::test]
    async fn list_excludes_cards_from_unselected_binders(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card_with_rarity(&pool, "TST", "1", "en", false, "Rare Card", 1, "R").await;
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
            Some("Bulk"),
        )
        .await;
        insert_trading_binder(&pool, "user-1", "Trade Binder").await;

        let adapter = CollectionRarityFiltersRepositoryAdapter::new(pool);
        let result = adapter
            .list_with_counts(&UserId::new("user-1"))
            .await
            .unwrap();

        assert!(result.is_empty());
    }

    #[sqlx::test]
    async fn list_excludes_cards_without_a_binder(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card_with_rarity(&pool, "TST", "1", "en", false, "Rare Card", 1, "R").await;
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
        insert_trading_binder(&pool, "user-1", "Trade Binder").await;

        let adapter = CollectionRarityFiltersRepositoryAdapter::new(pool);
        let result = adapter
            .list_with_counts(&UserId::new("user-1"))
            .await
            .unwrap();

        assert!(result.is_empty());
    }

    #[sqlx::test]
    async fn list_includes_special_rarity_when_owned(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card_with_rarity(&pool, "TST", "1", "en", false, "Special Card", 1, "S").await;
        insert_user(&pool, "user-1", "User1").await;
        insert_collection_entry_with_binder(
            &pool,
            "TST",
            "1",
            "en",
            false,
            "user-1",
            1,
            100,
            Utc::now(),
            Some("Trade Binder"),
        )
        .await;
        insert_trading_binder(&pool, "user-1", "Trade Binder").await;

        let adapter = CollectionRarityFiltersRepositoryAdapter::new(pool);
        let result = adapter
            .list_with_counts(&UserId::new("user-1"))
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].rarity, RarityCode::S);
    }

    #[sqlx::test]
    async fn upsert_then_list_reflects_the_new_rule_and_recomputed_proposed(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card_with_rarity(&pool, "TST", "1", "en", false, "Mythic Card", 1, "M").await;
        insert_user(&pool, "user-1", "User1").await;
        insert_collection_entry_with_binder(
            &pool,
            "TST",
            "1",
            "en",
            false,
            "user-1",
            5,
            100,
            Utc::now(),
            Some("Trade Binder"),
        )
        .await;
        insert_trading_binder(&pool, "user-1", "Trade Binder").await;

        let adapter = CollectionRarityFiltersRepositoryAdapter::new(pool.clone());
        adapter
            .upsert(
                &UserId::new("user-1"),
                &RarityTradeFilterRule {
                    rarity: RarityCode::M,
                    is_open: true,
                    kept_copies: 2,
                },
            )
            .await
            .unwrap();

        let count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) AS \"count!\" FROM collection_rarity_filters WHERE user_id = $1 AND rarity = 'M'",
            "user-1"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1);

        let result = adapter
            .list_with_counts(&UserId::new("user-1"))
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].rarity, RarityCode::M);
        assert!(result[0].is_open);
        assert_eq!(result[0].kept_copies, 2);
        assert_eq!(result[0].copies, 5);
        assert_eq!(result[0].proposed, 3);
    }

    #[sqlx::test]
    async fn upsert_is_idempotent_on_the_same_rarity(pool: PgPool) {
        insert_user(&pool, "user-1", "User1").await;

        let adapter = CollectionRarityFiltersRepositoryAdapter::new(pool.clone());
        adapter
            .upsert(
                &UserId::new("user-1"),
                &RarityTradeFilterRule {
                    rarity: RarityCode::M,
                    is_open: true,
                    kept_copies: 1,
                },
            )
            .await
            .unwrap();
        adapter
            .upsert(
                &UserId::new("user-1"),
                &RarityTradeFilterRule {
                    rarity: RarityCode::M,
                    is_open: true,
                    kept_copies: 2,
                },
            )
            .await
            .unwrap();

        let count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) AS \"count!\" FROM collection_rarity_filters WHERE user_id = $1",
            "user-1"
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(count, 1);
    }

    #[sqlx::test]
    async fn list_only_reflects_the_caller_own_rule(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card_with_rarity(&pool, "TST", "1", "en", false, "Rare Card", 1, "R").await;
        insert_user(&pool, "user-1", "User1").await;
        insert_user(&pool, "user-2", "User2").await;
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
            Some("Trade Binder"),
        )
        .await;
        insert_trading_binder(&pool, "user-1", "Trade Binder").await;
        insert_trading_binder(&pool, "user-2", "Trade Binder").await;

        let adapter = CollectionRarityFiltersRepositoryAdapter::new(pool);
        adapter
            .upsert(
                &UserId::new("user-1"),
                &RarityTradeFilterRule {
                    rarity: RarityCode::R,
                    is_open: true,
                    kept_copies: 0,
                },
            )
            .await
            .unwrap();

        let user1_result = adapter
            .list_with_counts(&UserId::new("user-1"))
            .await
            .unwrap();
        let user2_result = adapter
            .list_with_counts(&UserId::new("user-2"))
            .await
            .unwrap();

        assert!(user1_result[0].is_open);
        assert_eq!(user1_result[0].proposed, 2);
        assert!(!user2_result[0].is_open);
        assert_eq!(user2_result[0].proposed, 0);
    }

    #[sqlx::test]
    async fn list_only_returns_the_caller_data(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card_with_rarity(&pool, "TST", "1", "en", false, "Rare Card", 1, "R").await;
        insert_user(&pool, "user-1", "User1").await;
        insert_user(&pool, "user-2", "User2").await;
        insert_collection_entry_with_binder(
            &pool,
            "TST",
            "1",
            "en",
            false,
            "user-2",
            3,
            100,
            Utc::now(),
            Some("Trade Binder"),
        )
        .await;
        insert_trading_binder(&pool, "user-2", "Trade Binder").await;

        let adapter = CollectionRarityFiltersRepositoryAdapter::new(pool);
        let result = adapter
            .list_with_counts(&UserId::new("user-1"))
            .await
            .unwrap();

        assert!(result.is_empty());
    }
}
