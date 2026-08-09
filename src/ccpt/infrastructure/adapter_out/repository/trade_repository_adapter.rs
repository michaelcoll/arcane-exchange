use crate::application::error::AppError;
use crate::application::repository::TradeRepository;
use crate::domain::card::CardId;
use crate::domain::trade::{
    PaginatedTrades, Trade, TradeCard, TradeCardDetail, TradeId, TradeListQuery, TradeStatus,
    TradeSummary,
};
use crate::domain::user::UserId;
use crate::infrastructure::adapter_out::repository::entities::{
    TradeCardDetailEntity, TradeCardEntity, TradeEntity, TradeSummaryEntity,
};
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
                    initiator_confirmed_at, respondent_confirmed_at,
                    initiator_rating, respondent_rating,
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

    async fn find_trade_cards_with_details(
        &self,
        trade_id: TradeId,
    ) -> Result<Vec<TradeCardDetail>, AppError> {
        let rows = sqlx::query_as!(
            TradeCardDetailEntity,
            r#"WITH last_price AS (
                    SELECT id_produit, MAX(date) AS last_date
                    FROM cardmarket_price
                    GROUP BY id_produit
                )
                SELECT tc.set_code AS "set_code!", tc.collector_number AS "collector_number!",
                       tc.language_code AS "language_code!", tc.foil AS "foil!",
                       tc.owner_user_id AS "owner_user_id!", tc.quantity AS "quantity!",
                       c.name AS "name!", c.scryfall_id AS "scryfall_id!", c.the_gatherer_id,
                       CASE WHEN c.foil THEN cmp.low_foil ELSE cmp.low END AS low,
                       CASE WHEN c.foil THEN cmp.trend_foil ELSE cmp.trend END AS trend,
                       CASE WHEN c.foil THEN cmp.avg_foil ELSE cmp.avg END AS avg
                FROM trade_card tc
                JOIN card c ON c.set_code = tc.set_code AND c.collector_number = tc.collector_number
                    AND c.language_code = tc.language_code AND c.foil = tc.foil
                LEFT JOIN last_price lp ON c.cardmarket_id = lp.id_produit
                LEFT JOIN cardmarket_price cmp ON c.cardmarket_id = cmp.id_produit AND cmp.date = lp.last_date
                WHERE tc.trade_id = $1"#,
            trade_id.0
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(TradeCardDetail::from).collect())
    }

    async fn list_trades(
        &self,
        caller_id: &UserId,
        query: TradeListQuery,
    ) -> Result<PaginatedTrades, AppError> {
        let statuses: Option<Vec<String>> = if query.statuses.is_empty() {
            None
        } else {
            Some(
                query
                    .statuses
                    .iter()
                    .map(|s| s.as_db_str().to_string())
                    .collect(),
            )
        };
        let limit = query.page_size as i64;
        let offset = (query.page * query.page_size) as i64;

        let rows = sqlx::query_as!(
            TradeSummaryEntity,
            r#"SELECT t.id, t.status, t.updated_at,
                    u.username AS partner_username,
                    COALESCE(SUM(tc.quantity) FILTER (WHERE tc.owner_user_id = $1), 0)::bigint AS "my_card_count!",
                    COALESCE(SUM(tc.quantity) FILTER (WHERE tc.owner_user_id != $1), 0)::bigint AS "partner_card_count!"
                FROM trade t
                JOIN users u ON u.id = CASE WHEN t.initiator_user_id = $1 THEN t.respondent_user_id ELSE t.initiator_user_id END
                LEFT JOIN trade_card tc ON tc.trade_id = t.id
                WHERE (t.initiator_user_id = $1 OR t.respondent_user_id = $1)
                    AND ($2::text[] IS NULL OR t.status = ANY($2))
                GROUP BY t.id, t.status, t.updated_at, u.username
                ORDER BY t.updated_at DESC
                LIMIT $3 OFFSET $4"#,
            caller_id.as_str(),
            statuses.as_deref(),
            limit,
            offset,
        )
        .fetch_all(&self.pool)
        .await?;

        let total = sqlx::query_scalar!(
            r#"SELECT COUNT(*) FROM trade t
                WHERE (t.initiator_user_id = $1 OR t.respondent_user_id = $1)
                    AND ($2::text[] IS NULL OR t.status = ANY($2))"#,
            caller_id.as_str(),
            statuses.as_deref(),
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(0);

        Ok(PaginatedTrades {
            items: rows.into_iter().map(TradeSummary::from).collect(),
            total: total as u64,
            page: query.page,
            page_size: query.page_size,
        })
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

    async fn accept(
        &self,
        trade_id: TradeId,
        is_initiator: bool,
    ) -> Result<Option<TradeStatus>, AppError> {
        let mut tx = self.pool.begin().await?;

        let row = sqlx::query!(
            r#"UPDATE trade
                SET initiator_accepted_at = CASE WHEN $2 THEN NOW() ELSE initiator_accepted_at END,
                    respondent_accepted_at = CASE WHEN NOT $2 THEN NOW() ELSE respondent_accepted_at END,
                    status = CASE
                        WHEN ($2 AND respondent_accepted_at IS NOT NULL)
                          OR (NOT $2 AND initiator_accepted_at IS NOT NULL)
                        THEN 'FULLY_ACCEPTED' ELSE 'ONE_ACCEPTED' END,
                    updated_at = NOW()
                WHERE id = $1
                  AND status IN ('PENDING', 'ONE_ACCEPTED')
                  AND ( ($2 AND initiator_accepted_at IS NULL) OR (NOT $2 AND respondent_accepted_at IS NULL) )
                RETURNING status"#,
            trade_id.0,
            is_initiator,
        )
        .fetch_optional(&mut *tx)
        .await?;

        let new_status = row.map(|r| TradeStatus::from_db_str(&r.status));

        // `ONE_ACCEPTED` can only be reached from `PENDING` (see `TradeRepository::accept` doc):
        // it is the signal that this is the very first acceptance, hence the moment to reserve
        // this trade's cards by abandoning every other active trade sharing one of them.
        if matches!(new_status, Some(TradeStatus::OneAccepted)) {
            sqlx::query!(
                r#"UPDATE trade SET status = 'ABANDONED', updated_at = NOW()
                    WHERE id != $1 AND status IN ('PENDING', 'ONE_ACCEPTED')
                      AND id IN (
                        SELECT DISTINCT tc2.trade_id FROM trade_card tc1
                        JOIN trade_card tc2
                          ON tc1.set_code = tc2.set_code AND tc1.collector_number = tc2.collector_number
                         AND tc1.language_code = tc2.language_code AND tc1.foil = tc2.foil
                         AND tc1.owner_user_id = tc2.owner_user_id
                        WHERE tc1.trade_id = $1 AND tc2.trade_id != $1
                      )"#,
                trade_id.0,
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(new_status)
    }

    async fn abandon(&self, trade_id: TradeId) -> Result<bool, AppError> {
        let result = sqlx::query!(
            r#"UPDATE trade SET status = 'ABANDONED', updated_at = NOW()
                WHERE id = $1 AND status NOT IN ('COMPLETED', 'CLOSED', 'ABANDONED')"#,
            trade_id.0,
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn confirm(
        &self,
        trade_id: TradeId,
        is_initiator: bool,
    ) -> Result<Option<TradeStatus>, AppError> {
        let row = sqlx::query!(
            r#"UPDATE trade
                SET initiator_confirmed_at = CASE WHEN $2 THEN NOW() ELSE initiator_confirmed_at END,
                    respondent_confirmed_at = CASE WHEN NOT $2 THEN NOW() ELSE respondent_confirmed_at END,
                    status = CASE
                        WHEN ($2 AND respondent_confirmed_at IS NOT NULL)
                          OR (NOT $2 AND initiator_confirmed_at IS NOT NULL)
                        THEN 'COMPLETED' ELSE 'FULLY_ACCEPTED' END,
                    updated_at = NOW()
                WHERE id = $1
                  AND status = 'FULLY_ACCEPTED'
                  AND ( ($2 AND initiator_confirmed_at IS NULL) OR (NOT $2 AND respondent_confirmed_at IS NULL) )
                RETURNING status"#,
            trade_id.0,
            is_initiator,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| TradeStatus::from_db_str(&r.status)))
    }

    async fn rate(
        &self,
        trade_id: TradeId,
        is_initiator: bool,
        rating: u8,
    ) -> Result<Option<TradeStatus>, AppError> {
        let row = sqlx::query!(
            r#"UPDATE trade
                SET initiator_rating = CASE WHEN $2 THEN $3 ELSE initiator_rating END,
                    respondent_rating = CASE WHEN NOT $2 THEN $3 ELSE respondent_rating END,
                    status = CASE
                        WHEN ($2 AND respondent_rating IS NOT NULL)
                          OR (NOT $2 AND initiator_rating IS NOT NULL)
                        THEN 'CLOSED' ELSE 'COMPLETED' END,
                    updated_at = NOW()
                WHERE id = $1
                  AND status = 'COMPLETED'
                  AND ( ($2 AND initiator_rating IS NULL) OR (NOT $2 AND respondent_rating IS NULL) )
                RETURNING status"#,
            trade_id.0,
            is_initiator,
            rating as i16,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| TradeStatus::from_db_str(&r.status)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::language_code::LanguageCode;
    use crate::infrastructure::adapter_out::repository::common_repository_tests::{
        insert_card, insert_collection_entry, insert_price, insert_trade, insert_trade_card,
        insert_user, mark_trade_accepted_by_both, mark_trade_party_accepted,
        mark_trade_party_confirmed, mark_trade_party_rated,
    };
    use crate::infrastructure::adapter_out::repository::entities::{
        CardMarketPriceEntity, PriceGuideEntity,
    };
    use sqlx::PgPool;

    fn make_price(id_produit: i32, avg: i32) -> CardMarketPriceEntity {
        CardMarketPriceEntity {
            id_produit,
            date: chrono::Local::now().date_naive(),
            normal: PriceGuideEntity {
                low: Some(avg / 2),
                avg: Some(avg),
                trend: Some(avg),
            },
            foil: PriceGuideEntity::empty(),
        }
    }

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

    // --- accept ---

    #[sqlx::test]
    async fn accept_from_pending_by_initiator_moves_to_one_accepted(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "PENDING").await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        let result = repository.accept(TradeId(trade_id), true).await.unwrap();

        assert_eq!(result, Some(TradeStatus::OneAccepted));
        let trade = repository
            .find_by_id(TradeId(trade_id))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(trade.status, TradeStatus::OneAccepted);
        assert!(trade.initiator_accepted_at.is_some());
        assert_eq!(trade.respondent_accepted_at, None);
    }

    #[sqlx::test]
    async fn accept_from_pending_by_respondent_moves_to_one_accepted(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "PENDING").await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        let result = repository.accept(TradeId(trade_id), false).await.unwrap();

        assert_eq!(result, Some(TradeStatus::OneAccepted));
        let trade = repository
            .find_by_id(TradeId(trade_id))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(trade.initiator_accepted_at, None);
        assert!(trade.respondent_accepted_at.is_some());
    }

    #[sqlx::test]
    async fn accept_second_party_from_one_accepted_moves_to_fully_accepted(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "ONE_ACCEPTED").await;
        mark_trade_party_accepted(&pool, trade_id, true).await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        let result = repository.accept(TradeId(trade_id), false).await.unwrap();

        assert_eq!(result, Some(TradeStatus::FullyAccepted));
        let trade = repository
            .find_by_id(TradeId(trade_id))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(trade.status, TradeStatus::FullyAccepted);
        assert!(trade.initiator_accepted_at.is_some());
        assert!(trade.respondent_accepted_at.is_some());
    }

    #[sqlx::test]
    async fn accept_by_party_who_already_accepted_returns_none(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "ONE_ACCEPTED").await;
        mark_trade_party_accepted(&pool, trade_id, true).await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        let result = repository.accept(TradeId(trade_id), true).await.unwrap();

        assert_eq!(result, None);
        let trade = repository
            .find_by_id(TradeId(trade_id))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(trade.status, TradeStatus::OneAccepted);
    }

    #[sqlx::test]
    async fn accept_returns_none_for_terminal_statuses(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        let repository = TradeRepositoryAdapter::new(pool.clone());

        for status in ["FULLY_ACCEPTED", "COMPLETED", "CLOSED", "ABANDONED"] {
            let trade_id = uuid::Uuid::new_v4();
            insert_trade(&pool, trade_id, "user_a", "user_b", status).await;

            let result = repository.accept(TradeId(trade_id), true).await.unwrap();

            assert_eq!(result, None, "status {status} should not be acceptable");
        }
    }

    #[sqlx::test]
    async fn accept_cascade_abandons_other_active_trade_sharing_card(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_user(&pool, "user_c", "carol").await;
        insert_card(&pool, "FDN", "87", "FR", false, "Goblin Boarders", 1).await;

        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "PENDING").await;
        insert_trade_card(&pool, trade_id, "FDN", "87", "FR", false, "user_b", 1).await;

        let other_trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, other_trade_id, "user_c", "user_b", "PENDING").await;
        insert_trade_card(&pool, other_trade_id, "FDN", "87", "FR", false, "user_b", 1).await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        let result = repository.accept(TradeId(trade_id), true).await.unwrap();

        assert_eq!(result, Some(TradeStatus::OneAccepted));
        let other_trade = repository
            .find_by_id(TradeId(other_trade_id))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(other_trade.status, TradeStatus::Abandoned);
    }

    #[sqlx::test]
    async fn accept_cascade_does_not_abandon_fully_accepted_trade_sharing_card(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_user(&pool, "user_c", "carol").await;
        insert_card(&pool, "FDN", "87", "FR", false, "Goblin Boarders", 1).await;

        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "PENDING").await;
        insert_trade_card(&pool, trade_id, "FDN", "87", "FR", false, "user_b", 1).await;

        let other_trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, other_trade_id, "user_c", "user_b", "FULLY_ACCEPTED").await;
        insert_trade_card(&pool, other_trade_id, "FDN", "87", "FR", false, "user_b", 1).await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        repository.accept(TradeId(trade_id), true).await.unwrap();

        let other_trade = repository
            .find_by_id(TradeId(other_trade_id))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(other_trade.status, TradeStatus::FullyAccepted);
    }

    #[sqlx::test]
    async fn accept_cascade_does_not_abandon_trade_without_shared_card(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_user(&pool, "user_c", "carol").await;
        insert_card(&pool, "FDN", "87", "FR", false, "Goblin Boarders", 1).await;
        insert_card(&pool, "FDN", "12", "FR", false, "Sol Ring", 2).await;

        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "PENDING").await;
        insert_trade_card(&pool, trade_id, "FDN", "87", "FR", false, "user_b", 1).await;

        let other_trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, other_trade_id, "user_c", "user_b", "PENDING").await;
        insert_trade_card(&pool, other_trade_id, "FDN", "12", "FR", false, "user_b", 1).await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        repository.accept(TradeId(trade_id), true).await.unwrap();

        let other_trade = repository
            .find_by_id(TradeId(other_trade_id))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(other_trade.status, TradeStatus::Pending);
    }

    #[sqlx::test]
    async fn accept_second_acceptance_does_not_trigger_cascade(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_user(&pool, "user_c", "carol").await;
        insert_card(&pool, "FDN", "87", "FR", false, "Goblin Boarders", 1).await;

        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "ONE_ACCEPTED").await;
        insert_trade_card(&pool, trade_id, "FDN", "87", "FR", false, "user_b", 1).await;
        mark_trade_party_accepted(&pool, trade_id, true).await;

        let other_trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, other_trade_id, "user_c", "user_b", "PENDING").await;
        insert_trade_card(&pool, other_trade_id, "FDN", "87", "FR", false, "user_b", 1).await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        let result = repository.accept(TradeId(trade_id), false).await.unwrap();

        assert_eq!(result, Some(TradeStatus::FullyAccepted));
        let other_trade = repository
            .find_by_id(TradeId(other_trade_id))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(other_trade.status, TradeStatus::Pending);
    }

    // --- abandon ---

    #[sqlx::test]
    async fn abandon_from_pending_returns_true_and_sets_abandoned(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "PENDING").await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        let result = repository.abandon(TradeId(trade_id)).await.unwrap();

        assert!(result);
        let trade = repository
            .find_by_id(TradeId(trade_id))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(trade.status, TradeStatus::Abandoned);
    }

    #[sqlx::test]
    async fn abandon_from_one_accepted_returns_true(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "ONE_ACCEPTED").await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        let result = repository.abandon(TradeId(trade_id)).await.unwrap();

        assert!(result);
    }

    #[sqlx::test]
    async fn abandon_from_fully_accepted_returns_true(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "FULLY_ACCEPTED").await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        let result = repository.abandon(TradeId(trade_id)).await.unwrap();

        assert!(result);
    }

    #[sqlx::test]
    async fn abandon_returns_false_for_terminal_statuses(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        let repository = TradeRepositoryAdapter::new(pool.clone());

        for status in ["COMPLETED", "CLOSED", "ABANDONED"] {
            let trade_id = uuid::Uuid::new_v4();
            insert_trade(&pool, trade_id, "user_a", "user_b", status).await;

            let result = repository.abandon(TradeId(trade_id)).await.unwrap();

            assert!(!result, "status {status} should not be abandonable");
        }
    }

    // --- confirm ---

    #[sqlx::test]
    async fn confirm_first_party_from_fully_accepted_stays_fully_accepted(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "FULLY_ACCEPTED").await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        let result = repository.confirm(TradeId(trade_id), true).await.unwrap();

        assert_eq!(result, Some(TradeStatus::FullyAccepted));
        let trade = repository
            .find_by_id(TradeId(trade_id))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(trade.status, TradeStatus::FullyAccepted);
        assert!(trade.initiator_confirmed_at.is_some());
        assert_eq!(trade.respondent_confirmed_at, None);
    }

    #[sqlx::test]
    async fn confirm_second_party_moves_to_completed(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "FULLY_ACCEPTED").await;
        mark_trade_party_confirmed(&pool, trade_id, true).await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        let result = repository.confirm(TradeId(trade_id), false).await.unwrap();

        assert_eq!(result, Some(TradeStatus::Completed));
        let trade = repository
            .find_by_id(TradeId(trade_id))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(trade.status, TradeStatus::Completed);
        assert!(trade.initiator_confirmed_at.is_some());
        assert!(trade.respondent_confirmed_at.is_some());
    }

    #[sqlx::test]
    async fn confirm_by_party_who_already_confirmed_returns_none(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "FULLY_ACCEPTED").await;
        mark_trade_party_confirmed(&pool, trade_id, true).await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        let result = repository.confirm(TradeId(trade_id), true).await.unwrap();

        assert_eq!(result, None);
    }

    #[sqlx::test]
    async fn confirm_returns_none_when_not_fully_accepted(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        let repository = TradeRepositoryAdapter::new(pool.clone());

        for status in [
            "PENDING",
            "ONE_ACCEPTED",
            "COMPLETED",
            "CLOSED",
            "ABANDONED",
        ] {
            let trade_id = uuid::Uuid::new_v4();
            insert_trade(&pool, trade_id, "user_a", "user_b", status).await;

            let result = repository.confirm(TradeId(trade_id), true).await.unwrap();

            assert_eq!(result, None, "status {status} should not be confirmable");
        }
    }

    // --- rate ---

    #[sqlx::test]
    async fn rate_first_party_from_completed_stays_completed(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "COMPLETED").await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        let result = repository.rate(TradeId(trade_id), true, 5).await.unwrap();

        assert_eq!(result, Some(TradeStatus::Completed));
        let trade = repository
            .find_by_id(TradeId(trade_id))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(trade.status, TradeStatus::Completed);
        assert_eq!(trade.initiator_rating, Some(5));
        assert_eq!(trade.respondent_rating, None);
    }

    #[sqlx::test]
    async fn rate_second_party_moves_to_closed_and_stores_both_ratings(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "COMPLETED").await;
        mark_trade_party_rated(&pool, trade_id, true, 5).await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        let result = repository.rate(TradeId(trade_id), false, 3).await.unwrap();

        assert_eq!(result, Some(TradeStatus::Closed));
        let trade = repository
            .find_by_id(TradeId(trade_id))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(trade.status, TradeStatus::Closed);
        assert_eq!(trade.initiator_rating, Some(5));
        assert_eq!(trade.respondent_rating, Some(3));
    }

    #[sqlx::test]
    async fn rate_by_party_who_already_rated_returns_none(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "COMPLETED").await;
        mark_trade_party_rated(&pool, trade_id, true, 5).await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        let result = repository.rate(TradeId(trade_id), true, 2).await.unwrap();

        assert_eq!(result, None);
    }

    #[sqlx::test]
    async fn rate_returns_none_when_not_completed(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        let repository = TradeRepositoryAdapter::new(pool.clone());

        for status in [
            "PENDING",
            "ONE_ACCEPTED",
            "FULLY_ACCEPTED",
            "CLOSED",
            "ABANDONED",
        ] {
            let trade_id = uuid::Uuid::new_v4();
            insert_trade(&pool, trade_id, "user_a", "user_b", status).await;

            let result = repository.rate(TradeId(trade_id), true, 4).await.unwrap();

            assert_eq!(result, None, "status {status} should not be ratable");
        }
    }

    // --- find_trade_cards_with_details ---

    #[sqlx::test]
    async fn find_trade_cards_with_details_returns_name_and_price_for_each_card(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_card(&pool, "FDN", "87", "FR", false, "Goblin Boarders", 1).await;
        insert_price(&pool, make_price(1, 200)).await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "PENDING").await;
        insert_trade_card(&pool, trade_id, "FDN", "87", "FR", false, "user_b", 3).await;

        let repository = TradeRepositoryAdapter::new(pool);
        let cards = repository
            .find_trade_cards_with_details(TradeId(trade_id))
            .await
            .unwrap();

        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].name, "Goblin Boarders");
        assert_eq!(cards[0].quantity, 3);
        assert_eq!(cards[0].owner_user_id, UserId::new("user_b"));
        assert_eq!(
            cards[0].price_guide.as_ref().and_then(|p| p.avg.value),
            Some(200)
        );
    }

    #[sqlx::test]
    async fn find_trade_cards_with_details_returns_empty_for_trade_without_cards(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "PENDING").await;

        let repository = TradeRepositoryAdapter::new(pool);
        let cards = repository
            .find_trade_cards_with_details(TradeId(trade_id))
            .await
            .unwrap();

        assert!(cards.is_empty());
    }

    #[sqlx::test]
    async fn find_trade_cards_with_details_price_is_none_without_cardmarket_data(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_card(&pool, "FDN", "87", "FR", false, "Goblin Boarders", 1).await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "PENDING").await;
        insert_trade_card(&pool, trade_id, "FDN", "87", "FR", false, "user_b", 1).await;

        let repository = TradeRepositoryAdapter::new(pool);
        let cards = repository
            .find_trade_cards_with_details(TradeId(trade_id))
            .await
            .unwrap();

        assert_eq!(cards.len(), 1);
        assert!(cards[0].price_guide.is_none());
    }

    #[sqlx::test]
    async fn find_trade_cards_with_details_survives_owner_removing_collection_entry(pool: PgPool) {
        // The card was added to the trade, but the owner's `collection_entry` row is absent
        // (e.g. they removed it from their collection afterwards). `mv_card_prices` would drop
        // the card entirely in that case; this query must not, since it joins `card` directly.
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_card(&pool, "FDN", "87", "FR", false, "Goblin Boarders", 1).await;
        insert_price(&pool, make_price(1, 200)).await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "PENDING").await;
        insert_trade_card(&pool, trade_id, "FDN", "87", "FR", false, "user_b", 1).await;

        let repository = TradeRepositoryAdapter::new(pool);
        let cards = repository
            .find_trade_cards_with_details(TradeId(trade_id))
            .await
            .unwrap();

        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].name, "Goblin Boarders");
        assert_eq!(
            cards[0].price_guide.as_ref().and_then(|p| p.avg.value),
            Some(200)
        );
    }

    // --- list_trades ---

    #[sqlx::test]
    async fn list_trades_returns_trades_where_caller_is_initiator_or_respondent(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_user(&pool, "user_c", "carol").await;
        insert_trade(&pool, uuid::Uuid::new_v4(), "user_a", "user_b", "PENDING").await;
        insert_trade(&pool, uuid::Uuid::new_v4(), "user_b", "user_a", "PENDING").await;

        let repository = TradeRepositoryAdapter::new(pool);
        let result = repository
            .list_trades(
                &UserId::new("user_a"),
                TradeListQuery {
                    statuses: vec![],
                    page: 0,
                    page_size: 20,
                },
            )
            .await
            .unwrap();

        assert_eq!(result.total, 2);
        assert_eq!(result.items.len(), 2);
    }

    #[sqlx::test]
    async fn list_trades_excludes_trades_where_caller_is_not_a_party(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_user(&pool, "user_c", "carol").await;
        insert_trade(&pool, uuid::Uuid::new_v4(), "user_b", "user_c", "PENDING").await;

        let repository = TradeRepositoryAdapter::new(pool);
        let result = repository
            .list_trades(
                &UserId::new("user_a"),
                TradeListQuery {
                    statuses: vec![],
                    page: 0,
                    page_size: 20,
                },
            )
            .await
            .unwrap();

        assert!(result.items.is_empty());
        assert_eq!(result.total, 0);
    }

    #[sqlx::test]
    async fn list_trades_filters_by_status(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_trade(&pool, uuid::Uuid::new_v4(), "user_a", "user_b", "PENDING").await;
        insert_trade(&pool, uuid::Uuid::new_v4(), "user_a", "user_b", "CLOSED").await;

        let repository = TradeRepositoryAdapter::new(pool);
        let result = repository
            .list_trades(
                &UserId::new("user_a"),
                TradeListQuery {
                    statuses: vec![TradeStatus::Closed],
                    page: 0,
                    page_size: 20,
                },
            )
            .await
            .unwrap();

        assert_eq!(result.total, 1);
        assert_eq!(result.items[0].status, TradeStatus::Closed);
    }

    #[sqlx::test]
    async fn list_trades_orders_by_updated_at_descending(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        let older_id = uuid::Uuid::new_v4();
        let newer_id = uuid::Uuid::new_v4();
        let now = chrono::Utc::now();
        insert_trade(&pool, older_id, "user_a", "user_b", "PENDING").await;
        sqlx::query("UPDATE trade SET updated_at = $2 WHERE id = $1")
            .bind(older_id)
            .bind(now - chrono::Duration::days(1))
            .execute(&pool)
            .await
            .unwrap();
        insert_trade(&pool, newer_id, "user_a", "user_b", "PENDING").await;
        sqlx::query("UPDATE trade SET updated_at = $2 WHERE id = $1")
            .bind(newer_id)
            .bind(now)
            .execute(&pool)
            .await
            .unwrap();

        let repository = TradeRepositoryAdapter::new(pool);
        let result = repository
            .list_trades(
                &UserId::new("user_a"),
                TradeListQuery {
                    statuses: vec![],
                    page: 0,
                    page_size: 20,
                },
            )
            .await
            .unwrap();

        assert_eq!(result.items[0].id, TradeId(newer_id));
        assert_eq!(result.items[1].id, TradeId(older_id));
    }

    #[sqlx::test]
    async fn list_trades_paginates_with_page_and_page_size(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        for _ in 0..5 {
            insert_trade(&pool, uuid::Uuid::new_v4(), "user_a", "user_b", "PENDING").await;
        }

        let repository = TradeRepositoryAdapter::new(pool);
        let page0 = repository
            .list_trades(
                &UserId::new("user_a"),
                TradeListQuery {
                    statuses: vec![],
                    page: 0,
                    page_size: 2,
                },
            )
            .await
            .unwrap();
        let page1 = repository
            .list_trades(
                &UserId::new("user_a"),
                TradeListQuery {
                    statuses: vec![],
                    page: 1,
                    page_size: 2,
                },
            )
            .await
            .unwrap();

        assert_eq!(page0.items.len(), 2);
        assert_eq!(page1.items.len(), 2);
        assert_eq!(page0.total, 5);
        assert_eq!(page0.page_size, 2);
    }

    #[sqlx::test]
    async fn list_trades_computes_my_and_partner_card_count_from_quantities(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_card(&pool, "FDN", "87", "FR", false, "Goblin Boarders", 1).await;
        insert_card(&pool, "FDN", "12", "FR", false, "Sol Ring", 2).await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "PENDING").await;
        insert_trade_card(&pool, trade_id, "FDN", "87", "FR", false, "user_a", 2).await;
        insert_trade_card(&pool, trade_id, "FDN", "12", "FR", false, "user_b", 3).await;

        let repository = TradeRepositoryAdapter::new(pool);
        let result = repository
            .list_trades(
                &UserId::new("user_a"),
                TradeListQuery {
                    statuses: vec![],
                    page: 0,
                    page_size: 20,
                },
            )
            .await
            .unwrap();

        assert_eq!(result.items[0].my_card_count, 2);
        assert_eq!(result.items[0].partner_card_count, 3);
    }

    #[sqlx::test]
    async fn list_trades_partner_username_is_the_other_party_regardless_of_initiator_respondent(
        pool: PgPool,
    ) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        let trade_as_initiator = uuid::Uuid::new_v4();
        let trade_as_respondent = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_as_initiator, "user_a", "user_b", "PENDING").await;
        insert_trade(&pool, trade_as_respondent, "user_b", "user_a", "PENDING").await;

        let repository = TradeRepositoryAdapter::new(pool);
        let result = repository
            .list_trades(
                &UserId::new("user_a"),
                TradeListQuery {
                    statuses: vec![],
                    page: 0,
                    page_size: 20,
                },
            )
            .await
            .unwrap();

        assert!(
            result
                .items
                .iter()
                .all(|summary| summary.partner_username == "bob")
        );
    }
}
