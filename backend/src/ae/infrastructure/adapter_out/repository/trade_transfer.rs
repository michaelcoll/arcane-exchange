//! The Transfer: when a Trade completes, each of its cards leaves its owner's Collection and
//! enters the other party's. It runs inside the transaction of the transition that completes the
//! Trade (`TradeRepositoryAdapter::apply_transition`), so both succeed or fail together.

use crate::application::error::{AppError, InfraError};
use crate::domain::card::CardId;
use crate::domain::collection_entry::{EntryCopies, MAX_ENTRY_QUANTITY};
use crate::domain::language_code::LanguageCode;
use crate::domain::trade::TradeTransition;
use crate::domain::user::UserId;
use crate::infrastructure::adapter_out::repository::collection_entry_repository_adapter::{
    find_entry, insert_entry, lock_card_entries, write_copies,
};
use chrono::Utc;
use sqlx::PgConnection;
use uuid::Uuid;

/// The binder the cards a player receives go into, created closed to trade on the first
/// reception. Its name is compared as the binders' uniqueness rule does.
const DEFAULT_RECEIVING_BINDER: &str = "trades";

/// A card of the Trade, as the Transfer moves it.
struct TransferredCard {
    card_id: CardId,
    foil: bool,
    giver: UserId,
    quantity: u32,
}

/// Moves every card of the Trade of `transition` from its owner to the other party. Fails, for
/// the caller to roll back, on any error; a giver who holds less than the committed quantity
/// loses what is left and is not an error.
pub(super) async fn transfer_trade_cards(
    conn: &mut PgConnection,
    transition: &TradeTransition,
) -> Result<(), AppError> {
    let trade = transition.next();
    // A fixed order of the cards, so two Transfers touching the same players' entries queue the
    // same way.
    let rows = sqlx::query!(
        r#"SELECT set_code, collector_number, language_code, foil, owner_user_id, quantity
            FROM trade_card
            WHERE trade_id = $1
            ORDER BY set_code, collector_number, language_code, owner_user_id, foil"#,
        trade.id.0,
    )
    .fetch_all(&mut *conn)
    .await?;

    for row in rows {
        let card = TransferredCard {
            card_id: CardId::try_new(
                row.set_code.as_str(),
                row.collector_number,
                LanguageCode::try_new(&row.language_code)?,
            )?,
            foil: row.foil,
            giver: UserId::new(&row.owner_user_id),
            quantity: u32::try_from(row.quantity).map_err(|_| {
                InfraError::RepositoryError("trade_card holds an out-of-range quantity".into())
            })?,
        };
        let receiver = trade.user_id(trade.party_of(&card.giver)?.other());

        // Entries are locked card by card, per player, in the order of a Collection entry edit.
        // Two Transfers of concurrent Trades between the same players can still lock in
        // opposite orders: Postgres detects that rare deadlock and rolls one back, and its
        // transition and Transfer fail together.
        withdraw(conn, &card).await?;
        receive(conn, &card, receiver).await?;
    }

    Ok(())
}

/// Takes the committed quantity out of the giver's entries of the card and finish: the binders
/// open to trade first, then the other binders, each by name, then the entry without binder. An
/// emptied entry is deleted. Stops when the giver has nothing left.
async fn withdraw(conn: &mut PgConnection, card: &TransferredCard) -> Result<(), AppError> {
    // Locked in the order of a Collection entry edit; the ordered read below runs under them.
    lock_card_entries(conn, &card.giver, &card.card_id).await?;

    let entries = sqlx::query!(
        r#"SELECT ce.id, ce.quantity
            FROM collection_entry ce
                     LEFT JOIN binder b ON b.id = ce.binder_id
            WHERE ce.user_id = $1 AND ce.set_code = $2 AND ce.collector_number = $3
              AND ce.language_code = $4 AND ce.foil = $5
            ORDER BY CASE
                         WHEN b.id IS NULL THEN 2
                         WHEN EXISTS (SELECT 1 FROM trading_binders tb WHERE tb.binder_id = b.id) THEN 0
                         ELSE 1
                     END,
                     b.name,
                     ce.id"#,
        card.giver.as_str(),
        card.card_id.set_code.to_string(),
        card.card_id.collector_number,
        card.card_id.language_code.to_string(),
        card.foil,
    )
    .fetch_all(&mut *conn)
    .await?;

    let mut left = i64::from(card.quantity);
    for entry in entries {
        if left == 0 {
            break;
        }
        let taken = left.min(i64::from(entry.quantity));
        left -= taken;
        if taken == i64::from(entry.quantity) {
            sqlx::query!("DELETE FROM collection_entry WHERE id = $1", entry.id)
                .execute(&mut *conn)
                .await?;
        } else {
            sqlx::query!(
                "UPDATE collection_entry SET quantity = quantity - $2 WHERE id = $1",
                entry.id,
                i32::try_from(taken).unwrap_or(i32::MAX),
            )
            .execute(&mut *conn)
            .await?;
        }
    }
    Ok(())
}

/// Adds the full committed quantity to the receiver's binder, merging with the entry of the
/// same card and finish already there (capped at the entry bound). It has no purchase price and
/// is added today.
async fn receive(
    conn: &mut PgConnection,
    card: &TransferredCard,
    receiver: &UserId,
) -> Result<(), AppError> {
    let binder_id = receiving_binder(conn, receiver).await?;
    let locked = lock_card_entries(conn, receiver, &card.card_id).await?;
    let received = EntryCopies {
        quantity: card.quantity.min(MAX_ENTRY_QUANTITY),
        purchase_price: None,
        added_at: Utc::now(),
    };

    match find_entry(&locked, card.foil, Some(binder_id)) {
        Some(existing) => {
            let merged = existing.copies().merge_saturating(&received);
            write_copies(conn, existing.id, &merged).await?;
        }
        None => {
            insert_entry(
                conn,
                receiver,
                &card.card_id,
                card.foil,
                Some(binder_id),
                &received,
            )
            .await?;
        }
    }
    Ok(())
}

/// The binder where `user_id`'s received cards arrive.
async fn receiving_binder(conn: &mut PgConnection, user_id: &UserId) -> Result<Uuid, AppError> {
    // The no-op update makes `RETURNING` yield the id of an existing binder too, and the unique
    // name keeps two concurrent first receptions from creating two.
    Ok(sqlx::query_scalar!(
        r#"INSERT INTO binder (user_id, name) VALUES ($1, $2)
            ON CONFLICT (user_id, name) DO UPDATE SET name = EXCLUDED.name
            RETURNING id"#,
        user_id.as_str(),
        DEFAULT_RECEIVING_BINDER,
    )
    .fetch_one(conn)
    .await?)
}

#[cfg(test)]
mod tests {
    use crate::application::repository::TradeRepository;
    use crate::domain::collection_entry::MAX_ENTRY_QUANTITY;
    use crate::domain::trade::{Party, Trade, TradeId, TradeStatus};
    use crate::infrastructure::adapter_out::repository::common_repository_tests::{
        insert_binder, insert_card, insert_collection_entry_with_binder, insert_trade,
        insert_trade_card, insert_trading_binder, insert_user, insert_user_with_visibility,
        mark_trade_party_confirmed,
    };
    use crate::infrastructure::adapter_out::repository::trade_repository_adapter::TradeRepositoryAdapter;
    use chrono::{DateTime, TimeZone, Utc};
    use sqlx::PgPool;
    use uuid::Uuid;

    const GIVER: &str = "user_a";
    const RECEIVER: &str = "user_b";

    fn day(day: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, day, 0, 0, 0).unwrap()
    }

    /// An entry of the test card, as found in the Collection.
    #[derive(Debug, PartialEq, Eq, sqlx::FromRow)]
    struct Held {
        binder: Option<String>,
        foil: bool,
        language: String,
        quantity: i32,
        purchase_price: Option<i32>,
    }

    fn held(binder: Option<&str>, foil: bool, quantity: i32) -> Held {
        Held {
            binder: binder.map(str::to_string),
            foil,
            language: "FR".to_string(),
            quantity,
            purchase_price: None,
        }
    }

    /// An entry the giver keeps: the test seeds the giver's entries at a price of 100.
    fn kept(binder: Option<&str>, foil: bool, quantity: i32) -> Held {
        Held {
            purchase_price: Some(100),
            ..held(binder, foil, quantity)
        }
    }

    async fn seed(pool: &PgPool) {
        insert_user(pool, GIVER, "alice").await;
        insert_user(pool, RECEIVER, "bob").await;
        insert_card(pool, "FDN", "87", "FR", "Goblin Boarders", 1).await;
    }

    /// A `FULLY_ACCEPTED` trade, confirmed by the initiator (`GIVER`) only: the respondent's
    /// confirmation completes it.
    async fn awaiting_last_confirmation(pool: &PgPool, initiator: &str, respondent: &str) -> Uuid {
        let trade_id = Uuid::new_v4();
        insert_trade(pool, trade_id, initiator, respondent, "FULLY_ACCEPTED").await;
        mark_trade_party_confirmed(pool, trade_id, true).await;
        trade_id
    }

    async fn add_entry(
        pool: &PgPool,
        user: &str,
        foil: bool,
        quantity: i32,
        price: i32,
        binder: Option<&str>,
    ) {
        insert_collection_entry_with_binder(
            pool,
            "FDN",
            "87",
            "FR",
            foil,
            user,
            quantity,
            price,
            day(10),
            binder,
        )
        .await;
    }

    async fn complete(pool: &PgPool, trade_id: Uuid) -> bool {
        let repository = TradeRepositoryAdapter::new(pool.clone());
        let trade: Trade = repository
            .find_by_id(TradeId(trade_id))
            .await
            .unwrap()
            .unwrap();
        let transition = trade.confirm(Party::Respondent).unwrap();
        repository.apply_transition(&transition).await.unwrap()
    }

    async fn status(pool: &PgPool, trade_id: Uuid) -> TradeStatus {
        let repository = TradeRepositoryAdapter::new(pool.clone());
        repository
            .find_by_id(TradeId(trade_id))
            .await
            .unwrap()
            .unwrap()
            .status
    }

    /// What `user` holds of any card, by binder name, then finish.
    async fn collection_of(pool: &PgPool, user: &str) -> Vec<Held> {
        sqlx::query_as(
            r#"SELECT b.name AS binder, ce.foil, ce.language_code AS language, ce.quantity,
                      ce.purchase_price
                 FROM collection_entry ce LEFT JOIN binder b ON b.id = ce.binder_id
                WHERE ce.user_id = $1
                ORDER BY b.name NULLS FIRST, ce.foil, ce.collector_number"#,
        )
        .bind(user)
        .fetch_all(pool)
        .await
        .unwrap()
    }

    async fn trades_binders_of(pool: &PgPool, user: &str) -> Vec<(Uuid, bool)> {
        sqlx::query_as(
            r#"SELECT b.id, EXISTS(SELECT 1 FROM trading_binders tb WHERE tb.binder_id = b.id)
                 FROM binder b WHERE b.user_id = $1 AND b.name = 'trades'"#,
        )
        .bind(user)
        .fetch_all(pool)
        .await
        .unwrap()
    }

    #[sqlx::test]
    async fn completing_moves_the_cards_of_both_sides_keeping_finish_and_language(pool: PgPool) {
        seed(&pool).await;
        insert_card(&pool, "FDN", "88", "EN", "Other Card", 2).await;
        add_entry(&pool, GIVER, false, 3, 100, None).await;
        insert_collection_entry_with_binder(
            &pool,
            "FDN",
            "88",
            "EN",
            true,
            RECEIVER,
            2,
            50,
            day(10),
            None,
        )
        .await;
        let trade_id = awaiting_last_confirmation(&pool, GIVER, RECEIVER).await;
        insert_trade_card(&pool, trade_id, "FDN", "87", "FR", false, GIVER, 2).await;
        insert_trade_card(&pool, trade_id, "FDN", "88", "EN", true, RECEIVER, 1).await;

        assert!(complete(&pool, trade_id).await);

        assert_eq!(status(&pool, trade_id).await, TradeStatus::Completed);
        assert_eq!(
            collection_of(&pool, GIVER).await,
            vec![
                kept(None, false, 1),
                Held {
                    language: "EN".to_string(),
                    ..held(Some("trades"), true, 1)
                },
            ]
        );
        assert_eq!(
            collection_of(&pool, RECEIVER).await,
            vec![
                Held {
                    language: "EN".to_string(),
                    purchase_price: Some(50),
                    ..held(None, true, 1)
                },
                held(Some("trades"), false, 2),
            ]
        );
    }

    #[sqlx::test]
    async fn withdrawal_takes_open_binders_then_other_binders_then_no_binder_by_name(pool: PgPool) {
        seed(&pool).await;
        add_entry(&pool, GIVER, false, 4, 100, None).await;
        add_entry(&pool, GIVER, false, 3, 100, Some("Beta")).await;
        add_entry(&pool, GIVER, false, 2, 100, Some("Alpha")).await;
        add_entry(&pool, GIVER, false, 1, 100, Some("Zeta open")).await;
        add_entry(&pool, GIVER, false, 1, 100, Some("Mu open")).await;
        insert_trading_binder(&pool, GIVER, "Zeta open").await;
        insert_trading_binder(&pool, GIVER, "Mu open").await;
        let trade_id = awaiting_last_confirmation(&pool, GIVER, RECEIVER).await;
        // Both open binders (2), Alpha (2), then 1 of Beta.
        insert_trade_card(&pool, trade_id, "FDN", "87", "FR", false, GIVER, 5).await;

        assert!(complete(&pool, trade_id).await);

        assert_eq!(
            collection_of(&pool, GIVER).await,
            vec![kept(None, false, 4), kept(Some("Beta"), false, 2)]
        );
    }

    #[sqlx::test]
    async fn withdrawal_reaches_the_entry_without_binder_last_and_drops_emptied_entries(
        pool: PgPool,
    ) {
        seed(&pool).await;
        add_entry(&pool, GIVER, false, 4, 100, None).await;
        add_entry(&pool, GIVER, false, 2, 100, Some("Alpha")).await;
        let trade_id = awaiting_last_confirmation(&pool, GIVER, RECEIVER).await;
        insert_trade_card(&pool, trade_id, "FDN", "87", "FR", false, GIVER, 3).await;

        assert!(complete(&pool, trade_id).await);

        assert_eq!(
            collection_of(&pool, GIVER).await,
            vec![kept(None, false, 3)]
        );
    }

    #[sqlx::test]
    async fn withdrawal_leaves_the_other_finish_untouched(pool: PgPool) {
        seed(&pool).await;
        add_entry(&pool, GIVER, false, 1, 100, None).await;
        add_entry(&pool, GIVER, true, 5, 100, None).await;
        let trade_id = awaiting_last_confirmation(&pool, GIVER, RECEIVER).await;
        insert_trade_card(&pool, trade_id, "FDN", "87", "FR", false, GIVER, 1).await;

        assert!(complete(&pool, trade_id).await);

        assert_eq!(collection_of(&pool, GIVER).await, vec![kept(None, true, 5)]);
    }

    #[sqlx::test]
    async fn a_public_player_follows_the_same_order(pool: PgPool) {
        seed(&pool).await;
        insert_user_with_visibility(&pool, "user_p", "pat", "public").await;
        add_entry(&pool, "user_p", false, 1, 100, Some("Alpha")).await;
        add_entry(&pool, "user_p", false, 1, 100, Some("Zeta open")).await;
        insert_trading_binder(&pool, "user_p", "Zeta open").await;
        let trade_id = awaiting_last_confirmation(&pool, "user_p", RECEIVER).await;
        insert_trade_card(&pool, trade_id, "FDN", "87", "FR", false, "user_p", 1).await;

        assert!(complete(&pool, trade_id).await);

        assert_eq!(
            collection_of(&pool, "user_p").await,
            vec![kept(Some("Alpha"), false, 1)]
        );
    }

    #[sqlx::test]
    async fn a_shortfall_empties_the_giver_and_the_receiver_gets_the_full_quantity(pool: PgPool) {
        seed(&pool).await;
        add_entry(&pool, GIVER, false, 1, 100, Some("Alpha")).await;
        let trade_id = awaiting_last_confirmation(&pool, GIVER, RECEIVER).await;
        insert_trade_card(&pool, trade_id, "FDN", "87", "FR", false, GIVER, 3).await;

        assert!(complete(&pool, trade_id).await);

        assert_eq!(status(&pool, trade_id).await, TradeStatus::Completed);
        assert!(collection_of(&pool, GIVER).await.is_empty());
        assert_eq!(
            collection_of(&pool, RECEIVER).await,
            vec![held(Some("trades"), false, 3)]
        );
    }

    #[sqlx::test]
    async fn the_trades_binder_is_created_closed_then_reused(pool: PgPool) {
        seed(&pool).await;
        add_entry(&pool, GIVER, false, 2, 100, None).await;
        let first = awaiting_last_confirmation(&pool, GIVER, RECEIVER).await;
        insert_trade_card(&pool, first, "FDN", "87", "FR", false, GIVER, 1).await;
        assert!(complete(&pool, first).await);
        let created = trades_binders_of(&pool, RECEIVER).await;
        assert_eq!(created.len(), 1);
        assert!(!created[0].1, "the created binder is closed to trade");

        let second = awaiting_last_confirmation(&pool, GIVER, RECEIVER).await;
        insert_trade_card(&pool, second, "FDN", "87", "FR", false, GIVER, 1).await;
        assert!(complete(&pool, second).await);

        assert_eq!(trades_binders_of(&pool, RECEIVER).await, created);
        assert_eq!(
            collection_of(&pool, RECEIVER).await,
            vec![held(Some("trades"), false, 2)]
        );
    }

    #[sqlx::test]
    async fn a_trades_binder_made_by_the_player_is_reused(pool: PgPool) {
        seed(&pool).await;
        let own = insert_binder(&pool, RECEIVER, "trades").await;
        insert_trading_binder(&pool, RECEIVER, "trades").await;
        add_entry(&pool, GIVER, false, 1, 100, None).await;
        let trade_id = awaiting_last_confirmation(&pool, GIVER, RECEIVER).await;
        insert_trade_card(&pool, trade_id, "FDN", "87", "FR", false, GIVER, 1).await;

        assert!(complete(&pool, trade_id).await);

        assert_eq!(trades_binders_of(&pool, RECEIVER).await, vec![(own, true)]);
        assert_eq!(
            collection_of(&pool, RECEIVER).await,
            vec![held(Some("trades"), false, 1)]
        );
    }

    #[sqlx::test]
    async fn a_received_card_merges_with_the_existing_entry(pool: PgPool) {
        seed(&pool).await;
        add_entry(&pool, GIVER, false, 3, 100, None).await;
        // Added on the 10th (day 10) with a price: the transfer is later and brings no price.
        add_entry(&pool, RECEIVER, false, 2, 300, Some("trades")).await;
        let trade_id = awaiting_last_confirmation(&pool, GIVER, RECEIVER).await;
        insert_trade_card(&pool, trade_id, "FDN", "87", "FR", false, GIVER, 3).await;

        assert!(complete(&pool, trade_id).await);

        assert_eq!(
            collection_of(&pool, RECEIVER).await,
            vec![Held {
                purchase_price: Some(300),
                ..held(Some("trades"), false, 5)
            }]
        );
        let added_at: DateTime<Utc> =
            sqlx::query_scalar("SELECT added_at FROM collection_entry WHERE user_id = $1")
                .bind(RECEIVER)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(added_at, day(10));
    }

    #[sqlx::test]
    async fn a_merge_is_capped_at_the_entry_bound(pool: PgPool) {
        seed(&pool).await;
        add_entry(&pool, GIVER, false, 20, 100, None).await;
        add_entry(&pool, RECEIVER, false, 250, 100, Some("trades")).await;
        let trade_id = awaiting_last_confirmation(&pool, GIVER, RECEIVER).await;
        insert_trade_card(&pool, trade_id, "FDN", "87", "FR", false, GIVER, 20).await;

        assert!(complete(&pool, trade_id).await);

        let received = collection_of(&pool, RECEIVER).await;
        assert_eq!(received.len(), 1);
        assert_eq!(received[0].quantity, MAX_ENTRY_QUANTITY as i32);
    }

    #[sqlx::test]
    async fn a_first_confirmation_transfers_nothing(pool: PgPool) {
        seed(&pool).await;
        add_entry(&pool, GIVER, false, 1, 100, None).await;
        let trade_id = Uuid::new_v4();
        insert_trade(&pool, trade_id, GIVER, RECEIVER, "FULLY_ACCEPTED").await;
        insert_trade_card(&pool, trade_id, "FDN", "87", "FR", false, GIVER, 1).await;

        assert!(complete(&pool, trade_id).await);

        assert_eq!(status(&pool, trade_id).await, TradeStatus::FullyAccepted);
        assert_eq!(
            collection_of(&pool, GIVER).await,
            vec![Held {
                purchase_price: Some(100),
                ..held(None, false, 1)
            }]
        );
        assert!(collection_of(&pool, RECEIVER).await.is_empty());
    }

    #[sqlx::test]
    async fn a_stale_transition_transfers_nothing(pool: PgPool) {
        seed(&pool).await;
        add_entry(&pool, GIVER, false, 1, 100, None).await;
        let trade_id = awaiting_last_confirmation(&pool, GIVER, RECEIVER).await;
        insert_trade_card(&pool, trade_id, "FDN", "87", "FR", false, GIVER, 1).await;
        let repository = TradeRepositoryAdapter::new(pool.clone());
        let stale = repository
            .find_by_id(TradeId(trade_id))
            .await
            .unwrap()
            .unwrap()
            .confirm(Party::Respondent)
            .unwrap();
        // The trade is abandoned meanwhile.
        sqlx::query("UPDATE trade SET status = 'ABANDONED' WHERE id = $1")
            .bind(trade_id)
            .execute(&pool)
            .await
            .unwrap();

        assert!(!repository.apply_transition(&stale).await.unwrap());

        assert_eq!(collection_of(&pool, GIVER).await.len(), 1);
        assert!(collection_of(&pool, RECEIVER).await.is_empty());
    }

    #[sqlx::test]
    async fn transferred_cards_are_no_longer_proposed(pool: PgPool) {
        seed(&pool).await;
        insert_user_with_visibility(&pool, "user_p", "pat", "public").await;
        add_entry(&pool, "user_p", false, 3, 100, None).await;
        let trade_id = awaiting_last_confirmation(&pool, "user_p", RECEIVER).await;
        insert_trade_card(&pool, trade_id, "FDN", "87", "FR", false, "user_p", 2).await;

        assert!(complete(&pool, trade_id).await);

        let proposed: i32 = sqlx::query_scalar(
            r#"SELECT COALESCE(SUM(proposed_quantity), 0)::INT FROM v_tradable_entry
                WHERE user_id = 'user_p'"#,
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(proposed, 1);
    }
}
