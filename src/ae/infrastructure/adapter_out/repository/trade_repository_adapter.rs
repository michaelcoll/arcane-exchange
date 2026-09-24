use crate::application::error::{AppError, InfraError};
use crate::application::repository::{CardAvailability, TradeRepository};
use crate::domain::card::CopyId;
use crate::domain::error::FunctionalError;
use crate::domain::pagination::Paginated;
use crate::domain::trade::{
    Trade, TradeCard, TradeCardDetail, TradeId, TradeListQuery, TradeSummary, TradeTransition,
};
use crate::domain::user::UserId;
use crate::infrastructure::adapter_out::repository::entities::{
    TradeCardDetailEntity, TradeCardEntity, TradeEntity, TradeSummaryEntity,
};
use async_trait::async_trait;
use sqlx::{PgExecutor, Pool, Postgres};

pub struct TradeRepositoryAdapter {
    pool: Pool<Postgres>,
}

impl TradeRepositoryAdapter {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

/// Each attempt of `create_or_find_active` only fails when the pair's active trade finishes in
/// the instant between its two statements; a few attempts make that practically impossible.
const CREATE_OR_FIND_ACTIVE_ATTEMPTS: usize = 3;

/// Copies of `copy_id` that `user_id` owns, summed across all binders.
#[tracing::instrument(name = "trade_repo.find_collection_entry_quantity", skip_all, fields(sentry.op = "db"))]
async fn find_collection_entry_quantity(
    executor: impl PgExecutor<'_>,
    user_id: &UserId,
    copy_id: &CopyId,
) -> Result<Option<i32>, AppError> {
    // Sums across all binders: a card split across multiple `collection_entry` rows
    // (one per binder) must still count as fully owned. `SUM` over a filter matching no
    // rows returns a single row with `NULL`, not zero rows — `fetch_one` (not
    // `fetch_optional`) plus mapping the `Option` keeps "card not owned" as `None`.
    let row = sqlx::query!(
        r#"SELECT SUM(quantity) AS quantity FROM collection_entry
                WHERE user_id = $1 AND set_code = $2 AND collector_number = $3
                  AND language_code = $4 AND foil = $5"#,
        user_id.as_str(),
        copy_id.card_id.set_code.to_string(),
        copy_id.card_id.collector_number,
        copy_id.card_id.language_code.to_string(),
        copy_id.foil,
    )
    .fetch_one(executor)
    .await?;

    Ok(row.quantity.map(|q| q as i32))
}

/// Quantity of `copy_id` that `owner_id` actually offers to trade (`v_tradable_entry`), i.e.
/// after applying their visibility, trade binders and rarity filters. Returns `0` when the
/// owner doesn't offer this card at all (private, closed rarity, or unselected binder).
#[tracing::instrument(name = "trade_repo.find_proposed_quantity", skip_all, fields(sentry.op = "db"))]
async fn find_proposed_quantity(
    executor: impl PgExecutor<'_>,
    owner_id: &UserId,
    copy_id: &CopyId,
) -> Result<u8, AppError> {
    let quantity = sqlx::query_scalar!(
        r#"SELECT proposed_quantity FROM v_tradable_entry
                WHERE user_id = $1 AND set_code = $2 AND collector_number = $3
                  AND language_code = $4 AND foil = $5"#,
        owner_id.as_str(),
        copy_id.card_id.set_code.to_string(),
        copy_id.card_id.collector_number,
        copy_id.card_id.language_code.to_string(),
        copy_id.foil,
    )
    .fetch_optional(executor)
    .await?
    .flatten();

    // The view clamps to `LEAST(..., 255)`, so this always fits — `try_from` makes that
    // invariant explicit instead of silently truncating if the clamp is ever dropped.
    Ok(quantity.map_or(0, |q| u8::try_from(q).unwrap_or(u8::MAX)))
}

/// True when `copy_id` (owned by `owner_id`) already appears in a trade other than `trade_id`
/// with status `ONE_ACCEPTED` or `FULLY_ACCEPTED` — i.e. it is already committed to another
/// trade and cannot be added to this one.
#[tracing::instrument(name = "trade_repo.is_card_reserved_elsewhere", skip_all, fields(sentry.op = "db"))]
async fn is_card_reserved_elsewhere(
    executor: impl PgExecutor<'_>,
    trade_id: TradeId,
    owner_id: &UserId,
    copy_id: &CopyId,
) -> Result<bool, AppError> {
    let reserved = sqlx::query_scalar!(
        r#"SELECT EXISTS (
                 SELECT 1 FROM trade_card tc
                 JOIN trade t ON t.id = tc.trade_id
                 WHERE tc.set_code = $1 AND tc.collector_number = $2
                   AND tc.language_code = $3 AND tc.foil = $4
                   AND tc.owner_user_id = $5
                   AND t.id != $6
                   AND t.status IN ('ONE_ACCEPTED', 'FULLY_ACCEPTED')
               ) AS "reserved!""#,
        copy_id.card_id.set_code.to_string(),
        copy_id.card_id.collector_number,
        copy_id.card_id.language_code.to_string(),
        copy_id.foil,
        owner_id.as_str(),
        trade_id.0,
    )
    .fetch_one(executor)
    .await?;

    Ok(reserved)
}

/// Locks the trade row until the end of the transaction. Fails with `TradeNotFound` if it
/// doesn't exist.
#[tracing::instrument(name = "trade_repo.lock_trade", skip_all, fields(sentry.op = "db"))]
async fn lock_trade(executor: impl PgExecutor<'_>, trade_id: TradeId) -> Result<(), AppError> {
    sqlx::query_scalar!("SELECT id FROM trade WHERE id = $1 FOR UPDATE", trade_id.0)
        .fetch_optional(executor)
        .await?
        .ok_or(FunctionalError::TradeNotFound)?;
    Ok(())
}

/// Writes the status and party columns of `transition.next()` if they all still hold the values
/// the transition was decided from (`transition.from()`). The decision itself belongs to `Trade`:
/// this only persists it. Comparing the party columns and not just the status matters: a first
/// confirmation or rating keeps the status, and an acceptance withdrawn then given again brings
/// it back — either way the transition would overwrite a change it never saw.
#[tracing::instrument(name = "trade_repo.write_transition", skip_all, fields(sentry.op = "db"))]
async fn write_transition(
    executor: impl PgExecutor<'_>,
    transition: &TradeTransition,
) -> Result<bool, AppError> {
    let (from, next) = (transition.from(), transition.next());
    let result = sqlx::query!(
        r#"UPDATE trade
            SET status = $2,
                initiator_accepted_at = $3, respondent_accepted_at = $4,
                initiator_confirmed_at = $5, respondent_confirmed_at = $6,
                initiator_rating = $7, respondent_rating = $8,
                updated_at = NOW()
            WHERE id = $1 AND status = $9
              AND initiator_accepted_at IS NOT DISTINCT FROM $10
              AND respondent_accepted_at IS NOT DISTINCT FROM $11
              AND initiator_confirmed_at IS NOT DISTINCT FROM $12
              AND respondent_confirmed_at IS NOT DISTINCT FROM $13
              AND initiator_rating IS NOT DISTINCT FROM $14
              AND respondent_rating IS NOT DISTINCT FROM $15"#,
        transition.trade_id().0,
        next.status.as_db_str(),
        next.initiator_accepted_at,
        next.respondent_accepted_at,
        next.initiator_confirmed_at,
        next.respondent_confirmed_at,
        next.initiator_rating.map(i16::from),
        next.respondent_rating.map(i16::from),
        from.status.as_db_str(),
        from.initiator_accepted_at,
        from.respondent_accepted_at,
        from.initiator_confirmed_at,
        from.respondent_confirmed_at,
        from.initiator_rating.map(i16::from),
        from.respondent_rating.map(i16::from),
    )
    .execute(executor)
    .await?;

    Ok(result.rows_affected() > 0)
}

#[async_trait]
impl TradeRepository for TradeRepositoryAdapter {
    #[tracing::instrument(name = "trade_repo.create_or_find_active", skip_all, fields(sentry.op = "db"))]
    async fn create_or_find_active(
        &self,
        initiator_id: &UserId,
        respondent_id: &UserId,
    ) -> Result<TradeId, AppError> {
        // `ON CONFLICT DO NOTHING` without a target covers `trade_one_active_per_pair`, an
        // expression index: a pair that already has an active trade (or whose concurrent insert
        // commits first) yields no row instead of an error, and that trade is read back. The
        // retry covers the tiny window where it reaches a terminal status between the two
        // statements, freeing the pair again.
        for _ in 0..CREATE_OR_FIND_ACTIVE_ATTEMPTS {
            let inserted = sqlx::query_scalar!(
                r#"INSERT INTO trade (id, initiator_user_id, respondent_user_id, status)
                    VALUES ($1, $2, $3, 'PENDING')
                    ON CONFLICT DO NOTHING
                    RETURNING id"#,
                TradeId::new().0,
                initiator_id.as_str(),
                respondent_id.as_str(),
            )
            .fetch_optional(&self.pool)
            .await?;
            if let Some(id) = inserted {
                return Ok(TradeId(id));
            }

            let existing = sqlx::query_scalar!(
                r#"SELECT id FROM trade
                    WHERE LEAST(initiator_user_id, respondent_user_id) = LEAST($1, $2)
                      AND GREATEST(initiator_user_id, respondent_user_id) = GREATEST($1, $2)
                      AND status IN ('PENDING', 'ONE_ACCEPTED', 'FULLY_ACCEPTED')"#,
                initiator_id.as_str(),
                respondent_id.as_str(),
            )
            .fetch_optional(&self.pool)
            .await?;
            if let Some(id) = existing {
                return Ok(TradeId(id));
            }
        }

        Err(AppError::Infra(InfraError::RepositoryError(
            "could not create or find the active trade of a pair".to_string(),
        )))
    }

    #[tracing::instrument(name = "trade_repo.find_by_id", skip_all, fields(sentry.op = "db"))]
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

        Ok(row.map(Trade::try_from).transpose()?)
    }

    #[tracing::instrument(name = "trade_repo.find_trade_cards", skip_all, fields(sentry.op = "db"))]
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

    #[tracing::instrument(name = "trade_repo.find_trade_cards_with_details", skip_all, fields(sentry.op = "db"))]
    async fn find_trade_cards_with_details(
        &self,
        trade_id: TradeId,
    ) -> Result<Vec<TradeCardDetail>, AppError> {
        let rows = sqlx::query_as!(
            TradeCardDetailEntity,
            r#"SELECT tc.set_code AS "set_code!", tc.collector_number AS "collector_number!",
                       tc.language_code AS "language_code!", tc.foil AS "foil!",
                       tc.owner_user_id AS "owner_user_id!", tc.quantity AS "quantity!",
                       c.name AS "name!", c.scryfall_id AS "scryfall_id!", c.the_gatherer_id,
                       lcp.low, lcp.trend, lcp.avg
                FROM trade_card tc
                JOIN card c ON c.set_code = tc.set_code AND c.collector_number = tc.collector_number
                    AND c.language_code = tc.language_code
                LEFT JOIN mv_last_cardmarket_prices lcp ON lcp.set_code = c.set_code
                    AND lcp.collector_number = c.collector_number AND lcp.foil = tc.foil
                WHERE tc.trade_id = $1"#,
            trade_id.0
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(TradeCardDetail::from).collect())
    }

    #[tracing::instrument(name = "trade_repo.list_trades", skip_all, fields(sentry.op = "db"))]
    async fn list_trades(
        &self,
        caller_id: &UserId,
        query: TradeListQuery,
    ) -> Result<Paginated<TradeSummary>, AppError> {
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
        let limit = i64::from(query.pagination.limit());
        let offset = i64::from(query.pagination.offset());

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

        Ok(Paginated {
            items: rows
                .into_iter()
                .map(TradeSummary::try_from)
                .collect::<Result<_, _>>()?,
            total: total as u64,
            pagination: query.pagination,
        })
    }

    #[tracing::instrument(name = "trade_repo.merge_card_into_trade", skip_all, fields(sentry.op = "db"))]
    async fn merge_card_into_trade(
        &self,
        transition: &TradeTransition,
        copy_id: &CopyId,
        owner_id: &UserId,
        quantity: u8,
        availability: CardAvailability,
    ) -> Result<(), AppError> {
        let trade_id = transition.trade_id();
        let mut tx = self.pool.begin().await?;

        // Serializes every addition to this trade: a concurrent one waits here, then sees the
        // quantity this one committed. The state the transition was decided from must still
        // hold under the lock — else an acceptance committed meanwhile would let a card slip
        // into a trade without reopening it. Everything is rolled back if a check below fails.
        lock_trade(&mut *tx, trade_id).await?;
        if !write_transition(&mut *tx, transition).await? {
            return Err(FunctionalError::TradeNotModifiable.into());
        }

        // The merge below adds `quantity` to whatever this trade already holds for this
        // (card, owner) pair, so availability must cover the resulting total — not just this
        // call's `quantity` — or repeated calls could add more than is available one bite at a
        // time.
        let already_in_trade = sqlx::query_scalar!(
            r#"SELECT quantity FROM trade_card
                WHERE trade_id = $1 AND set_code = $2 AND collector_number = $3
                  AND language_code = $4 AND foil = $5 AND owner_user_id = $6"#,
            trade_id.0,
            copy_id.card_id.set_code.to_string(),
            copy_id.card_id.collector_number,
            copy_id.card_id.language_code.to_string(),
            copy_id.foil,
            owner_id.as_str(),
        )
        .fetch_optional(&mut *tx)
        .await?
        .unwrap_or(0);
        let total_requested = i64::from(already_in_trade) + i64::from(quantity);

        let available = match availability {
            CardAvailability::Owned => find_collection_entry_quantity(&mut *tx, owner_id, copy_id)
                .await?
                .unwrap_or(0),
            CardAvailability::Offered => {
                i32::from(find_proposed_quantity(&mut *tx, owner_id, copy_id).await?)
            }
        };
        // Checked before the reservation so an unavailable card never leaks through a
        // `CardAlreadyReserved` (409) instead: from the caller's perspective an unavailable card
        // is indistinguishable from a nonexistent one, which also avoids leaking the other
        // party's trade settings.
        if total_requested > i64::from(available) {
            return Err(FunctionalError::CardNotFound.into());
        }

        if is_card_reserved_elsewhere(&mut *tx, trade_id, owner_id, copy_id).await? {
            return Err(FunctionalError::CardAlreadyReserved.into());
        }

        sqlx::query!(
            r#"INSERT INTO trade_card (trade_id, set_code, collector_number, language_code, foil, owner_user_id, quantity)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                ON CONFLICT (trade_id, set_code, collector_number, language_code, foil, owner_user_id)
                    DO UPDATE SET quantity = trade_card.quantity + EXCLUDED.quantity"#,
            trade_id.0,
            copy_id.card_id.set_code.to_string(),
            copy_id.card_id.collector_number,
            copy_id.card_id.language_code.to_string(),
            copy_id.foil,
            owner_id.as_str(),
            quantity as i32,
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }

    #[tracing::instrument(name = "trade_repo.remove_card_from_trade", skip_all, fields(sentry.op = "db"))]
    async fn remove_card_from_trade(
        &self,
        transition: &TradeTransition,
        copy_id: &CopyId,
        owner_id: &UserId,
    ) -> Result<bool, AppError> {
        let trade_id = transition.trade_id();
        let mut tx = self.pool.begin().await?;

        // Same lock order as `merge_card_into_trade` (trade row, then its cards).
        lock_trade(&mut *tx, trade_id).await?;

        let result = sqlx::query!(
            r#"DELETE FROM trade_card
                WHERE trade_id = $1 AND set_code = $2 AND collector_number = $3
                  AND language_code = $4 AND foil = $5 AND owner_user_id = $6"#,
            trade_id.0,
            copy_id.card_id.set_code.to_string(),
            copy_id.card_id.collector_number,
            copy_id.card_id.language_code.to_string(),
            copy_id.foil,
            owner_id.as_str(),
        )
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() == 0 {
            return Ok(false);
        }
        // Dropping `tx` rolls back the removal above.
        if !write_transition(&mut *tx, transition).await? {
            return Err(FunctionalError::TradeNotModifiable.into());
        }

        tx.commit().await?;

        Ok(true)
    }

    #[tracing::instrument(name = "trade_repo.apply_transition", skip_all, fields(sentry.op = "db"))]
    async fn apply_transition(&self, transition: &TradeTransition) -> Result<bool, AppError> {
        let trade_id = transition.trade_id();
        let mut tx = self.pool.begin().await?;

        if !write_transition(&mut *tx, transition).await? {
            return Ok(false);
        }

        // Reserving this trade's cards means abandoning every other active trade sharing one of
        // them, in the same transaction (ADR-0008).
        if transition.reserves_cards() {
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

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::service::trade_service::TRADES_MAX_OFFSET;
    use crate::domain::language_code::LanguageCode;
    use crate::domain::pagination::Pagination;
    use crate::domain::trade::{Party, TradeStatus};
    use crate::infrastructure::adapter_out::repository::common_repository_tests::{
        insert_card, insert_card_with_rarity, insert_collection_entry,
        insert_collection_entry_with_binder, insert_price, insert_rarity_filter, insert_trade,
        insert_trade_card, insert_trading_binder, insert_user, insert_user_with_visibility,
        mark_trade_accepted_by_both, mark_trade_party_accepted, mark_trade_party_confirmed,
        mark_trade_party_rated, refresh_view,
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

    fn make_card_id() -> CopyId {
        CopyId::new("FDN", "87", LanguageCode::FR, false)
    }

    #[sqlx::test]
    async fn find_collection_entry_quantity_returns_quantity_when_found(pool: PgPool) {
        insert_user(&pool, "user_b", "bob").await;
        insert_card(&pool, "FDN", "87", "FR", "Goblin Boarders", 1).await;
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

        let result = find_collection_entry_quantity(&pool, &UserId::new("user_b"), &make_card_id())
            .await
            .unwrap();

        assert_eq!(result, Some(3));
    }

    #[sqlx::test]
    async fn find_collection_entry_quantity_returns_none_when_not_found(pool: PgPool) {
        let result =
            find_collection_entry_quantity(&pool, &UserId::new("user_unknown"), &make_card_id())
                .await
                .unwrap();

        assert_eq!(result, None);
    }

    #[sqlx::test]
    async fn find_collection_entry_quantity_sums_across_binders(pool: PgPool) {
        insert_user(&pool, "user_b", "bob").await;
        insert_card(&pool, "FDN", "87", "FR", "Goblin Boarders", 1).await;
        insert_collection_entry_with_binder(
            &pool,
            "FDN",
            "87",
            "FR",
            false,
            "user_b",
            2,
            100,
            chrono::Utc::now(),
            Some("Binder A"),
        )
        .await;
        insert_collection_entry_with_binder(
            &pool,
            "FDN",
            "87",
            "FR",
            false,
            "user_b",
            3,
            100,
            chrono::Utc::now(),
            Some("Binder B"),
        )
        .await;

        let result = find_collection_entry_quantity(&pool, &UserId::new("user_b"), &make_card_id())
            .await
            .unwrap();

        assert_eq!(result, Some(5));
    }

    #[sqlx::test]
    async fn find_proposed_quantity_is_zero_for_private_user(pool: PgPool) {
        insert_user_with_visibility(&pool, "user_b", "bob", "private").await;
        insert_card(&pool, "FDN", "87", "FR", "Goblin Boarders", 1).await;
        insert_trading_binder(&pool, "user_b", "Trade Binder").await;
        insert_collection_entry_with_binder(
            &pool,
            "FDN",
            "87",
            "FR",
            false,
            "user_b",
            3,
            100,
            chrono::Utc::now(),
            Some("Trade Binder"),
        )
        .await;

        let result = find_proposed_quantity(&pool, &UserId::new("user_b"), &make_card_id())
            .await
            .unwrap();

        assert_eq!(result, 0);
    }

    #[sqlx::test]
    async fn find_proposed_quantity_is_zero_when_unknown_owner(pool: PgPool) {
        let result = find_proposed_quantity(&pool, &UserId::new("user_unknown"), &make_card_id())
            .await
            .unwrap();

        assert_eq!(result, 0);
    }

    #[sqlx::test]
    async fn find_proposed_quantity_is_total_for_public_user(pool: PgPool) {
        insert_user_with_visibility(&pool, "user_b", "bob", "public").await;
        insert_card(&pool, "FDN", "87", "FR", "Goblin Boarders", 1).await;
        insert_collection_entry_with_binder(
            &pool,
            "FDN",
            "87",
            "FR",
            false,
            "user_b",
            2,
            100,
            chrono::Utc::now(),
            None,
        )
        .await;
        insert_collection_entry_with_binder(
            &pool,
            "FDN",
            "87",
            "FR",
            false,
            "user_b",
            1,
            100,
            chrono::Utc::now(),
            Some("Binder A"),
        )
        .await;

        let result = find_proposed_quantity(&pool, &UserId::new("user_b"), &make_card_id())
            .await
            .unwrap();

        assert_eq!(result, 3);
    }

    #[sqlx::test]
    async fn find_proposed_quantity_applies_rarity_kept_copies_for_trade_user(pool: PgPool) {
        insert_user_with_visibility(&pool, "user_b", "bob", "trade").await;
        insert_card_with_rarity(&pool, "FDN", "87", "FR", "Goblin Boarders", 1, "R").await;
        insert_trading_binder(&pool, "user_b", "Trade Binder").await;
        insert_rarity_filter(&pool, "user_b", "R", true, 1).await;
        insert_collection_entry_with_binder(
            &pool,
            "FDN",
            "87",
            "FR",
            false,
            "user_b",
            3,
            100,
            chrono::Utc::now(),
            Some("Trade Binder"),
        )
        .await;

        let result = find_proposed_quantity(&pool, &UserId::new("user_b"), &make_card_id())
            .await
            .unwrap();

        assert_eq!(result, 2);
    }

    #[sqlx::test]
    async fn find_proposed_quantity_deducts_kept_copies_per_binder_row(pool: PgPool) {
        // Same card split across two checked binders, 3 copies each, `kept_copies = 1`.
        // `kept_copies` must be deducted per `collection_entry` row (2 + 2 = 4), matching
        // `collection_rarity_filters_repository_adapter::list_with_counts`'s "Proposés" counter
        // — not once on the aggregated total (which would wrongly yield 3 + 3 - 1 = 5).
        insert_user_with_visibility(&pool, "user_b", "bob", "trade").await;
        insert_card_with_rarity(&pool, "FDN", "87", "FR", "Goblin Boarders", 1, "R").await;
        insert_trading_binder(&pool, "user_b", "Binder A").await;
        insert_trading_binder(&pool, "user_b", "Binder B").await;
        insert_rarity_filter(&pool, "user_b", "R", true, 1).await;
        insert_collection_entry_with_binder(
            &pool,
            "FDN",
            "87",
            "FR",
            false,
            "user_b",
            3,
            100,
            chrono::Utc::now(),
            Some("Binder A"),
        )
        .await;
        insert_collection_entry_with_binder(
            &pool,
            "FDN",
            "87",
            "FR",
            false,
            "user_b",
            3,
            100,
            chrono::Utc::now(),
            Some("Binder B"),
        )
        .await;

        let result = find_proposed_quantity(&pool, &UserId::new("user_b"), &make_card_id())
            .await
            .unwrap();

        assert_eq!(result, 4);
    }

    #[sqlx::test]
    async fn find_proposed_quantity_is_zero_when_rarity_closed(pool: PgPool) {
        insert_user_with_visibility(&pool, "user_b", "bob", "trade").await;
        insert_card_with_rarity(&pool, "FDN", "87", "FR", "Goblin Boarders", 1, "R").await;
        insert_trading_binder(&pool, "user_b", "Trade Binder").await;
        insert_collection_entry_with_binder(
            &pool,
            "FDN",
            "87",
            "FR",
            false,
            "user_b",
            3,
            100,
            chrono::Utc::now(),
            Some("Trade Binder"),
        )
        .await;

        let result = find_proposed_quantity(&pool, &UserId::new("user_b"), &make_card_id())
            .await
            .unwrap();

        assert_eq!(result, 0);
    }

    #[sqlx::test]
    async fn find_proposed_quantity_is_zero_when_binder_not_selected(pool: PgPool) {
        insert_user_with_visibility(&pool, "user_b", "bob", "trade").await;
        insert_card_with_rarity(&pool, "FDN", "87", "FR", "Goblin Boarders", 1, "R").await;
        insert_rarity_filter(&pool, "user_b", "R", true, 0).await;
        insert_collection_entry_with_binder(
            &pool,
            "FDN",
            "87",
            "FR",
            false,
            "user_b",
            3,
            100,
            chrono::Utc::now(),
            Some("Untracked Binder"),
        )
        .await;

        let result = find_proposed_quantity(&pool, &UserId::new("user_b"), &make_card_id())
            .await
            .unwrap();

        assert_eq!(result, 0);
    }

    #[sqlx::test]
    async fn find_proposed_quantity_is_zero_when_kept_copies_covers_quantity(pool: PgPool) {
        insert_user_with_visibility(&pool, "user_b", "bob", "trade").await;
        insert_card_with_rarity(&pool, "FDN", "87", "FR", "Goblin Boarders", 1, "R").await;
        insert_trading_binder(&pool, "user_b", "Trade Binder").await;
        insert_rarity_filter(&pool, "user_b", "R", true, 2).await;
        insert_collection_entry_with_binder(
            &pool,
            "FDN",
            "87",
            "FR",
            false,
            "user_b",
            2,
            100,
            chrono::Utc::now(),
            Some("Trade Binder"),
        )
        .await;

        let result = find_proposed_quantity(&pool, &UserId::new("user_b"), &make_card_id())
            .await
            .unwrap();

        assert_eq!(result, 0);
    }

    #[sqlx::test]
    async fn is_card_reserved_elsewhere_true_when_engaged_in_another_one_accepted_trade(
        pool: PgPool,
    ) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_user(&pool, "user_c", "carol").await;
        insert_card(&pool, "FDN", "87", "FR", "Goblin Boarders", 1).await;
        let other_trade = uuid::Uuid::new_v4();
        insert_trade(&pool, other_trade, "user_a", "user_b", "ONE_ACCEPTED").await;
        insert_trade_card(&pool, other_trade, "FDN", "87", "FR", false, "user_a", 1).await;

        let this_trade = uuid::Uuid::new_v4();
        insert_trade(&pool, this_trade, "user_a", "user_c", "PENDING").await;

        let result = is_card_reserved_elsewhere(
            &pool,
            TradeId(this_trade),
            &UserId::new("user_a"),
            &make_card_id(),
        )
        .await
        .unwrap();

        assert!(result);
    }

    #[sqlx::test]
    async fn is_card_reserved_elsewhere_true_when_engaged_in_another_fully_accepted_trade(
        pool: PgPool,
    ) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_user(&pool, "user_c", "carol").await;
        insert_card(&pool, "FDN", "87", "FR", "Goblin Boarders", 1).await;
        let other_trade = uuid::Uuid::new_v4();
        insert_trade(&pool, other_trade, "user_a", "user_b", "FULLY_ACCEPTED").await;
        insert_trade_card(&pool, other_trade, "FDN", "87", "FR", false, "user_a", 1).await;

        let this_trade = uuid::Uuid::new_v4();
        insert_trade(&pool, this_trade, "user_a", "user_c", "PENDING").await;

        let result = is_card_reserved_elsewhere(
            &pool,
            TradeId(this_trade),
            &UserId::new("user_a"),
            &make_card_id(),
        )
        .await
        .unwrap();

        assert!(result);
    }

    #[sqlx::test]
    async fn is_card_reserved_elsewhere_false_when_only_engaged_in_this_trade(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_c", "carol").await;
        insert_card(&pool, "FDN", "87", "FR", "Goblin Boarders", 1).await;
        let this_trade = uuid::Uuid::new_v4();
        insert_trade(&pool, this_trade, "user_a", "user_c", "ONE_ACCEPTED").await;
        insert_trade_card(&pool, this_trade, "FDN", "87", "FR", false, "user_a", 1).await;

        let result = is_card_reserved_elsewhere(
            &pool,
            TradeId(this_trade),
            &UserId::new("user_a"),
            &make_card_id(),
        )
        .await
        .unwrap();

        assert!(!result);
    }

    #[sqlx::test]
    async fn is_card_reserved_elsewhere_false_when_other_trade_is_only_pending(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_user(&pool, "user_c", "carol").await;
        insert_card(&pool, "FDN", "87", "FR", "Goblin Boarders", 1).await;
        let other_trade = uuid::Uuid::new_v4();
        insert_trade(&pool, other_trade, "user_a", "user_b", "PENDING").await;
        insert_trade_card(&pool, other_trade, "FDN", "87", "FR", false, "user_a", 1).await;

        let this_trade = uuid::Uuid::new_v4();
        insert_trade(&pool, this_trade, "user_a", "user_c", "PENDING").await;

        let result = is_card_reserved_elsewhere(
            &pool,
            TradeId(this_trade),
            &UserId::new("user_a"),
            &make_card_id(),
        )
        .await
        .unwrap();

        assert!(!result);
    }

    #[sqlx::test]
    async fn is_card_reserved_elsewhere_false_when_other_trade_is_terminal(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_user(&pool, "user_c", "carol").await;
        insert_card(&pool, "FDN", "87", "FR", "Goblin Boarders", 1).await;
        let this_trade = uuid::Uuid::new_v4();
        insert_trade(&pool, this_trade, "user_a", "user_c", "PENDING").await;

        for status in ["COMPLETED", "CLOSED", "ABANDONED"] {
            let other_trade = uuid::Uuid::new_v4();
            insert_trade(&pool, other_trade, "user_a", "user_b", status).await;
            insert_trade_card(&pool, other_trade, "FDN", "87", "FR", false, "user_a", 1).await;

            let result = is_card_reserved_elsewhere(
                &pool,
                TradeId(this_trade),
                &UserId::new("user_a"),
                &make_card_id(),
            )
            .await
            .unwrap();

            assert!(!result, "status {status} must not count as reserved");
        }
    }

    #[sqlx::test]
    async fn is_card_reserved_elsewhere_false_when_engaged_by_a_different_owner(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_user(&pool, "user_c", "carol").await;
        insert_card(&pool, "FDN", "87", "FR", "Goblin Boarders", 1).await;
        let other_trade = uuid::Uuid::new_v4();
        insert_trade(&pool, other_trade, "user_b", "user_c", "FULLY_ACCEPTED").await;
        insert_trade_card(&pool, other_trade, "FDN", "87", "FR", false, "user_b", 1).await;

        let this_trade = uuid::Uuid::new_v4();
        insert_trade(&pool, this_trade, "user_a", "user_c", "PENDING").await;

        let result = is_card_reserved_elsewhere(
            &pool,
            TradeId(this_trade),
            &UserId::new("user_a"),
            &make_card_id(),
        )
        .await
        .unwrap();

        assert!(!result);
    }

    /// Opens a second idle connection so that two concurrent calls really overlap: otherwise the
    /// second one spends its time connecting while the first one runs to completion.
    async fn warm_up(pool: &PgPool) {
        let first = pool.acquire().await.unwrap();
        let second = pool.acquire().await.unwrap();
        drop((first, second));
    }

    /// Forces a status behind the repository's back, e.g. to free a pair for the next active
    /// trade or to simulate a concurrent change.
    async fn set_status(pool: &PgPool, trade_id: uuid::Uuid, status: &str) {
        sqlx::query("UPDATE trade SET status = $2 WHERE id = $1")
            .bind(trade_id)
            .bind(status)
            .execute(pool)
            .await
            .unwrap();
    }

    async fn find(repository: &TradeRepositoryAdapter, trade_id: uuid::Uuid) -> Trade {
        repository
            .find_by_id(TradeId(trade_id))
            .await
            .unwrap()
            .unwrap()
    }

    /// The transition a card addition/removal decides on the trade's current state.
    async fn modify(repository: &TradeRepositoryAdapter, trade_id: uuid::Uuid) -> TradeTransition {
        find(repository, trade_id).await.modify().unwrap()
    }

    // --- create_or_find_active ---

    async fn count_active_trades(pool: &PgPool) -> i64 {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM trade WHERE status IN ('PENDING', 'ONE_ACCEPTED', 'FULLY_ACCEPTED')",
        )
        .fetch_one(pool)
        .await
        .unwrap()
    }

    #[sqlx::test]
    async fn create_or_find_active_creates_pending_trade_without_cards(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        let id = repository
            .create_or_find_active(&UserId::new("user_a"), &UserId::new("user_b"))
            .await
            .unwrap();

        let trade = repository.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(trade.initiator_user_id, UserId::new("user_a"));
        assert_eq!(trade.respondent_user_id, UserId::new("user_b"));
        assert_eq!(trade.status, TradeStatus::Pending);
        assert_eq!(trade.initiator_amount_due, None);
        assert_eq!(trade.respondent_amount_due, None);
        assert!(repository.find_trade_cards(id).await.unwrap().is_empty());
    }

    #[sqlx::test]
    async fn create_or_find_active_returns_the_existing_active_trade(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        let repository = TradeRepositoryAdapter::new(pool.clone());

        for status in ["PENDING", "ONE_ACCEPTED", "FULLY_ACCEPTED"] {
            let trade_id = uuid::Uuid::new_v4();
            insert_trade(&pool, trade_id, "user_a", "user_b", status).await;

            let result = repository
                .create_or_find_active(&UserId::new("user_a"), &UserId::new("user_b"))
                .await
                .unwrap();

            assert_eq!(result, TradeId(trade_id), "status {status} is active");
            assert_eq!(count_active_trades(&pool).await, 1);
            set_status(&pool, trade_id, "ABANDONED").await;
        }
    }

    #[sqlx::test]
    async fn create_or_find_active_matches_regardless_of_direction(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_b", "user_a", "PENDING").await;

        let repository = TradeRepositoryAdapter::new(pool);
        let result = repository
            .create_or_find_active(&UserId::new("user_a"), &UserId::new("user_b"))
            .await
            .unwrap();

        assert_eq!(result, TradeId(trade_id));
    }

    #[sqlx::test]
    async fn create_or_find_active_ignores_terminal_statuses(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        let mut terminal_ids = vec![];
        for status in ["COMPLETED", "CLOSED", "ABANDONED"] {
            let trade_id = uuid::Uuid::new_v4();
            insert_trade(&pool, trade_id, "user_a", "user_b", status).await;
            terminal_ids.push(TradeId(trade_id));
        }

        let repository = TradeRepositoryAdapter::new(pool.clone());
        let result = repository
            .create_or_find_active(&UserId::new("user_a"), &UserId::new("user_b"))
            .await
            .unwrap();

        assert!(!terminal_ids.contains(&result));
        assert_eq!(count_active_trades(&pool).await, 1);
    }

    #[sqlx::test]
    async fn create_or_find_active_concurrent_calls_converge_on_a_single_trade(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        warm_up(&pool).await;
        let repository = TradeRepositoryAdapter::new(pool.clone());
        let (user_a, user_b) = (UserId::new("user_a"), UserId::new("user_b"));

        // Both directions at once, as when both players open a trade with each other.
        let (from_a, from_b) = tokio::join!(
            repository.create_or_find_active(&user_a, &user_b),
            repository.create_or_find_active(&user_b, &user_a),
        );

        assert_eq!(from_a.unwrap(), from_b.unwrap());
        assert_eq!(count_active_trades(&pool).await, 1);
    }

    #[sqlx::test]
    async fn a_pair_cannot_hold_two_active_trades(pool: PgPool) {
        // The invariant lives in the data (`trade_one_active_per_pair`), not only in the
        // repository: a write bypassing `create_or_find_active` is rejected too.
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_trade(
            &pool,
            uuid::Uuid::new_v4(),
            "user_a",
            "user_b",
            "FULLY_ACCEPTED",
        )
        .await;

        let second = sqlx::query(
            "INSERT INTO trade (id, initiator_user_id, respondent_user_id, status)
                VALUES ($1, 'user_b', 'user_a', 'PENDING')",
        )
        .bind(uuid::Uuid::new_v4())
        .execute(&pool)
        .await;

        let error = second.unwrap_err();
        assert_eq!(
            error.as_database_error().and_then(|e| e.constraint()),
            Some("trade_one_active_per_pair")
        );
    }

    #[sqlx::test(migrations = false)]
    async fn migration_keeps_the_most_advanced_then_oldest_duplicate_active_trade(pool: PgPool) {
        use crate::infrastructure::adapter_out::repository::common_repository_tests::insert_trade_with_created_at;

        let migrator = sqlx::migrate!("./migrations");
        migrator.run_to(27, &pool).await.unwrap();
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        let (finished, oldest_pending, kept, newest_accepted) = (
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
        );
        let now = chrono::Utc::now();
        let day = chrono::Duration::days(1);
        let trades = [
            (finished, "CLOSED", now - day * 3),
            (oldest_pending, "PENDING", now - day * 2),
            (kept, "FULLY_ACCEPTED", now - day),
            (newest_accepted, "FULLY_ACCEPTED", now),
        ];
        for (id, status, created_at) in trades {
            insert_trade_with_created_at(&pool, id, "user_a", "user_b", status, created_at).await;
        }

        migrator.run(&pool).await.unwrap();

        let repository = TradeRepositoryAdapter::new(pool);
        let status_of = async |id| {
            repository
                .find_by_id(TradeId(id))
                .await
                .unwrap()
                .unwrap()
                .status
        };
        assert_eq!(status_of(kept).await, TradeStatus::FullyAccepted);
        assert_eq!(status_of(oldest_pending).await, TradeStatus::Abandoned);
        assert_eq!(status_of(newest_accepted).await, TradeStatus::Abandoned);
        assert_eq!(status_of(finished).await, TradeStatus::Closed);
    }

    // --- merge_card_into_trade ---

    /// Merges `quantity` copies of `make_card_id()` owned by `user_b`, who is first given more
    /// than enough of them for the availability check to stay out of the way.
    async fn merge_owned_card(
        pool: &PgPool,
        repository: &TradeRepositoryAdapter,
        trade_id: uuid::Uuid,
        quantity: u8,
    ) -> Result<(), AppError> {
        insert_collection_entry(
            pool,
            "FDN",
            "87",
            "FR",
            false,
            "user_b",
            99,
            100,
            chrono::Utc::now(),
        )
        .await;
        repository
            .merge_card_into_trade(
                &modify(repository, trade_id).await,
                &make_card_id(),
                &UserId::new("user_b"),
                quantity,
                CardAvailability::Owned,
            )
            .await
    }

    /// `user_a` and `user_b` with a `PENDING` trade between them, `user_b` offering `offered`
    /// copies of `make_card_id()` (public collection, so all owned copies are offered).
    async fn setup_trade_with_offered_copies(pool: &PgPool, offered: i32) -> uuid::Uuid {
        insert_user(pool, "user_a", "alice").await;
        insert_user_with_visibility(pool, "user_b", "bob", "public").await;
        insert_card(pool, "FDN", "87", "FR", "Goblin Boarders", 1).await;
        insert_collection_entry(
            pool,
            "FDN",
            "87",
            "FR",
            false,
            "user_b",
            offered,
            100,
            chrono::Utc::now(),
        )
        .await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(pool, trade_id, "user_a", "user_b", "PENDING").await;
        trade_id
    }

    async fn merge_offered_card_with(
        repository: &TradeRepositoryAdapter,
        transition: &TradeTransition,
        quantity: u8,
    ) -> Result<(), AppError> {
        repository
            .merge_card_into_trade(
                transition,
                &make_card_id(),
                &UserId::new("user_b"),
                quantity,
                CardAvailability::Offered,
            )
            .await
    }

    async fn merge_offered_card(
        repository: &TradeRepositoryAdapter,
        trade_id: uuid::Uuid,
        quantity: u8,
    ) -> Result<(), AppError> {
        let transition = modify(repository, trade_id).await;
        merge_offered_card_with(repository, &transition, quantity).await
    }

    #[sqlx::test]
    async fn merge_card_into_trade_fails_when_status_changed_since_it_was_read(pool: PgPool) {
        // The caller read `PENDING` (no reopening needed), but an acceptance committed since.
        let trade_id = setup_trade_with_offered_copies(&pool, 1).await;
        let repository = TradeRepositoryAdapter::new(pool.clone());
        let transition = modify(&repository, trade_id).await;
        mark_trade_party_accepted(&pool, trade_id, false).await;
        set_status(&pool, trade_id, "ONE_ACCEPTED").await;

        let result = merge_offered_card_with(&repository, &transition, 1).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::TradeNotModifiable))
        ));
        assert_eq!(quantity_in_trade(&repository, trade_id).await, 0);
        assert!(
            find(&repository, trade_id)
                .await
                .respondent_accepted_at
                .is_some()
        );
    }

    #[sqlx::test]
    async fn merge_card_into_trade_fails_when_trade_does_not_exist(pool: PgPool) {
        let trade_id = setup_trade_with_offered_copies(&pool, 1).await;
        let repository = TradeRepositoryAdapter::new(pool);
        let unknown = Trade {
            id: TradeId::new(),
            ..find(&repository, trade_id).await
        };
        let transition = unknown.modify().unwrap();

        let result = merge_offered_card_with(&repository, &transition, 1).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::TradeNotFound))
        ));
    }

    async fn quantity_in_trade(repository: &TradeRepositoryAdapter, trade_id: uuid::Uuid) -> u32 {
        repository
            .find_trade_cards(TradeId(trade_id))
            .await
            .unwrap()
            .iter()
            .map(|c| c.quantity)
            .sum()
    }

    #[sqlx::test]
    async fn merge_card_into_trade_accepts_up_to_the_offered_quantity(pool: PgPool) {
        let trade_id = setup_trade_with_offered_copies(&pool, 2).await;
        let repository = TradeRepositoryAdapter::new(pool);

        merge_offered_card(&repository, trade_id, 2).await.unwrap();

        assert_eq!(quantity_in_trade(&repository, trade_id).await, 2);
    }

    #[sqlx::test]
    async fn merge_card_into_trade_fails_when_request_exceeds_offered_quantity(pool: PgPool) {
        let trade_id = setup_trade_with_offered_copies(&pool, 2).await;
        let repository = TradeRepositoryAdapter::new(pool);

        let result = merge_offered_card(&repository, trade_id, 3).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::CardNotFound))
        ));
        assert_eq!(quantity_in_trade(&repository, trade_id).await, 0);
    }

    #[sqlx::test]
    async fn merge_card_into_trade_counts_copies_already_in_the_trade(pool: PgPool) {
        // Bob offers 2 copies. 1 is already in the trade, 2 more would sum to 3, which Bob never
        // offered.
        let trade_id = setup_trade_with_offered_copies(&pool, 2).await;
        insert_trade_card(&pool, trade_id, "FDN", "87", "FR", false, "user_b", 1).await;
        let repository = TradeRepositoryAdapter::new(pool);

        let result = merge_offered_card(&repository, trade_id, 2).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::CardNotFound))
        ));
        assert_eq!(quantity_in_trade(&repository, trade_id).await, 1);
    }

    #[sqlx::test]
    async fn merge_card_into_trade_offered_excludes_a_private_collection(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user_with_visibility(&pool, "user_b", "bob", "private").await;
        insert_card(&pool, "FDN", "87", "FR", "Goblin Boarders", 1).await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "PENDING").await;
        let repository = TradeRepositoryAdapter::new(pool.clone());

        let offered = merge_offered_card(&repository, trade_id, 1).await;
        // The same copies are fully available to their owner, whatever their visibility.
        let owned = merge_owned_card(&pool, &repository, trade_id, 1).await;

        assert!(matches!(
            offered,
            Err(AppError::Functional(FunctionalError::CardNotFound))
        ));
        assert!(owned.is_ok());
    }

    #[sqlx::test]
    async fn merge_card_into_trade_fails_when_card_reserved_elsewhere(pool: PgPool) {
        let trade_id = setup_trade_with_offered_copies(&pool, 1).await;
        insert_user(&pool, "user_c", "carol").await;
        let other_trade = uuid::Uuid::new_v4();
        insert_trade(&pool, other_trade, "user_b", "user_c", "ONE_ACCEPTED").await;
        insert_trade_card(&pool, other_trade, "FDN", "87", "FR", false, "user_b", 1).await;
        let repository = TradeRepositoryAdapter::new(pool);

        let result = merge_offered_card(&repository, trade_id, 1).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::CardAlreadyReserved))
        ));
    }

    #[sqlx::test]
    async fn merge_card_into_trade_reports_unavailable_before_reserved(pool: PgPool) {
        // An unavailable card must not leak through a `CardAlreadyReserved` (409).
        let trade_id = setup_trade_with_offered_copies(&pool, 1).await;
        insert_user(&pool, "user_c", "carol").await;
        let other_trade = uuid::Uuid::new_v4();
        insert_trade(&pool, other_trade, "user_b", "user_c", "ONE_ACCEPTED").await;
        insert_trade_card(&pool, other_trade, "FDN", "87", "FR", false, "user_b", 1).await;
        let repository = TradeRepositoryAdapter::new(pool);

        let result = merge_offered_card(&repository, trade_id, 2).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::CardNotFound))
        ));
    }

    #[sqlx::test]
    async fn merge_card_into_trade_concurrent_additions_cannot_exceed_offered_quantity(
        pool: PgPool,
    ) {
        let trade_id = setup_trade_with_offered_copies(&pool, 1).await;
        warm_up(&pool).await;
        let repository = TradeRepositoryAdapter::new(pool);

        let (first, second) = tokio::join!(
            merge_offered_card(&repository, trade_id, 1),
            merge_offered_card(&repository, trade_id, 1),
        );

        assert!(
            first.is_ok() ^ second.is_ok(),
            "exactly one addition must win: {first:?} / {second:?}"
        );
        assert_eq!(quantity_in_trade(&repository, trade_id).await, 1);
    }

    #[sqlx::test]
    async fn merge_card_into_trade_adds_new_card_and_keeps_status(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_card(&pool, "FDN", "87", "FR", "Goblin Boarders", 1).await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "PENDING").await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        merge_owned_card(&pool, &repository, trade_id, 2)
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
        insert_card(&pool, "FDN", "87", "FR", "Goblin Boarders", 1).await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "ONE_ACCEPTED").await;
        mark_trade_accepted_by_both(&pool, trade_id).await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        merge_owned_card(&pool, &repository, trade_id, 1)
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
        insert_card(&pool, "FDN", "87", "FR", "Goblin Boarders", 1).await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "PENDING").await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        merge_owned_card(&pool, &repository, trade_id, 1)
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
        insert_card(&pool, "FDN", "87", "FR", "Goblin Boarders", 1).await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "PENDING").await;
        insert_trade_card(&pool, trade_id, "FDN", "87", "FR", false, "user_b", 2).await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        merge_owned_card(&pool, &repository, trade_id, 3)
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
        insert_card(&pool, "FDN", "87", "FR", "Goblin Boarders", 1).await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "PENDING").await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        let before = repository
            .find_by_id(TradeId(trade_id))
            .await
            .unwrap()
            .unwrap()
            .updated_at;

        merge_owned_card(&pool, &repository, trade_id, 1)
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

    // --- remove_card_from_trade ---

    #[sqlx::test]
    async fn remove_card_from_trade_removes_matching_card_entirely(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_card(&pool, "FDN", "87", "FR", "Goblin Boarders", 1).await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "PENDING").await;
        insert_trade_card(&pool, trade_id, "FDN", "87", "FR", false, "user_b", 5).await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        let removed = repository
            .remove_card_from_trade(
                &modify(&repository, trade_id).await,
                &make_card_id(),
                &UserId::new("user_b"),
            )
            .await
            .unwrap();

        assert!(removed);
        let trade_cards = repository
            .find_trade_cards(TradeId(trade_id))
            .await
            .unwrap();
        assert!(trade_cards.is_empty());
    }

    #[sqlx::test]
    async fn remove_card_from_trade_returns_false_when_card_absent(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_card(&pool, "FDN", "87", "FR", "Goblin Boarders", 1).await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "PENDING").await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        let before = repository
            .find_by_id(TradeId(trade_id))
            .await
            .unwrap()
            .unwrap();

        let removed = repository
            .remove_card_from_trade(
                &modify(&repository, trade_id).await,
                &make_card_id(),
                &UserId::new("user_b"),
            )
            .await
            .unwrap();

        assert!(!removed);
        let after = repository
            .find_by_id(TradeId(trade_id))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(after.updated_at, before.updated_at);
        assert_eq!(after.status, TradeStatus::Pending);
    }

    #[sqlx::test]
    async fn remove_card_from_trade_returns_false_when_owner_does_not_match(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_card(&pool, "FDN", "87", "FR", "Goblin Boarders", 1).await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "PENDING").await;
        insert_trade_card(&pool, trade_id, "FDN", "87", "FR", false, "user_b", 1).await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        let removed = repository
            .remove_card_from_trade(
                &modify(&repository, trade_id).await,
                &make_card_id(),
                &UserId::new("user_a"),
            )
            .await
            .unwrap();

        assert!(!removed);
    }

    #[sqlx::test]
    async fn remove_card_from_trade_reopens_one_accepted_trade(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_card(&pool, "FDN", "87", "FR", "Goblin Boarders", 1).await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "ONE_ACCEPTED").await;
        insert_trade_card(&pool, trade_id, "FDN", "87", "FR", false, "user_b", 1).await;
        mark_trade_party_accepted(&pool, trade_id, true).await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        let removed = repository
            .remove_card_from_trade(
                &modify(&repository, trade_id).await,
                &make_card_id(),
                &UserId::new("user_b"),
            )
            .await
            .unwrap();

        assert!(removed);
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
    async fn remove_card_from_trade_fails_when_status_changed_since_it_was_read(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_card(&pool, "FDN", "87", "FR", "Goblin Boarders", 1).await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "PENDING").await;
        insert_trade_card(&pool, trade_id, "FDN", "87", "FR", false, "user_b", 1).await;
        let repository = TradeRepositoryAdapter::new(pool.clone());
        let transition = modify(&repository, trade_id).await;
        mark_trade_party_accepted(&pool, trade_id, true).await;
        set_status(&pool, trade_id, "ONE_ACCEPTED").await;

        let result = repository
            .remove_card_from_trade(&transition, &make_card_id(), &UserId::new("user_b"))
            .await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::TradeNotModifiable))
        ));
        let trade = find(&repository, trade_id).await;
        assert_eq!(trade.status, TradeStatus::OneAccepted);
        assert!(trade.initiator_accepted_at.is_some());
        assert_eq!(
            repository
                .find_trade_cards(TradeId(trade_id))
                .await
                .unwrap()
                .len(),
            1
        );
    }

    #[sqlx::test]
    async fn remove_card_from_trade_removes_last_card_leaving_trade_empty(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_card(&pool, "FDN", "87", "FR", "Goblin Boarders", 1).await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "PENDING").await;
        insert_trade_card(&pool, trade_id, "FDN", "87", "FR", false, "user_b", 1).await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        let removed = repository
            .remove_card_from_trade(
                &modify(&repository, trade_id).await,
                &make_card_id(),
                &UserId::new("user_b"),
            )
            .await
            .unwrap();

        assert!(removed);
        let trade = repository
            .find_by_id(TradeId(trade_id))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(trade.status, TradeStatus::Pending);
        let trade_cards = repository
            .find_trade_cards(TradeId(trade_id))
            .await
            .unwrap();
        assert!(trade_cards.is_empty());
    }

    // --- apply_transition ---

    /// Takes a transition on the trade's current state, as the services do, then applies it.
    async fn apply(
        repository: &TradeRepositoryAdapter,
        trade_id: uuid::Uuid,
        decide: impl FnOnce(&Trade) -> Result<TradeTransition, FunctionalError>,
    ) -> bool {
        let transition = decide(&find(repository, trade_id).await).unwrap();
        repository.apply_transition(&transition).await.unwrap()
    }

    async fn accept(
        repository: &TradeRepositoryAdapter,
        trade_id: uuid::Uuid,
        party: Party,
    ) -> bool {
        // The domain refuses an empty trade; which cards it holds is irrelevant here.
        let cards = [TradeCard {
            card_id: make_card_id(),
            owner_user_id: UserId::new("user_b"),
            quantity: 1,
        }];
        apply(repository, trade_id, |trade| trade.accept(party, &cards)).await
    }

    async fn confirm(
        repository: &TradeRepositoryAdapter,
        trade_id: uuid::Uuid,
        party: Party,
    ) -> bool {
        apply(repository, trade_id, |trade| trade.confirm(party)).await
    }

    async fn rate(
        repository: &TradeRepositoryAdapter,
        trade_id: uuid::Uuid,
        party: Party,
        rating: u8,
    ) -> bool {
        apply(repository, trade_id, |trade| trade.rate(party, rating)).await
    }

    async fn abandon(repository: &TradeRepositoryAdapter, trade_id: uuid::Uuid) -> bool {
        apply(repository, trade_id, Trade::abandon).await
    }

    #[sqlx::test]
    async fn apply_transition_writes_nothing_when_status_changed_since_it_was_read(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_user(&pool, "user_c", "carol").await;
        insert_card(&pool, "FDN", "87", "FR", "Goblin Boarders", 1).await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "PENDING").await;
        insert_trade_card(&pool, trade_id, "FDN", "87", "FR", false, "user_b", 1).await;
        let other_trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, other_trade_id, "user_c", "user_b", "PENDING").await;
        insert_trade_card(&pool, other_trade_id, "FDN", "87", "FR", false, "user_b", 1).await;
        let repository = TradeRepositoryAdapter::new(pool.clone());
        // The initiator's first acceptance, decided on `PENDING`...
        let cards = repository
            .find_trade_cards(TradeId(trade_id))
            .await
            .unwrap();
        let stale = find(&repository, trade_id)
            .await
            .accept(Party::Initiator, &cards)
            .unwrap();
        // ...while the respondent's acceptance commits first.
        assert!(accept(&repository, trade_id, Party::Respondent).await);
        set_status(&pool, other_trade_id, "PENDING").await;

        let applied = repository.apply_transition(&stale).await.unwrap();

        assert!(!applied);
        let trade = find(&repository, trade_id).await;
        assert_eq!(trade.status, TradeStatus::OneAccepted);
        assert_eq!(trade.initiator_accepted_at, None);
        assert!(trade.respondent_accepted_at.is_some());
        // Nor is the cascade of a first acceptance replayed.
        assert_eq!(
            find(&repository, other_trade_id).await.status,
            TradeStatus::Pending
        );
    }

    #[sqlx::test]
    async fn apply_transition_returns_false_for_an_unknown_trade(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "PENDING").await;
        let repository = TradeRepositoryAdapter::new(pool);
        let unknown = Trade {
            id: TradeId::new(),
            ..find(&repository, trade_id).await
        };
        let transition = unknown.abandon().unwrap();

        assert!(!repository.apply_transition(&transition).await.unwrap());
        assert_eq!(
            find(&repository, trade_id).await.status,
            TradeStatus::Pending
        );
    }

    #[sqlx::test]
    async fn apply_transition_does_not_erase_a_change_that_kept_the_status(pool: PgPool) {
        // Both parties confirm at once: a first confirmation keeps `FULLY_ACCEPTED`.
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "FULLY_ACCEPTED").await;
        let repository = TradeRepositoryAdapter::new(pool);
        let stale = find(&repository, trade_id)
            .await
            .confirm(Party::Initiator)
            .unwrap();
        assert!(confirm(&repository, trade_id, Party::Respondent).await);

        assert!(!repository.apply_transition(&stale).await.unwrap());
        let trade = find(&repository, trade_id).await;
        assert_eq!(trade.status, TradeStatus::FullyAccepted);
        assert!(trade.respondent_confirmed_at.is_some());
        // Decided again on the fresh state, the same confirmation completes the trade.
        assert!(confirm(&repository, trade_id, Party::Initiator).await);
        assert_eq!(
            find(&repository, trade_id).await.status,
            TradeStatus::Completed
        );
    }

    #[sqlx::test]
    async fn apply_transition_does_not_revive_a_withdrawn_acceptance(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "ONE_ACCEPTED").await;
        mark_trade_party_accepted(&pool, trade_id, true).await;
        let repository = TradeRepositoryAdapter::new(pool);
        // The respondent decides to accept on top of the initiator's acceptance...
        let cards = [TradeCard {
            card_id: make_card_id(),
            owner_user_id: UserId::new("user_b"),
            quantity: 1,
        }];
        let stale = find(&repository, trade_id)
            .await
            .accept(Party::Respondent, &cards)
            .unwrap();
        // ...while the trade is modified (acceptances withdrawn) and accepted by them again,
        // which brings the status back to `ONE_ACCEPTED`.
        let modification = modify(&repository, trade_id).await;
        assert!(repository.apply_transition(&modification).await.unwrap());
        assert!(accept(&repository, trade_id, Party::Respondent).await);

        assert!(!repository.apply_transition(&stale).await.unwrap());
        let trade = find(&repository, trade_id).await;
        assert_eq!(trade.status, TradeStatus::OneAccepted);
        assert_eq!(trade.initiator_accepted_at, None);
    }

    // --- accept ---

    #[sqlx::test]
    async fn accept_from_pending_by_initiator_moves_to_one_accepted(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "PENDING").await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        assert!(accept(&repository, trade_id, Party::Initiator).await);
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
        assert!(accept(&repository, trade_id, Party::Respondent).await);
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
        assert!(accept(&repository, trade_id, Party::Respondent).await);
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
    async fn accept_cascade_abandons_other_active_trade_sharing_card(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_user(&pool, "user_c", "carol").await;
        insert_card(&pool, "FDN", "87", "FR", "Goblin Boarders", 1).await;

        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "PENDING").await;
        insert_trade_card(&pool, trade_id, "FDN", "87", "FR", false, "user_b", 1).await;

        let other_trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, other_trade_id, "user_c", "user_b", "PENDING").await;
        insert_trade_card(&pool, other_trade_id, "FDN", "87", "FR", false, "user_b", 1).await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        assert!(accept(&repository, trade_id, Party::Initiator).await);
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
        insert_card(&pool, "FDN", "87", "FR", "Goblin Boarders", 1).await;

        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "PENDING").await;
        insert_trade_card(&pool, trade_id, "FDN", "87", "FR", false, "user_b", 1).await;

        let other_trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, other_trade_id, "user_c", "user_b", "FULLY_ACCEPTED").await;
        insert_trade_card(&pool, other_trade_id, "FDN", "87", "FR", false, "user_b", 1).await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        assert!(accept(&repository, trade_id, Party::Initiator).await);

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
        insert_card(&pool, "FDN", "87", "FR", "Goblin Boarders", 1).await;
        insert_card(&pool, "FDN", "12", "FR", "Sol Ring", 2).await;

        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "PENDING").await;
        insert_trade_card(&pool, trade_id, "FDN", "87", "FR", false, "user_b", 1).await;

        let other_trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, other_trade_id, "user_c", "user_b", "PENDING").await;
        insert_trade_card(&pool, other_trade_id, "FDN", "12", "FR", false, "user_b", 1).await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        assert!(accept(&repository, trade_id, Party::Initiator).await);

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
        insert_card(&pool, "FDN", "87", "FR", "Goblin Boarders", 1).await;

        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "ONE_ACCEPTED").await;
        insert_trade_card(&pool, trade_id, "FDN", "87", "FR", false, "user_b", 1).await;
        mark_trade_party_accepted(&pool, trade_id, true).await;

        let other_trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, other_trade_id, "user_c", "user_b", "PENDING").await;
        insert_trade_card(&pool, other_trade_id, "FDN", "87", "FR", false, "user_b", 1).await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        assert!(accept(&repository, trade_id, Party::Respondent).await);
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
        assert!(abandon(&repository, trade_id).await);
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
        assert!(abandon(&repository, trade_id).await);
    }

    #[sqlx::test]
    async fn abandon_from_fully_accepted_returns_true(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "FULLY_ACCEPTED").await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        assert!(abandon(&repository, trade_id).await);
    }

    // --- confirm ---

    #[sqlx::test]
    async fn confirm_first_party_from_fully_accepted_stays_fully_accepted(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "FULLY_ACCEPTED").await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        assert!(confirm(&repository, trade_id, Party::Initiator).await);
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
        assert!(confirm(&repository, trade_id, Party::Respondent).await);
        let trade = repository
            .find_by_id(TradeId(trade_id))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(trade.status, TradeStatus::Completed);
        assert!(trade.initiator_confirmed_at.is_some());
        assert!(trade.respondent_confirmed_at.is_some());
    }

    // --- rate ---

    #[sqlx::test]
    async fn rate_first_party_from_completed_stays_completed(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "COMPLETED").await;

        let repository = TradeRepositoryAdapter::new(pool.clone());
        assert!(rate(&repository, trade_id, Party::Initiator, 5).await);
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
        assert!(rate(&repository, trade_id, Party::Respondent, 3).await);
        let trade = repository
            .find_by_id(TradeId(trade_id))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(trade.status, TradeStatus::Closed);
        assert_eq!(trade.initiator_rating, Some(5));
        assert_eq!(trade.respondent_rating, Some(3));
    }

    // --- find_trade_cards_with_details ---

    #[sqlx::test]
    async fn find_trade_cards_with_details_returns_name_and_price_for_each_card(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_card(&pool, "FDN", "87", "FR", "Goblin Boarders", 1).await;
        insert_price(&pool, make_price(1, 200)).await;
        refresh_view(&pool).await;
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
    async fn find_trade_cards_with_details_uses_the_finish_of_each_trade_card(pool: PgPool) {
        // The catalog carries a single card definition for both finishes; the price shown for
        // each trade_card must come from its own `foil` column, not from the (now finish-less)
        // `card` row they both join to.
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_card(&pool, "FDN", "87", "FR", "Goblin Boarders", 1).await;
        insert_price(
            &pool,
            CardMarketPriceEntity {
                id_produit: 1,
                date: chrono::Local::now().date_naive(),
                normal: PriceGuideEntity {
                    low: Some(100),
                    avg: Some(200),
                    trend: Some(200),
                },
                foil: PriceGuideEntity {
                    low: Some(400),
                    avg: Some(500),
                    trend: Some(500),
                },
            },
        )
        .await;
        refresh_view(&pool).await;

        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user_a", "user_b", "PENDING").await;
        insert_trade_card(&pool, trade_id, "FDN", "87", "FR", false, "user_b", 1).await;
        insert_trade_card(&pool, trade_id, "FDN", "87", "FR", true, "user_b", 1).await;

        let repository = TradeRepositoryAdapter::new(pool);
        let cards = repository
            .find_trade_cards_with_details(TradeId(trade_id))
            .await
            .unwrap();

        assert_eq!(cards.len(), 2);
        let non_foil = cards
            .iter()
            .find(|c| !c.card_id.foil)
            .expect("a non-foil trade_card must be present");
        let foil = cards
            .iter()
            .find(|c| c.card_id.foil)
            .expect("a foil trade_card must be present");

        assert_eq!(
            non_foil.price_guide.as_ref().and_then(|p| p.avg.value),
            Some(200)
        );
        assert_eq!(
            foil.price_guide.as_ref().and_then(|p| p.avg.value),
            Some(500),
            "the foil trade_card must expose the foil price, not the non-foil one"
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
        insert_card(&pool, "FDN", "87", "FR", "Goblin Boarders", 1).await;
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
        // the card entirely in that case; this query must not, since it joins `card` and
        // `mv_last_cardmarket_prices` directly, neither of which is gated by ownership.
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_card(&pool, "FDN", "87", "FR", "Goblin Boarders", 1).await;
        insert_price(&pool, make_price(1, 200)).await;
        refresh_view(&pool).await;
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
        insert_trade(&pool, uuid::Uuid::new_v4(), "user_b", "user_a", "CLOSED").await;

        let repository = TradeRepositoryAdapter::new(pool);
        let result = repository
            .list_trades(
                &UserId::new("user_a"),
                TradeListQuery {
                    statuses: vec![],
                    pagination: Pagination::try_new(0, 20, TRADES_MAX_OFFSET).unwrap(),
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
                    pagination: Pagination::try_new(0, 20, TRADES_MAX_OFFSET).unwrap(),
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
                    pagination: Pagination::try_new(0, 20, TRADES_MAX_OFFSET).unwrap(),
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
        insert_trade(&pool, older_id, "user_a", "user_b", "CLOSED").await;
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
                    pagination: Pagination::try_new(0, 20, TRADES_MAX_OFFSET).unwrap(),
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
            insert_trade(&pool, uuid::Uuid::new_v4(), "user_a", "user_b", "CLOSED").await;
        }

        let repository = TradeRepositoryAdapter::new(pool);
        let page0 = repository
            .list_trades(
                &UserId::new("user_a"),
                TradeListQuery {
                    statuses: vec![],
                    pagination: Pagination::try_new(0, 2, TRADES_MAX_OFFSET).unwrap(),
                },
            )
            .await
            .unwrap();
        let page1 = repository
            .list_trades(
                &UserId::new("user_a"),
                TradeListQuery {
                    statuses: vec![],
                    pagination: Pagination::try_new(1, 2, TRADES_MAX_OFFSET).unwrap(),
                },
            )
            .await
            .unwrap();

        assert_eq!(page0.items.len(), 2);
        assert_eq!(page1.items.len(), 2);
        assert_eq!(page0.total, 5);
        assert_eq!(page0.pagination.page_size(), 2);
    }

    #[sqlx::test]
    async fn list_trades_computes_my_and_partner_card_count_from_quantities(pool: PgPool) {
        insert_user(&pool, "user_a", "alice").await;
        insert_user(&pool, "user_b", "bob").await;
        insert_card(&pool, "FDN", "87", "FR", "Goblin Boarders", 1).await;
        insert_card(&pool, "FDN", "12", "FR", "Sol Ring", 2).await;
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
                    pagination: Pagination::try_new(0, 20, TRADES_MAX_OFFSET).unwrap(),
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
        insert_trade(&pool, trade_as_initiator, "user_a", "user_b", "CLOSED").await;
        insert_trade(&pool, trade_as_respondent, "user_b", "user_a", "PENDING").await;

        let repository = TradeRepositoryAdapter::new(pool);
        let result = repository
            .list_trades(
                &UserId::new("user_a"),
                TradeListQuery {
                    statuses: vec![],
                    pagination: Pagination::try_new(0, 20, TRADES_MAX_OFFSET).unwrap(),
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
