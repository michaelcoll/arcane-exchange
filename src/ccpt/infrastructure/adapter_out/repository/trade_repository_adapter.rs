use crate::application::error::AppError;
use crate::application::repository::TradeRepository;
use crate::domain::card::CardId;
use crate::domain::trade::{Trade, TradeCard, TradeId, TradeStatus};
use crate::domain::user::UserId;
use crate::infrastructure::adapter_out::repository::entities::{TradeCardEntity, TradeEntity};
use async_trait::async_trait;
use sqlx::{Pool, Postgres};

pub struct TradeRepositoryAdapter {
    pool: Pool<Postgres>,
}

impl TradeRepositoryAdapter {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TradeRepository for TradeRepositoryAdapter {
    async fn find_collection_entry_quantity(
        &self,
        user_id: &UserId,
        card_id: &CardId,
    ) -> Result<Option<i32>, AppError> {
        let row = sqlx::query!(
            r#"SELECT quantity FROM collection_entry
                WHERE user_id = $1 AND set_code = $2 AND collector_number = $3
                  AND language_code = $4 AND foil = $5"#,
            user_id.as_str(),
            card_id.set_code.to_string(),
            card_id.collector_number,
            card_id.language_code.to_string(),
            card_id.foil,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.quantity))
    }

    async fn find_active_trade(
        &self,
        user_a: &UserId,
        user_b: &UserId,
    ) -> Result<Option<(TradeId, TradeStatus)>, AppError> {
        let row = sqlx::query!(
            r#"SELECT id, status FROM trade
                WHERE ((initiator_user_id = $1 AND respondent_user_id = $2)
                    OR (initiator_user_id = $2 AND respondent_user_id = $1))
                  AND status IN ('PENDING', 'ONE_ACCEPTED', 'FULLY_ACCEPTED')
                ORDER BY created_at ASC
                LIMIT 1"#,
            user_a.as_str(),
            user_b.as_str(),
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| (TradeId(r.id), TradeStatus::from_db_str(&r.status))))
    }

    async fn find_by_id(&self, id: TradeId) -> Result<Option<Trade>, AppError> {
        let row = sqlx::query_as!(
            TradeEntity,
            r#"SELECT id, initiator_user_id, respondent_user_id, status,
                    initiator_amount_due, respondent_amount_due,
                    initiator_accepted_at, respondent_accepted_at,
                    created_at, updated_at
                FROM trade WHERE id = $1"#,
            id.0
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Trade::from))
    }

    async fn find_trade_cards(&self, trade_id: TradeId) -> Result<Vec<TradeCard>, AppError> {
        let rows = sqlx::query_as!(
            TradeCardEntity,
            r#"SELECT set_code, collector_number, language_code, foil, owner_user_id, quantity
                FROM trade_card WHERE trade_id = $1"#,
            trade_id.0
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(TradeCard::from).collect())
    }

    async fn create(
        &self,
        id: TradeId,
        initiator_id: &UserId,
        respondent_id: &UserId,
        card_id: &CardId,
        quantity: u8,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;

        sqlx::query!(
            r#"INSERT INTO trade (id, initiator_user_id, respondent_user_id, status)
                VALUES ($1, $2, $3, 'PENDING')"#,
            id.0,
            initiator_id.as_str(),
            respondent_id.as_str(),
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            r#"INSERT INTO trade_card (trade_id, set_code, collector_number, language_code, foil, owner_user_id, quantity)
                VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
            id.0,
            card_id.set_code.to_string(),
            card_id.collector_number,
            card_id.language_code.to_string(),
            card_id.foil,
            respondent_id.as_str(),
            quantity as i32,
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }

    async fn merge_card_into_trade(
        &self,
        trade_id: TradeId,
        card_id: &CardId,
        owner_id: &UserId,
        quantity: u8,
        reopen_to_pending: bool,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;

        sqlx::query!(
            r#"INSERT INTO trade_card (trade_id, set_code, collector_number, language_code, foil, owner_user_id, quantity)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                ON CONFLICT (trade_id, set_code, collector_number, language_code, foil, owner_user_id)
                    DO UPDATE SET quantity = trade_card.quantity + EXCLUDED.quantity"#,
            trade_id.0,
            card_id.set_code.to_string(),
            card_id.collector_number,
            card_id.language_code.to_string(),
            card_id.foil,
            owner_id.as_str(),
            quantity as i32,
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            r#"UPDATE trade
                SET status = CASE WHEN $2 THEN 'PENDING' ELSE status END,
                    initiator_accepted_at = CASE WHEN $2 THEN NULL ELSE initiator_accepted_at END,
                    respondent_accepted_at = CASE WHEN $2 THEN NULL ELSE respondent_accepted_at END,
                    updated_at = NOW()
                WHERE id = $1"#,
            trade_id.0,
            reopen_to_pending,
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::language_code::LanguageCode;
    use crate::infrastructure::adapter_out::repository::common_repository_tests::{
        insert_card, insert_collection_entry, insert_trade, insert_trade_card, insert_user,
        mark_trade_accepted_by_both,
    };
    use sqlx::PgPool;

    fn make_card_id() -> CardId {
        CardId::new("FDN", "87", LanguageCode::FR, false)
    }

    #[sqlx::test]
    async fn find_collection_entry_quantity_returns_quantity_when_found(pool: PgPool) {
        insert_user(&pool, "user_b", "bob").await;
        insert_card(&pool, "FDN", "87", "FR", false, "Goblin Boarders", 1).await;
        insert_collection_entry(
            &pool,
            "FDN",
            "87",
            "FR",
            false,
            "user_b",
            3,
            100,
            chrono::Utc::now(),
        )
        .await;

        let repository = TradeRepositoryAdapter::new(pool);
        let result = repository
            .find_collection_entry_quantity(&UserId::new("user_b"), &make_card_id())
            .await
            .unwrap();

        assert_eq!(result, Some(3));
    }

    #[sqlx::test]
    async fn find_collection_entry_quantity_returns_none_when_not_found(pool: PgPool) {
        let repository = TradeRepositoryAdapter::new(pool);
        let result = repository
            .find_collection_entry_quantity(&UserId::new("user_unknown"), &make_card_id())
            .await
            .unwrap();

        assert_eq!(result, None);
    }

    #[sqlx::test]
    async fn find_active_trade_returns_none_when_no_trade(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;

        let repository = TradeRepositoryAdapter::new(pool);
        let result = repository
            .find_active_trade(&UserId::new("user_a"), &UserId::new("user_b"))
            .await
            .unwrap();

        assert_eq!(result, None);
    }

    #[sqlx::test]
    async fn find_active_trade_returns_pending_trade(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "PENDING").await;

        let repository = TradeRepositoryAdapter::new(pool);
        let result = repository
            .find_active_trade(&UserId::new("user_a"), &UserId::new("user_b"))
            .await
            .unwrap();

        assert_eq!(result, Some((TradeId(trade_id), TradeStatus::Pending)));
    }

    #[sqlx::test]
    async fn find_active_trade_returns_one_accepted_trade(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "ONE_ACCEPTED").await;

        let repository = TradeRepositoryAdapter::new(pool);
        let result = repository
            .find_active_trade(&UserId::new("user_a"), &UserId::new("user_b"))
            .await
            .unwrap();

        assert_eq!(result, Some((TradeId(trade_id), TradeStatus::OneAccepted)));
    }

    #[sqlx::test]
    async fn find_active_trade_returns_fully_accepted_trade(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "FULLY_ACCEPTED").await;

        let repository = TradeRepositoryAdapter::new(pool);
        let result = repository
            .find_active_trade(&UserId::new("user_a"), &UserId::new("user_b"))
            .await
            .unwrap();

        assert_eq!(
            result,
            Some((TradeId(trade_id), TradeStatus::FullyAccepted))
        );
    }

    #[sqlx::test]
    async fn find_active_trade_ignores_terminal_statuses(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_trade(&pool, uuid::Uuid::new_v4(), "user_a", "user_b", "COMPLETED").await;
        insert_trade(&pool, uuid::Uuid::new_v4(), "user_a", "user_b", "CLOSED").await;
        insert_trade(&pool, uuid::Uuid::new_v4(), "user_a", "user_b", "ABANDONED").await;

        let repository = TradeRepositoryAdapter::new(pool);
        let result = repository
            .find_active_trade(&UserId::new("user_a"), &UserId::new("user_b"))
            .await
            .unwrap();

        assert_eq!(result, None);
    }

    #[sqlx::test]
    async fn find_active_trade_matches_regardless_of_direction(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_b", "user_a", "PENDING").await;

        let repository = TradeRepositoryAdapter::new(pool);
        let result = repository
            .find_active_trade(&UserId::new("user_a"), &UserId::new("user_b"))
            .await
            .unwrap();

        assert_eq!(result, Some((TradeId(trade_id), TradeStatus::Pending)));
    }

    #[sqlx::test]
    async fn find_active_trade_picks_the_oldest_when_several_exist(pool: PgPool) {
        use crate::infrastructure::adapter_out::repository::common_repository_tests::insert_trade_with_created_at;

        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        let older_id = uuid::Uuid::new_v4();
        let newer_id = uuid::Uuid::new_v4();
        let now = chrono::Utc::now();
        insert_trade_with_created_at(&pool, newer_id, "user_a", "user_b", "PENDING", now).await;
        insert_trade_with_created_at(
            &pool,
            older_id,
            "user_a",
            "user_b",
            "PENDING",
            now - chrono::Duration::days(1),
        )
        .await;

        let repository = TradeRepositoryAdapter::new(pool);
        let result = repository
            .find_active_trade(&UserId::new("user_a"), &UserId::new("user_b"))
            .await
            .unwrap();

        assert_eq!(result, Some((TradeId(older_id), TradeStatus::Pending)));
    }

    #[sqlx::test]
    async fn merge_card_into_trade_adds_new_card_and_keeps_status(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_card(&pool, "FDN", "87", "FR", false, "Goblin Boarders", 1).await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "PENDING").await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        repository
            .merge_card_into_trade(
                TradeId(trade_id),
                &make_card_id(),
                &UserId::new("user_b"),
                2,
                false,
            )
            .await
            .unwrap();

        let trade_cards = repository
            .find_trade_cards(TradeId(trade_id))
            .await
            .unwrap();
        assert_eq!(trade_cards.len(), 1);
        assert_eq!(trade_cards[0].quantity, 2);

        let trade = repository
            .find_by_id(TradeId(trade_id))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(trade.status, TradeStatus::Pending);
    }

    #[sqlx::test]
    async fn merge_card_into_trade_reopens_one_accepted_trade(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_card(&pool, "FDN", "87", "FR", false, "Goblin Boarders", 1).await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "ONE_ACCEPTED").await;
        mark_trade_accepted_by_both(&pool, trade_id).await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        repository
            .merge_card_into_trade(
                TradeId(trade_id),
                &make_card_id(),
                &UserId::new("user_b"),
                1,
                true,
            )
            .await
            .unwrap();

        let trade = repository
            .find_by_id(TradeId(trade_id))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(trade.status, TradeStatus::Pending);
        assert_eq!(trade.initiator_accepted_at, None);
        assert_eq!(trade.respondent_accepted_at, None);
    }

    #[sqlx::test]
    async fn merge_card_into_trade_leaves_acceptance_timestamps_untouched_when_not_reopening(
        pool: PgPool,
    ) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_card(&pool, "FDN", "87", "FR", false, "Goblin Boarders", 1).await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "PENDING").await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        repository
            .merge_card_into_trade(
                TradeId(trade_id),
                &make_card_id(),
                &UserId::new("user_b"),
                1,
                false,
            )
            .await
            .unwrap();

        let trade = repository
            .find_by_id(TradeId(trade_id))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(trade.initiator_accepted_at, None);
        assert_eq!(trade.respondent_accepted_at, None);
    }

    #[sqlx::test]
    async fn merge_card_into_trade_increments_quantity_when_card_already_present(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_card(&pool, "FDN", "87", "FR", false, "Goblin Boarders", 1).await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "PENDING").await;
        insert_trade_card(&pool, trade_id, "FDN", "87", "FR", false, "user_b", 2).await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        repository
            .merge_card_into_trade(
                TradeId(trade_id),
                &make_card_id(),
                &UserId::new("user_b"),
                3,
                false,
            )
            .await
            .unwrap();

        let trade_cards = repository
            .find_trade_cards(TradeId(trade_id))
            .await
            .unwrap();
        assert_eq!(trade_cards.len(), 1);
        assert_eq!(trade_cards[0].quantity, 5);
    }

    #[sqlx::test]
    async fn merge_card_into_trade_updates_updated_at(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_card(&pool, "FDN", "87", "FR", false, "Goblin Boarders", 1).await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "PENDING").await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        let before = repository
            .find_by_id(TradeId(trade_id))
            .await
            .unwrap()
            .unwrap()
            .updated_at;

        repository
            .merge_card_into_trade(
                TradeId(trade_id),
                &make_card_id(),
                &UserId::new("user_b"),
                1,
                false,
            )
            .await
            .unwrap();

        let after = repository
            .find_by_id(TradeId(trade_id))
            .await
            .unwrap()
            .unwrap()
            .updated_at;

        assert!(after > before);
    }

    #[sqlx::test]
    async fn create_inserts_trade_and_trade_card(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_card(&pool, "FDN", "87", "FR", false, "Goblin Boarders", 1).await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        let id = TradeId::new();
        repository
            .create(
                id,
                &UserId::new("user_a"),
                &UserId::new("user_b"),
                &make_card_id(),
                2,
            )
            .await
            .unwrap();

        let trade = repository.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(trade.initiator_user_id, UserId::new("user_a"));
        assert_eq!(trade.respondent_user_id, UserId::new("user_b"));
        assert_eq!(trade.status, TradeStatus::Pending);
        assert_eq!(trade.initiator_amount_due, None);
        assert_eq!(trade.respondent_amount_due, None);

        let trade_cards = repository.find_trade_cards(id).await.unwrap();
        assert_eq!(trade_cards.len(), 1);
        let trade_card = &trade_cards[0];
        assert_eq!(trade_card.card_id, make_card_id());
        assert_eq!(trade_card.owner_user_id, UserId::new("user_b"));
        assert_eq!(trade_card.quantity, 2);
    }
}
