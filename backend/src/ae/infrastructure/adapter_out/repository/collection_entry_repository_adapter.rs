use crate::application::error::{AppError, InfraError};
use crate::application::repository::CollectionEntryRepository;
use crate::domain::binder::BinderId;
use crate::domain::card::CardId;
use crate::domain::collection_entry::{
    CardEntries, CollectionEntryDetail, CollectionEntryId, EntryBinder, EntryCopies, EntryUpdate,
    ReservedQuantities, ensure_covers_reserved,
};
use crate::domain::error::FunctionalError;
use crate::domain::language_code::LanguageCode;
use crate::domain::purchase::{NewSingleCardPurchase, PurchaseId};
use crate::domain::scan_batch::{ScanBatchCopies, ScanBatchId, ScanBatchOutcome};
use crate::domain::user::UserId;
use crate::infrastructure::adapter_out::repository::entities::invalid_db_value;
use crate::infrastructure::adapter_out::repository::sealed_product_content::{
    add_content_line, ensure_sealed_product, lock_sealed_product,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgConnection, Pool, Postgres};
use uuid::Uuid;

pub struct CollectionEntryRepositoryAdapter {
    pool: Pool<Postgres>,
}

impl CollectionEntryRepositoryAdapter {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

/// An entry of the card being edited, read under lock.
pub(super) struct LockedEntry {
    pub(super) id: Uuid,
    pub(super) foil: bool,
    pub(super) binder_id: Option<Uuid>,
    quantity: u32,
    purchase_price: Option<u32>,
    added_at: DateTime<Utc>,
}

impl LockedEntry {
    pub(super) fn copies(&self) -> EntryCopies {
        EntryCopies {
            quantity: self.quantity,
            purchase_price: self.purchase_price,
            added_at: self.added_at,
        }
    }
}

/// Every entry of a card the player owns, locked, and the one an operation targets.
struct LockedCard {
    card_id: CardId,
    entries: Vec<LockedEntry>,
    target: usize,
}

impl LockedCard {
    fn entry(&self) -> &LockedEntry {
        &self.entries[self.target]
    }

    /// Copies owned in a finish, all binders together.
    fn owned(&self, foil: bool) -> u32 {
        self.entries
            .iter()
            .filter(|entry| entry.foil == foil)
            .map(|entry| entry.quantity)
            .sum()
    }

    /// The entry of the finish and binder, if there is one.
    fn find(&self, foil: bool, binder_id: Option<Uuid>) -> Option<&LockedEntry> {
        find_entry(&self.entries, foil, binder_id)
    }
}

/// The entry of the finish and binder among `entries`, if there is one.
pub(super) fn find_entry(
    entries: &[LockedEntry],
    foil: bool,
    binder_id: Option<Uuid>,
) -> Option<&LockedEntry> {
    entries
        .iter()
        .find(|entry| entry.foil == foil && entry.binder_id == binder_id)
}

fn corrupt(what: &str) -> AppError {
    InfraError::RepositoryError(format!("collection_entry holds an out-of-range {what}")).into()
}

/// A purchase price as the domain holds it, from its column.
fn purchase_price_from_column(price: Option<i32>) -> Result<Option<u32>, AppError> {
    price
        .map(|price| u32::try_from(price).map_err(|_| corrupt("purchase price")))
        .transpose()
}

/// A purchase price as its column holds it.
fn purchase_price_to_column(price: Option<u32>) -> Result<Option<i32>, AppError> {
    price
        .map(|price| i32::try_from(price).map_err(|_| corrupt("purchase price")))
        .transpose()
}

/// `BinderNotFound` unless the binder is the player's.
async fn ensure_binder_owned(
    conn: &mut PgConnection,
    user_id: &UserId,
    binder_id: BinderId,
) -> Result<(), AppError> {
    let owned = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM binder WHERE id = $1 AND user_id = $2) AS "owned!""#,
        binder_id.0,
        user_id.as_str(),
    )
    .fetch_one(conn)
    .await?;
    if owned {
        Ok(())
    } else {
        Err(FunctionalError::BinderNotFound.into())
    }
}

/// What the first validation of the scan batch answered, `None` when it was never recorded.
async fn recorded_scan_batch(
    conn: &mut PgConnection,
    user_id: &UserId,
    batch_id: ScanBatchId,
) -> Result<Option<ScanBatchOutcome>, AppError> {
    let copies_added = sqlx::query_scalar!(
        "SELECT copies_added FROM scan_batch WHERE user_id = $1 AND id = $2",
        user_id.as_str(),
        batch_id.0,
    )
    .fetch_optional(conn)
    .await?;
    copies_added
        .map(|copies_added| {
            Ok(ScanBatchOutcome {
                copies_added: u32::try_from(copies_added)
                    .map_err(|_| invalid_db_value("copies added", &copies_added.to_string()))?,
                already_validated: true,
            })
        })
        .transpose()
}

/// The three parts of a card's identity, as the columns hold them.
fn card_columns(card_id: &CardId) -> (String, &str, String) {
    (
        card_id.set_code.to_string(),
        card_id.collector_number.as_str(),
        card_id.language_code.to_string(),
    )
}

/// Locks every entry of the card the player owns, both finishes, in a fixed order so two edits
/// of the same card queue instead of deadlocking. The first acceptance of a Trade takes the same
/// locks, shared, before it commits the reservation (`apply_transition`): an edit and an
/// acceptance of a Trade holding the card never interleave, so the reservation guard reads what
/// the edit then writes against.
pub(super) async fn lock_card_entries(
    conn: &mut PgConnection,
    user_id: &UserId,
    card_id: &CardId,
) -> Result<Vec<LockedEntry>, AppError> {
    let (set_code, collector_number, language_code) = card_columns(card_id);
    let rows = sqlx::query!(
        r#"
        SELECT id,
               foil,
               binder_id AS "binder_id?",
               quantity,
               purchase_price,
               COALESCE(added_at, NOW()) AS "added_at!"
        FROM collection_entry
        WHERE user_id = $1 AND set_code = $2 AND collector_number = $3 AND language_code = $4
        ORDER BY id
        FOR UPDATE
        "#,
        user_id.as_str(),
        set_code,
        collector_number,
        language_code,
    )
    .fetch_all(conn)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(LockedEntry {
                id: row.id,
                foil: row.foil,
                binder_id: row.binder_id,
                quantity: u32::try_from(row.quantity).map_err(|_| corrupt("quantity"))?,
                purchase_price: purchase_price_from_column(row.purchase_price)?,
                added_at: row.added_at,
            })
        })
        .collect()
}

/// Locks every entry of the card the player owns, with the one `id` designates as the target:
/// `CollectionEntryNotFound` for an absent entry or another player's.
async fn lock_entry(
    conn: &mut PgConnection,
    user_id: &UserId,
    id: CollectionEntryId,
) -> Result<LockedCard, AppError> {
    let card_id = card_of_entry(conn, user_id, id).await?;
    let entries = lock_card_entries(conn, user_id, &card_id).await?;
    let target = entries
        .iter()
        .position(|entry| entry.id == id.0)
        .ok_or(FunctionalError::CollectionEntryNotFound)?;
    Ok(LockedCard {
        card_id,
        entries,
        target,
    })
}

/// The copies the accepted Trades commit of the player's card in each finish: the sum of their
/// `trade_card` quantities in `ONE_ACCEPTED` and `FULLY_ACCEPTED` Trades. Read, never stored
/// (ADR-0008).
async fn reserved_quantities(
    conn: &mut PgConnection,
    user_id: &UserId,
    card_id: &CardId,
) -> Result<ReservedQuantities, AppError> {
    let (set_code, collector_number, language_code) = card_columns(card_id);
    let rows = sqlx::query!(
        r#"
        SELECT tc.foil, SUM(tc.quantity)::BIGINT AS "reserved!"
        FROM trade_card tc
                 JOIN trade t ON t.id = tc.trade_id
        WHERE tc.owner_user_id = $1
          AND tc.set_code = $2 AND tc.collector_number = $3 AND tc.language_code = $4
          AND t.status IN ('ONE_ACCEPTED', 'FULLY_ACCEPTED')
        GROUP BY tc.foil
        "#,
        user_id.as_str(),
        set_code,
        collector_number,
        language_code,
    )
    .fetch_all(conn)
    .await?;

    let mut reserved = ReservedQuantities::default();
    for row in rows {
        let quantity = u32::try_from(row.reserved).map_err(|_| corrupt("reserved quantity"))?;
        if row.foil {
            reserved.foil = quantity;
        } else {
            reserved.normal = quantity;
        }
    }
    Ok(reserved)
}

async fn find_card_entries(
    conn: &mut PgConnection,
    user_id: &UserId,
    card_id: &CardId,
) -> Result<CardEntries, AppError> {
    let (set_code, collector_number, language_code) = card_columns(card_id);
    let rows = sqlx::query!(
        r#"
        SELECT ce.id,
               ce.foil,
               b.id AS "binder_id?",
               b.name AS "binder_name?",
               ce.quantity,
               ce.purchase_price,
               COALESCE(ce.added_at, NOW()) AS "added_at!"
        FROM collection_entry ce
                 LEFT JOIN binder b ON b.id = ce.binder_id
        WHERE ce.user_id = $1 AND ce.set_code = $2 AND ce.collector_number = $3
          AND ce.language_code = $4
        ORDER BY ce.foil, b.name NULLS FIRST, ce.id
        "#,
        user_id.as_str(),
        set_code,
        collector_number,
        language_code,
    )
    .fetch_all(&mut *conn)
    .await?;

    let entries = rows
        .into_iter()
        .map(|row| {
            let binder = match (row.binder_id, row.binder_name) {
                (Some(id), Some(name)) => Some(EntryBinder {
                    id: BinderId(id),
                    name,
                }),
                _ => None,
            };
            Ok(CollectionEntryDetail {
                id: CollectionEntryId(row.id),
                foil: row.foil,
                binder,
                quantity: u8::try_from(row.quantity).map_err(|_| corrupt("quantity"))?,
                purchase_price: purchase_price_from_column(row.purchase_price)?,
                added_at: row.added_at,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;

    let reserved = reserved_quantities(conn, user_id, card_id).await?;
    Ok(CardEntries {
        entries,
        reserved_normal: reserved.normal,
        reserved_foil: reserved.foil,
    })
}

/// The card an entry of the player belongs to; `CollectionEntryNotFound` for an absent entry or
/// another player's.
async fn card_of_entry(
    conn: &mut PgConnection,
    user_id: &UserId,
    id: CollectionEntryId,
) -> Result<CardId, AppError> {
    let row = sqlx::query!(
        r#"
        SELECT set_code, collector_number, language_code
        FROM collection_entry
        WHERE id = $1 AND user_id = $2
        "#,
        id.0,
        user_id.as_str(),
    )
    .fetch_optional(conn)
    .await?
    .ok_or(FunctionalError::CollectionEntryNotFound)?;

    Ok(CardId::try_new(
        row.set_code.as_str(),
        row.collector_number,
        LanguageCode::try_new(&row.language_code)?,
    )?)
}

pub(super) async fn write_copies(
    conn: &mut PgConnection,
    id: Uuid,
    copies: &EntryCopies,
) -> Result<(), AppError> {
    sqlx::query!(
        "UPDATE collection_entry SET quantity = $2, purchase_price = $3, added_at = $4 WHERE id = $1",
        id,
        i32::try_from(copies.quantity).map_err(|_| corrupt("quantity"))?,
        purchase_price_to_column(copies.purchase_price)?,
        copies.added_at,
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub(super) async fn insert_entry(
    conn: &mut PgConnection,
    user_id: &UserId,
    card_id: &CardId,
    foil: bool,
    binder_id: Option<Uuid>,
    copies: &EntryCopies,
) -> Result<(), AppError> {
    let (set_code, collector_number, language_code) = card_columns(card_id);
    sqlx::query!(
        r#"
        INSERT INTO collection_entry
            (set_code, collector_number, language_code, foil, user_id, quantity, purchase_price,
             added_at, binder_id)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
        set_code,
        collector_number,
        language_code,
        foil,
        user_id.as_str(),
        i32::try_from(copies.quantity).map_err(|_| corrupt("quantity"))?,
        purchase_price_to_column(copies.purchase_price)?,
        copies.added_at,
        binder_id,
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Adds `quantity` copies, each with `purchase_price` when given, to the entry of the same finish
/// and binder, or to a new one, within the caller's transaction. Returns the entries of the card.
pub(super) async fn add_copies(
    conn: &mut PgConnection,
    user_id: &UserId,
    card_id: &CardId,
    foil: bool,
    quantity: u8,
    binder_id: Option<BinderId>,
    purchase_price: Option<u32>,
) -> Result<CardEntries, AppError> {
    if let Some(binder_id) = binder_id {
        ensure_binder_owned(conn, user_id, binder_id).await?;
    }

    let locked = lock_card_entries(conn, user_id, card_id).await?;
    let added = EntryCopies {
        quantity: u32::from(quantity),
        purchase_price,
        added_at: Utc::now(),
    };
    let target_binder = binder_id.map(|id| id.0);
    let same_copy = locked
        .iter()
        .find(|entry| entry.foil == foil && entry.binder_id == target_binder);
    match same_copy {
        Some(existing) => {
            let merged = existing.copies().merge(&added)?;
            write_copies(conn, existing.id, &merged).await?;
        }
        None => {
            insert_entry(conn, user_id, card_id, foil, target_binder, &added).await?;
        }
    }

    find_card_entries(conn, user_id, card_id).await
}

/// Records the Purchase of the copies just added, with its one line of content. Its content has no
/// link to the entry, so what the entry later becomes does not change it (ADR-0023).
async fn insert_single_card_purchase(
    conn: &mut PgConnection,
    user_id: &UserId,
    card_id: &CardId,
    foil: bool,
    quantity: u8,
    purchase: &NewSingleCardPurchase,
) -> Result<(), AppError> {
    let purchase_id = sqlx::query_scalar!(
        r#"
        INSERT INTO purchase (user_id, kind, amount_cents, purchased_on)
        VALUES ($1, 'single_card', $2, $3)
        RETURNING id
        "#,
        user_id.as_str(),
        i32::try_from(purchase.amount.cents()).map_err(|_| corrupt("amount"))?,
        purchase.purchased_on,
    )
    .fetch_one(&mut *conn)
    .await?;

    let (set_code, collector_number, language_code) = card_columns(card_id);
    sqlx::query!(
        r#"
        INSERT INTO purchase_content
            (purchase_id, set_code, collector_number, language_code, foil, quantity)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        purchase_id,
        set_code,
        collector_number,
        language_code,
        foil,
        i32::from(quantity),
    )
    .execute(conn)
    .await?;
    Ok(())
}

#[async_trait]
impl CollectionEntryRepository for CollectionEntryRepositoryAdapter {
    #[tracing::instrument(name = "collection_entry_repo.find_by_card", skip_all, fields(sentry.op = "db"))]
    async fn find_by_card(
        &self,
        user_id: &UserId,
        card_id: &CardId,
    ) -> Result<CardEntries, AppError> {
        let mut conn = self.pool.acquire().await?;
        find_card_entries(&mut conn, user_id, card_id).await
    }

    #[tracing::instrument(name = "collection_entry_repo.add", skip_all, fields(sentry.op = "db"))]
    async fn add(
        &self,
        user_id: &UserId,
        card_id: &CardId,
        foil: bool,
        quantity: u8,
        binder_id: Option<BinderId>,
        purchase: Option<NewSingleCardPurchase>,
    ) -> Result<CardEntries, AppError> {
        let mut tx = self.pool.begin().await?;
        let entries = add_copies(
            &mut tx,
            user_id,
            card_id,
            foil,
            quantity,
            binder_id,
            purchase.map(|purchase| purchase.price_per_copy.cents()),
        )
        .await?;
        if let Some(purchase) = purchase {
            insert_single_card_purchase(&mut tx, user_id, card_id, foil, quantity, &purchase)
                .await?;
        }
        tx.commit().await?;
        Ok(entries)
    }

    #[tracing::instrument(
        name = "collection_entry_repo.find_scan_batch",
        skip_all,
        fields(sentry.op = "db")
    )]
    async fn find_scan_batch(
        &self,
        user_id: &UserId,
        batch_id: ScanBatchId,
    ) -> Result<Option<ScanBatchOutcome>, AppError> {
        let mut conn = self.pool.acquire().await?;
        recorded_scan_batch(&mut conn, user_id, batch_id).await
    }

    #[tracing::instrument(
        name = "collection_entry_repo.ensure_scan_batch_destination",
        skip_all,
        fields(sentry.op = "db")
    )]
    async fn ensure_scan_batch_destination(
        &self,
        user_id: &UserId,
        binder_id: Option<BinderId>,
        sealed_product_id: Option<PurchaseId>,
    ) -> Result<(), AppError> {
        let mut conn = self.pool.acquire().await?;
        if let Some(binder_id) = binder_id {
            ensure_binder_owned(&mut conn, user_id, binder_id).await?;
        }
        if let Some(sealed_product_id) = sealed_product_id {
            ensure_sealed_product(&mut conn, user_id, sealed_product_id).await?;
        }
        Ok(())
    }

    #[tracing::instrument(
        name = "collection_entry_repo.add_scan_batch",
        skip_all,
        fields(sentry.op = "db")
    )]
    async fn add_scan_batch(
        &self,
        user_id: &UserId,
        batch_id: ScanBatchId,
        binder_id: Option<BinderId>,
        sealed_product_id: Option<PurchaseId>,
        lines: &[ScanBatchCopies],
    ) -> Result<ScanBatchOutcome, AppError> {
        let mut tx = self.pool.begin().await?;

        let copies_added: u32 = lines.iter().map(|line| u32::from(line.quantity)).sum();
        // A concurrent send of the same batch waits here for the first one to commit.
        let recorded = sqlx::query!(
            "INSERT INTO scan_batch (user_id, id, copies_added) VALUES ($1, $2, $3)
             ON CONFLICT DO NOTHING",
            user_id.as_str(),
            batch_id.0,
            i32::try_from(copies_added).map_err(|_| FunctionalError::InvalidQuantity)?,
        )
        .execute(&mut *tx)
        .await?;
        if recorded.rows_affected() == 0 {
            return recorded_scan_batch(&mut tx, user_id, batch_id)
                .await?
                .ok_or_else(|| {
                    AppError::Infra(InfraError::RepositoryError(
                        "a recorded scan batch vanished".to_string(),
                    ))
                });
        }

        if let Some(sealed_product_id) = sealed_product_id {
            lock_sealed_product(&mut tx, user_id, sealed_product_id).await?;
        }
        for line in lines {
            add_copies(
                &mut tx,
                user_id,
                &line.card_id,
                line.foil,
                line.quantity,
                binder_id,
                None,
            )
            .await?;
            if let Some(sealed_product_id) = sealed_product_id {
                add_content_line(
                    &mut tx,
                    sealed_product_id,
                    &line.card_id,
                    line.foil,
                    line.quantity,
                )
                .await?;
            }
        }

        tx.commit().await?;
        Ok(ScanBatchOutcome {
            copies_added,
            already_validated: false,
        })
    }

    #[tracing::instrument(name = "collection_entry_repo.update", skip_all, fields(sentry.op = "db"))]
    async fn update(
        &self,
        user_id: &UserId,
        id: CollectionEntryId,
        update: EntryUpdate,
    ) -> Result<CardEntries, AppError> {
        let mut tx = self.pool.begin().await?;
        let locked = lock_entry(&mut tx, user_id, id).await?;
        let entry = locked.entry();
        let reserved = reserved_quantities(&mut tx, user_id, &locked.card_id).await?;

        let quantity = update.quantity.map_or(entry.quantity, u32::from);
        let foil = update.foil.unwrap_or(entry.foil);
        let updated = EntryCopies {
            quantity,
            purchase_price: update.purchase_price.or(entry.purchase_price),
            ..entry.copies()
        };

        if foil == entry.foil {
            if quantity < entry.quantity {
                ensure_covers_reserved(
                    locked.owned(foil) - entry.quantity + quantity,
                    reserved.of(foil),
                )?;
            }
            write_copies(&mut tx, entry.id, &updated).await?;
        } else {
            // The entry leaves its finish whatever its new quantity, so the old finish loses all
            // of it.
            ensure_covers_reserved(
                locked.owned(entry.foil) - entry.quantity,
                reserved.of(entry.foil),
            )?;
            match locked.find(foil, entry.binder_id) {
                Some(target) => {
                    let merged = target.copies().merge(&updated)?;
                    write_copies(&mut tx, target.id, &merged).await?;
                    sqlx::query!("DELETE FROM collection_entry WHERE id = $1", entry.id)
                        .execute(&mut *tx)
                        .await?;
                }
                None => {
                    write_copies(&mut tx, entry.id, &updated).await?;
                    sqlx::query!(
                        "UPDATE collection_entry SET foil = $2 WHERE id = $1",
                        entry.id,
                        foil
                    )
                    .execute(&mut *tx)
                    .await?;
                }
            }
        }

        let entries = find_card_entries(&mut tx, user_id, &locked.card_id).await?;
        tx.commit().await?;
        Ok(entries)
    }

    #[tracing::instrument(name = "collection_entry_repo.move_to_binder", skip_all, fields(sentry.op = "db"))]
    async fn move_to_binder(
        &self,
        user_id: &UserId,
        id: CollectionEntryId,
        binder_id: Option<BinderId>,
        quantity: Option<u8>,
    ) -> Result<CardEntries, AppError> {
        let mut tx = self.pool.begin().await?;
        let locked = lock_entry(&mut tx, user_id, id).await?;
        let entry = locked.entry();

        if let Some(binder_id) = binder_id {
            ensure_binder_owned(&mut tx, user_id, binder_id).await?;
        }

        let moved = quantity.map_or(entry.quantity, u32::from);
        if moved > entry.quantity {
            return Err(FunctionalError::InvalidQuantity.into());
        }
        let target_binder = binder_id.map(|binder| binder.0);

        // Moving into the binder the entry already is in leaves everything as it is.
        if target_binder != entry.binder_id {
            let target = locked.find(entry.foil, target_binder);
            let moved_copies = EntryCopies {
                quantity: moved,
                ..entry.copies()
            };

            match target {
                Some(target) => {
                    let merged = target.copies().merge(&moved_copies)?;
                    write_copies(&mut tx, target.id, &merged).await?;
                }
                None if moved == entry.quantity => {
                    sqlx::query!(
                        "UPDATE collection_entry SET binder_id = $2 WHERE id = $1",
                        entry.id,
                        target_binder
                    )
                    .execute(&mut *tx)
                    .await?;
                }
                None => {
                    insert_entry(
                        &mut tx,
                        user_id,
                        &locked.card_id,
                        entry.foil,
                        target_binder,
                        &moved_copies,
                    )
                    .await?;
                }
            }

            if moved < entry.quantity {
                let left = EntryCopies {
                    quantity: entry.quantity - moved,
                    ..entry.copies()
                };
                write_copies(&mut tx, entry.id, &left).await?;
            } else if target.is_some() {
                sqlx::query!("DELETE FROM collection_entry WHERE id = $1", entry.id)
                    .execute(&mut *tx)
                    .await?;
            }
        }

        let entries = find_card_entries(&mut tx, user_id, &locked.card_id).await?;
        tx.commit().await?;
        Ok(entries)
    }

    #[tracing::instrument(name = "collection_entry_repo.remove", skip_all, fields(sentry.op = "db"))]
    async fn remove(&self, user_id: &UserId, id: CollectionEntryId) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;
        let locked = lock_entry(&mut tx, user_id, id).await?;
        let entry = locked.entry();
        let reserved = reserved_quantities(&mut tx, user_id, &locked.card_id).await?;

        ensure_covers_reserved(
            locked.owned(entry.foil) - entry.quantity,
            reserved.of(entry.foil),
        )?;
        sqlx::query!("DELETE FROM collection_entry WHERE id = $1", entry.id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::purchase::PurchaseAmount;
    use crate::infrastructure::adapter_out::repository::common_repository_tests::{
        insert_binder, insert_card, insert_set, insert_trade, insert_trade_card, insert_user,
    };
    use chrono::TimeZone;
    use sqlx::PgPool;

    const USER: &str = "user_1";
    const OTHER: &str = "user_2";

    fn card() -> CardId {
        CardId::new("TST", "1", LanguageCode::EN)
    }

    fn day(day: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, day, 0, 0, 0).unwrap()
    }

    async fn seed(pool: &PgPool) {
        insert_set(pool, "TST").await;
        insert_card(pool, "TST", "1", "EN", "Test Card", 1).await;
        insert_user(pool, USER, "alice").await;
        insert_user(pool, OTHER, "bob").await;
    }

    async fn insert_entry(
        pool: &PgPool,
        user_id: &str,
        foil: bool,
        quantity: i32,
        purchase_price: Option<i32>,
        added_at: DateTime<Utc>,
        binder: Option<&str>,
    ) -> CollectionEntryId {
        let binder_id = match binder {
            Some(name) => Some(insert_binder(pool, user_id, name).await),
            None => None,
        };
        let id = sqlx::query_scalar(
            r#"INSERT INTO collection_entry
                   (set_code, collector_number, language_code, foil, user_id, quantity,
                    purchase_price, added_at, binder_id)
               VALUES ('TST', '1', 'EN', $1, $2, $3, $4, $5, $6)
               RETURNING id"#,
        )
        .bind(foil)
        .bind(user_id)
        .bind(quantity)
        .bind(purchase_price)
        .bind(added_at)
        .bind(binder_id)
        .fetch_one(pool)
        .await
        .unwrap();
        CollectionEntryId(id)
    }

    /// Puts `quantity` copies of the finish from `USER` into a trade in `status`, with a
    /// counterpart of its own: a pair of players has one active trade at most.
    async fn commit(pool: &PgPool, status: &str, foil: bool, quantity: i32) {
        let trade_id = Uuid::new_v4();
        let counterpart = format!("counterpart_{trade_id}");
        insert_user(pool, &counterpart, &counterpart).await;
        insert_trade(pool, trade_id, &counterpart, USER, status).await;
        insert_trade_card(pool, trade_id, "TST", "1", "EN", foil, USER, quantity).await;
    }

    fn adapter(pool: PgPool) -> CollectionEntryRepositoryAdapter {
        CollectionEntryRepositoryAdapter::new(pool)
    }

    fn user() -> UserId {
        UserId::new(USER)
    }

    fn quantity_update(quantity: u8) -> EntryUpdate {
        EntryUpdate {
            quantity: Some(quantity),
            foil: None,
            ..EntryUpdate::default()
        }
    }

    fn foil_update(foil: bool) -> EntryUpdate {
        EntryUpdate {
            foil: Some(foil),
            ..EntryUpdate::default()
        }
    }

    // ---- add_scan_batch ----

    fn batch_id() -> ScanBatchId {
        ScanBatchId(Uuid::from_u128(42))
    }

    fn copies(card_id: CardId, foil: bool, quantity: u8) -> ScanBatchCopies {
        ScanBatchCopies {
            card_id,
            foil,
            quantity,
        }
    }

    async fn insert_purchase(pool: &PgPool, user_id: &str, kind: &str) -> PurchaseId {
        let id = if kind == "sealed_product" {
            sqlx::query_scalar(
                r#"INSERT INTO purchase
                       (user_id, kind, amount_cents, purchased_on, product_type, set_code)
                   VALUES ($1, 'sealed_product', 499, '2026-11-13', 'BOOSTER', 'TST')
                   RETURNING id"#,
            )
        } else {
            sqlx::query_scalar(
                r#"INSERT INTO purchase (user_id, kind, amount_cents, purchased_on)
                   VALUES ($1, 'single_card', 200, '2026-11-13') RETURNING id"#,
            )
        }
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap();
        PurchaseId(id)
    }

    async fn count(pool: &PgPool, table: &str) -> i64 {
        sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT COUNT(*) FROM {table}")))
            .fetch_one(pool)
            .await
            .unwrap()
    }

    async fn copies_held(pool: &PgPool, user_id: &str) -> i64 {
        sqlx::query_scalar(
            "SELECT COALESCE(SUM(quantity), 0)::BIGINT FROM collection_entry WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    #[sqlx::test]
    async fn a_batch_adds_its_copies_to_the_binder_without_purchase_price_or_purchase(
        pool: PgPool,
    ) {
        seed(&pool).await;
        insert_card(&pool, "TST", "2", "EN", "Other", 2).await;
        let binder = BinderId(insert_binder(&pool, USER, "Opened").await);
        let adapter = adapter(pool.clone());

        let outcome = adapter
            .add_scan_batch(
                &user(),
                batch_id(),
                Some(binder),
                None,
                &[
                    copies(card(), false, 2),
                    copies(CardId::new("TST", "2", LanguageCode::EN), true, 1),
                ],
            )
            .await
            .unwrap();

        assert_eq!(
            outcome,
            ScanBatchOutcome {
                copies_added: 3,
                already_validated: false
            }
        );
        let entries: Vec<(String, bool, i32, Option<i32>)> = sqlx::query_as(
            "SELECT collector_number, foil, quantity, purchase_price
             FROM collection_entry WHERE binder_id = $1 ORDER BY collector_number",
        )
        .bind(binder.0)
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(
            entries,
            vec![
                ("1".to_string(), false, 2, None),
                ("2".to_string(), true, 1, None),
            ]
        );
        assert_eq!(count(&pool, "purchase").await, 0);
        assert_eq!(count(&pool, "scan_batch").await, 1);
    }

    #[sqlx::test]
    async fn a_batch_merges_into_the_entry_of_the_same_card_finish_and_binder(pool: PgPool) {
        seed(&pool).await;
        insert_entry(&pool, USER, false, 3, Some(100), day(1), None).await;
        let adapter = adapter(pool.clone());

        adapter
            .add_scan_batch(
                &user(),
                batch_id(),
                None,
                None,
                &[copies(card(), false, 2), copies(card(), false, 1)],
            )
            .await
            .unwrap();

        let found = adapter.find_by_card(&user(), &card()).await.unwrap();
        assert_eq!(found.entries.len(), 1);
        assert_eq!(found.entries[0].quantity, 6);
        assert_eq!(found.entries[0].purchase_price, Some(100));
    }

    #[sqlx::test]
    async fn sending_the_same_batch_again_writes_nothing(pool: PgPool) {
        seed(&pool).await;
        let adapter = adapter(pool.clone());
        let lines = [copies(card(), false, 2)];
        adapter
            .add_scan_batch(&user(), batch_id(), None, None, &lines)
            .await
            .unwrap();

        let outcome = adapter
            .add_scan_batch(&user(), batch_id(), None, None, &lines)
            .await
            .unwrap();

        assert!(outcome.already_validated);
        assert_eq!(copies_held(&pool, USER).await, 2);
        assert_eq!(count(&pool, "scan_batch").await, 1);
    }

    #[sqlx::test]
    async fn sending_the_same_batch_again_answers_the_copies_of_the_first_send(pool: PgPool) {
        seed(&pool).await;
        let adapter = adapter(pool.clone());
        adapter
            .add_scan_batch(
                &user(),
                batch_id(),
                None,
                None,
                &[copies(card(), false, 2), copies(card(), true, 3)],
            )
            .await
            .unwrap();

        let again = adapter
            .add_scan_batch(&user(), batch_id(), None, None, &[copies(card(), false, 9)])
            .await
            .unwrap();

        assert_eq!(
            again,
            ScanBatchOutcome {
                copies_added: 5,
                already_validated: true
            }
        );
        assert_eq!(copies_held(&pool, USER).await, 5);
    }

    #[sqlx::test]
    async fn find_scan_batch_knows_only_the_batches_of_the_player_that_were_recorded(pool: PgPool) {
        seed(&pool).await;
        let adapter = adapter(pool.clone());
        assert_eq!(
            adapter.find_scan_batch(&user(), batch_id()).await.unwrap(),
            None
        );
        adapter
            .add_scan_batch(&user(), batch_id(), None, None, &[copies(card(), false, 4)])
            .await
            .unwrap();

        let found = adapter.find_scan_batch(&user(), batch_id()).await.unwrap();
        let of_another = adapter
            .find_scan_batch(&UserId::new(OTHER), batch_id())
            .await
            .unwrap();

        assert_eq!(
            found,
            Some(ScanBatchOutcome {
                copies_added: 4,
                already_validated: true
            })
        );
        assert_eq!(of_another, None);
    }

    #[sqlx::test]
    async fn the_destination_of_a_batch_is_checked_without_writing(pool: PgPool) {
        seed(&pool).await;
        let mine = BinderId(insert_binder(&pool, USER, "Mine").await);
        let theirs = BinderId(insert_binder(&pool, OTHER, "Bob's").await);
        let sealed_product = insert_purchase(&pool, USER, "sealed_product").await;
        let single_card = insert_purchase(&pool, USER, "single_card").await;
        let adapter = adapter(pool.clone());

        assert!(
            adapter
                .ensure_scan_batch_destination(&user(), Some(mine), Some(sealed_product))
                .await
                .is_ok()
        );
        assert!(
            adapter
                .ensure_scan_batch_destination(&user(), None, None)
                .await
                .is_ok()
        );
        let binder_error = adapter
            .ensure_scan_batch_destination(&user(), Some(theirs), None)
            .await
            .unwrap_err();
        assert_eq!(binder_error.code(), "binder_not_found");
        for id in [single_card, PurchaseId(Uuid::new_v4())] {
            let error = adapter
                .ensure_scan_batch_destination(&user(), None, Some(id))
                .await
                .unwrap_err();
            assert_eq!(error.code(), "purchase_not_found");
        }
        assert_eq!(count(&pool, "scan_batch").await, 0);
        assert_eq!(count(&pool, "collection_entry").await, 0);
    }

    #[sqlx::test]
    async fn the_same_batch_identifier_of_another_player_is_a_new_batch(pool: PgPool) {
        seed(&pool).await;
        let adapter = adapter(pool.clone());
        let lines = [copies(card(), false, 2)];
        adapter
            .add_scan_batch(&user(), batch_id(), None, None, &lines)
            .await
            .unwrap();

        let outcome = adapter
            .add_scan_batch(&UserId::new(OTHER), batch_id(), None, None, &lines)
            .await
            .unwrap();

        assert!(!outcome.already_validated);
        assert_eq!(copies_held(&pool, OTHER).await, 2);
    }

    #[sqlx::test]
    async fn a_card_the_database_lacks_writes_nothing_and_leaves_the_batch_unrecorded(
        pool: PgPool,
    ) {
        seed(&pool).await;
        let adapter = adapter(pool.clone());

        let result = adapter
            .add_scan_batch(
                &user(),
                batch_id(),
                None,
                None,
                &[
                    copies(card(), false, 2),
                    copies(CardId::new("TST", "999", LanguageCode::EN), false, 1),
                ],
            )
            .await;

        assert!(result.is_err());
        assert_eq!(count(&pool, "collection_entry").await, 0);
        assert_eq!(count(&pool, "scan_batch").await, 0);
    }

    #[sqlx::test]
    async fn a_merge_over_255_copies_writes_nothing(pool: PgPool) {
        seed(&pool).await;
        insert_entry(&pool, USER, false, 255, None, day(1), None).await;
        insert_card(&pool, "TST", "2", "EN", "Other", 2).await;
        let adapter = adapter(pool.clone());

        let error = adapter
            .add_scan_batch(
                &user(),
                batch_id(),
                None,
                None,
                &[
                    copies(CardId::new("TST", "2", LanguageCode::EN), false, 1),
                    copies(card(), false, 1),
                ],
            )
            .await
            .unwrap_err();

        assert_eq!(error.code(), "invalid_quantity");
        assert_eq!(copies_held(&pool, USER).await, 255);
        assert_eq!(count(&pool, "scan_batch").await, 0);
    }

    #[sqlx::test]
    async fn a_batch_feeds_the_content_of_the_sealed_product_without_a_new_purchase(pool: PgPool) {
        seed(&pool).await;
        let sealed_product = insert_purchase(&pool, USER, "sealed_product").await;
        let adapter = adapter(pool.clone());

        adapter
            .add_scan_batch(
                &user(),
                batch_id(),
                None,
                Some(sealed_product),
                &[
                    copies(card(), false, 2),
                    copies(card(), false, 1),
                    copies(card(), true, 1),
                ],
            )
            .await
            .unwrap();

        let content: Vec<(bool, i32)> = sqlx::query_as(
            "SELECT foil, quantity FROM purchase_content WHERE purchase_id = $1 ORDER BY foil",
        )
        .bind(sealed_product.0)
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(content, vec![(false, 3), (true, 1)]);
        assert_eq!(count(&pool, "purchase").await, 1);
        assert_eq!(copies_held(&pool, USER).await, 4);
    }

    #[sqlx::test]
    async fn a_player_cannot_validate_into_the_binder_of_another(pool: PgPool) {
        seed(&pool).await;
        let theirs = BinderId(insert_binder(&pool, OTHER, "Bob's").await);
        let adapter = adapter(pool.clone());

        let error = adapter
            .add_scan_batch(
                &user(),
                batch_id(),
                Some(theirs),
                None,
                &[copies(card(), false, 1)],
            )
            .await
            .unwrap_err();

        assert_eq!(error.code(), "binder_not_found");
        assert_eq!(count(&pool, "collection_entry").await, 0);
        assert_eq!(count(&pool, "scan_batch").await, 0);
    }

    #[sqlx::test]
    async fn a_player_cannot_validate_into_a_purchase_that_is_not_their_sealed_product(
        pool: PgPool,
    ) {
        seed(&pool).await;
        let theirs = insert_purchase(&pool, OTHER, "sealed_product").await;
        let single_card = insert_purchase(&pool, USER, "single_card").await;
        let adapter = adapter(pool.clone());

        for id in [theirs, single_card, PurchaseId(Uuid::new_v4())] {
            let error = adapter
                .add_scan_batch(
                    &user(),
                    batch_id(),
                    None,
                    Some(id),
                    &[copies(card(), false, 1)],
                )
                .await
                .unwrap_err();

            assert_eq!(error.code(), "purchase_not_found");
        }
        assert_eq!(count(&pool, "collection_entry").await, 0);
        assert_eq!(count(&pool, "purchase_content").await, 0);
        assert_eq!(count(&pool, "scan_batch").await, 0);
    }

    // ---- find_by_card ----

    #[sqlx::test]
    async fn find_by_card_lists_both_finishes_with_their_binder(pool: PgPool) {
        seed(&pool).await;
        insert_entry(&pool, USER, false, 2, Some(500), day(2), Some("Trade")).await;
        insert_entry(&pool, USER, true, 1, None, day(3), None).await;

        let found = adapter(pool).find_by_card(&user(), &card()).await.unwrap();

        assert_eq!(found.entries.len(), 2);
        let normal = &found.entries[0];
        assert!(!normal.foil);
        assert_eq!(normal.binder.as_ref().unwrap().name, "Trade");
        assert_eq!(normal.quantity, 2);
        assert_eq!(normal.purchase_price, Some(500));
        assert_eq!(normal.added_at, day(2));
        let foil = &found.entries[1];
        assert!(foil.foil);
        assert_eq!(foil.binder, None);
        assert_eq!(foil.purchase_price, None);
    }

    #[sqlx::test]
    async fn find_by_card_reports_the_quantity_committed_per_finish(pool: PgPool) {
        seed(&pool).await;
        insert_entry(&pool, USER, false, 5, None, day(1), None).await;
        commit(&pool, "ONE_ACCEPTED", false, 2).await;
        commit(&pool, "FULLY_ACCEPTED", true, 1).await;

        let found = adapter(pool).find_by_card(&user(), &card()).await.unwrap();

        assert_eq!((found.reserved_normal, found.reserved_foil), (2, 1));
    }

    #[sqlx::test]
    async fn find_by_card_counts_only_accepted_trades_as_committed(pool: PgPool) {
        seed(&pool).await;
        insert_entry(&pool, USER, false, 5, None, day(1), None).await;
        for status in ["PENDING", "COMPLETED", "CLOSED", "ABANDONED"] {
            commit(&pool, status, false, 3).await;
        }

        let found = adapter(pool).find_by_card(&user(), &card()).await.unwrap();

        assert_eq!((found.reserved_normal, found.reserved_foil), (0, 0));
    }

    #[sqlx::test]
    async fn find_by_card_leaves_out_another_players_entries(pool: PgPool) {
        seed(&pool).await;
        insert_entry(&pool, OTHER, false, 4, None, day(1), None).await;

        let found = adapter(pool).find_by_card(&user(), &card()).await.unwrap();

        assert!(found.entries.is_empty());
    }

    // ---- add ----

    #[sqlx::test]
    async fn add_writes_a_new_entry_without_purchase_price(pool: PgPool) {
        seed(&pool).await;
        let binder = BinderId(insert_binder(&pool, USER, "Trade").await);

        let entries = adapter(pool)
            .add(&user(), &card(), true, 3, Some(binder), None)
            .await
            .unwrap();

        assert_eq!(entries.entries.len(), 1);
        let entry = &entries.entries[0];
        assert!(entry.foil);
        assert_eq!(entry.quantity, 3);
        assert_eq!(entry.purchase_price, None);
        assert_eq!(entry.binder.as_ref().map(|b| b.id), Some(binder));
    }

    #[sqlx::test]
    async fn add_without_binder_stores_the_entry_in_no_binder(pool: PgPool) {
        seed(&pool).await;

        let entries = adapter(pool)
            .add(&user(), &card(), false, 1, None, None)
            .await
            .unwrap();

        assert_eq!(entries.entries[0].binder, None);
    }

    #[sqlx::test]
    async fn add_merges_into_the_entry_of_the_same_finish_and_binder(pool: PgPool) {
        seed(&pool).await;
        insert_entry(&pool, USER, false, 2, Some(500), day(2), Some("Trade")).await;
        let binder = sqlx::query_scalar("SELECT id FROM binder WHERE name = 'Trade'")
            .fetch_one(&pool)
            .await
            .map(BinderId)
            .unwrap();

        let entries = adapter(pool)
            .add(&user(), &card(), false, 3, Some(binder), None)
            .await
            .unwrap();

        assert_eq!(entries.entries.len(), 1);
        let entry = &entries.entries[0];
        assert_eq!(entry.quantity, 5);
        // The added copies have no price: the average of the prices present is unchanged.
        assert_eq!(entry.purchase_price, Some(500));
        assert_eq!(entry.added_at, day(2));
    }

    #[sqlx::test]
    async fn add_merges_into_the_entry_stored_in_no_binder(pool: PgPool) {
        seed(&pool).await;
        insert_entry(&pool, USER, true, 1, None, day(2), None).await;

        let entries = adapter(pool)
            .add(&user(), &card(), true, 1, None, None)
            .await
            .unwrap();

        assert_eq!(entries.entries.len(), 1);
        assert_eq!(entries.entries[0].quantity, 2);
    }

    #[sqlx::test]
    async fn add_keeps_a_distinct_entry_per_finish_and_binder(pool: PgPool) {
        seed(&pool).await;
        insert_entry(&pool, USER, false, 2, None, day(2), Some("Trade")).await;

        let entries = adapter(pool)
            .add(&user(), &card(), true, 1, None, None)
            .await
            .unwrap();

        assert_eq!(entries.entries.len(), 2);
    }

    #[sqlx::test]
    async fn add_refuses_a_merge_over_255_copies(pool: PgPool) {
        seed(&pool).await;
        insert_entry(&pool, USER, false, 255, None, day(2), None).await;

        let error = adapter(pool)
            .add(&user(), &card(), false, 1, None, None)
            .await
            .unwrap_err();

        assert_eq!(error.code(), "invalid_quantity");
    }

    #[sqlx::test]
    async fn add_refuses_a_binder_of_another_player_and_writes_nothing(pool: PgPool) {
        seed(&pool).await;
        let others = BinderId(insert_binder(&pool, OTHER, "Bob's binder").await);

        let error = adapter(pool.clone())
            .add(&user(), &card(), false, 1, Some(others), None)
            .await
            .unwrap_err();

        assert_eq!(error.code(), "binder_not_found");
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM collection_entry")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }

    #[sqlx::test]
    async fn add_refuses_an_unknown_binder(pool: PgPool) {
        seed(&pool).await;

        let error = adapter(pool)
            .add(
                &user(),
                &card(),
                false,
                1,
                Some(BinderId(Uuid::new_v4())),
                None,
            )
            .await
            .unwrap_err();

        assert_eq!(error.code(), "binder_not_found");
    }

    #[sqlx::test]
    async fn add_leaves_another_players_entry_alone(pool: PgPool) {
        seed(&pool).await;
        insert_entry(&pool, OTHER, false, 4, None, day(1), None).await;

        let entries = adapter(pool)
            .add(&user(), &card(), false, 1, None, None)
            .await
            .unwrap();

        assert_eq!(entries.entries.len(), 1);
        assert_eq!(entries.entries[0].quantity, 1);
    }

    // ---- add: with a purchase ----

    fn single_card_purchase(price: i32, quantity: u8) -> NewSingleCardPurchase {
        NewSingleCardPurchase::try_new(
            PurchaseAmount::try_new(price).unwrap(),
            quantity,
            chrono::NaiveDate::from_ymd_opt(2026, 11, 13).unwrap(),
        )
        .unwrap()
    }

    #[sqlx::test]
    async fn add_with_a_purchase_gives_the_copies_their_price_and_records_the_purchase(
        pool: PgPool,
    ) {
        seed(&pool).await;

        let entries = adapter(pool.clone())
            .add(
                &user(),
                &card(),
                true,
                4,
                None,
                Some(single_card_purchase(200, 4)),
            )
            .await
            .unwrap();

        assert_eq!(entries.entries[0].purchase_price, Some(200));
        let purchases: Vec<(String, i32, chrono::NaiveDate)> =
            sqlx::query_as("SELECT kind, amount_cents, purchased_on FROM purchase")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(
            purchases,
            vec![(
                "single_card".to_string(),
                800,
                chrono::NaiveDate::from_ymd_opt(2026, 11, 13).unwrap()
            )]
        );
        let content: Vec<(String, String, String, bool, i32)> = sqlx::query_as(
            "SELECT set_code, collector_number, language_code, foil, quantity FROM purchase_content",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(
            content,
            vec![("TST".into(), "1".into(), "EN".into(), true, 4)]
        );
    }

    #[sqlx::test]
    async fn add_without_a_purchase_records_none(pool: PgPool) {
        seed(&pool).await;

        adapter(pool.clone())
            .add(&user(), &card(), false, 2, None, None)
            .await
            .unwrap();

        let purchases: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM purchase")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(purchases, 0);
    }

    #[sqlx::test]
    async fn add_with_a_purchase_merges_the_price_by_weighted_average(pool: PgPool) {
        seed(&pool).await;
        insert_entry(&pool, USER, false, 2, Some(500), day(2), None).await;

        let entries = adapter(pool)
            .add(
                &user(),
                &card(),
                false,
                2,
                None,
                Some(single_card_purchase(300, 2)),
            )
            .await
            .unwrap();

        assert_eq!(entries.entries.len(), 1);
        assert_eq!(entries.entries[0].quantity, 4);
        assert_eq!(entries.entries[0].purchase_price, Some(400));
    }

    #[sqlx::test]
    async fn a_refused_merge_records_no_purchase(pool: PgPool) {
        seed(&pool).await;
        insert_entry(&pool, USER, false, 255, None, day(2), None).await;

        let error = adapter(pool.clone())
            .add(
                &user(),
                &card(),
                false,
                1,
                None,
                Some(single_card_purchase(200, 1)),
            )
            .await
            .unwrap_err();

        assert_eq!(error.code(), "invalid_quantity");
        let purchases: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM purchase")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(purchases, 0);
    }

    #[sqlx::test]
    async fn a_purchase_that_cannot_be_written_takes_the_entry_with_it(pool: PgPool) {
        seed(&pool).await;
        sqlx::query(
            r#"CREATE FUNCTION refuse_purchase() RETURNS trigger AS
               $$ BEGIN RAISE EXCEPTION 'purchase refused'; END $$ LANGUAGE plpgsql"#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "CREATE TRIGGER refuse_purchase BEFORE INSERT ON purchase
             FOR EACH ROW EXECUTE FUNCTION refuse_purchase()",
        )
        .execute(&pool)
        .await
        .unwrap();

        let result = adapter(pool.clone())
            .add(
                &user(),
                &card(),
                false,
                1,
                None,
                Some(single_card_purchase(200, 1)),
            )
            .await;

        assert!(result.is_err());
        let entries: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM collection_entry")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(entries, 0);
    }

    // ---- update: quantity ----

    #[sqlx::test]
    async fn update_changes_the_quantity_and_returns_the_entries(pool: PgPool) {
        seed(&pool).await;
        let id = insert_entry(&pool, USER, false, 2, Some(500), day(1), None).await;

        let entries = adapter(pool)
            .update(&user(), id, quantity_update(7))
            .await
            .unwrap();

        assert_eq!(entries.entries.len(), 1);
        assert_eq!(entries.entries[0].quantity, 7);
        assert_eq!(entries.entries[0].purchase_price, Some(500));
    }

    #[sqlx::test]
    async fn update_refuses_a_decrease_under_the_committed_quantity(pool: PgPool) {
        seed(&pool).await;
        let id = insert_entry(&pool, USER, false, 4, None, day(1), None).await;
        commit(&pool, "ONE_ACCEPTED", false, 3).await;
        let adapter = adapter(pool);

        let error = adapter
            .update(&user(), id, quantity_update(2))
            .await
            .unwrap_err();
        let kept = adapter
            .update(&user(), id, quantity_update(3))
            .await
            .unwrap();

        assert_eq!(error.code(), "reserved_quantity_exceeded");
        assert_eq!(kept.entries[0].quantity, 3);
    }

    #[sqlx::test]
    async fn update_counts_the_copies_of_every_binder_against_the_committed_quantity(pool: PgPool) {
        seed(&pool).await;
        let a = insert_entry(&pool, USER, false, 2, None, day(1), Some("A")).await;
        let b = insert_entry(&pool, USER, false, 2, None, day(1), Some("B")).await;
        commit(&pool, "FULLY_ACCEPTED", false, 3).await;
        let adapter = adapter(pool);

        // 4 owned in total, 3 committed: one copy may go, not two.
        let first = adapter.update(&user(), a, quantity_update(1)).await;
        let second = adapter.update(&user(), b, quantity_update(1)).await;

        assert!(first.is_ok());
        assert_eq!(second.unwrap_err().code(), "reserved_quantity_exceeded");
    }

    #[sqlx::test]
    async fn update_ignores_a_trade_that_is_not_accepted(pool: PgPool) {
        seed(&pool).await;
        let id = insert_entry(&pool, USER, false, 4, None, day(1), None).await;
        for status in ["PENDING", "COMPLETED"] {
            commit(&pool, status, false, 4).await;
        }

        let entries = adapter(pool)
            .update(&user(), id, quantity_update(1))
            .await
            .unwrap();

        assert_eq!(entries.entries[0].quantity, 1);
    }

    #[sqlx::test]
    async fn update_allows_an_increase_even_when_already_under_the_committed_quantity(
        pool: PgPool,
    ) {
        seed(&pool).await;
        let id = insert_entry(&pool, USER, false, 1, None, day(1), None).await;
        commit(&pool, "ONE_ACCEPTED", false, 3).await;

        let entries = adapter(pool)
            .update(&user(), id, quantity_update(2))
            .await
            .unwrap();

        assert_eq!(entries.entries[0].quantity, 2);
    }

    // ---- update: finish ----

    #[sqlx::test]
    async fn update_turns_an_entry_into_the_other_finish(pool: PgPool) {
        seed(&pool).await;
        let id = insert_entry(&pool, USER, false, 2, Some(500), day(1), Some("A")).await;

        let entries = adapter(pool)
            .update(&user(), id, foil_update(true))
            .await
            .unwrap();

        assert_eq!(entries.entries.len(), 1);
        let entry = &entries.entries[0];
        assert_eq!(entry.id, id);
        assert!(entry.foil);
        assert_eq!(entry.quantity, 2);
        assert_eq!(entry.binder.as_ref().unwrap().name, "A");
    }

    #[sqlx::test]
    async fn update_merges_a_finish_change_into_the_existing_entry(pool: PgPool) {
        seed(&pool).await;
        let normal = insert_entry(&pool, USER, false, 2, Some(1000), day(20), Some("A")).await;
        insert_entry(&pool, USER, true, 1, Some(400), day(3), Some("A")).await;

        let entries = adapter(pool)
            .update(&user(), normal, foil_update(true))
            .await
            .unwrap();

        assert_eq!(entries.entries.len(), 1);
        let merged = &entries.entries[0];
        assert!(merged.foil);
        assert_eq!(merged.quantity, 3);
        // (2 * 1000 + 1 * 400) / 3
        assert_eq!(merged.purchase_price, Some(800));
        assert_eq!(merged.added_at, day(3));
    }

    #[sqlx::test]
    async fn update_merge_averages_only_the_prices_present(pool: PgPool) {
        seed(&pool).await;
        let normal = insert_entry(&pool, USER, false, 5, None, day(1), None).await;
        insert_entry(&pool, USER, true, 1, Some(1000), day(1), None).await;

        let merged = adapter(pool)
            .update(&user(), normal, foil_update(true))
            .await
            .unwrap();

        assert_eq!(merged.entries[0].quantity, 6);
        assert_eq!(merged.entries[0].purchase_price, Some(1000));
    }

    #[sqlx::test]
    async fn update_merge_has_no_price_when_neither_entry_has_one(pool: PgPool) {
        seed(&pool).await;
        let normal = insert_entry(&pool, USER, false, 1, None, day(1), None).await;
        insert_entry(&pool, USER, true, 1, None, day(1), None).await;

        let merged = adapter(pool)
            .update(&user(), normal, foil_update(true))
            .await
            .unwrap();

        assert_eq!(merged.entries[0].purchase_price, None);
    }

    #[sqlx::test]
    async fn update_replaces_the_purchase_price_and_nothing_else(pool: PgPool) {
        seed(&pool).await;
        let normal = insert_entry(&pool, USER, false, 3, Some(500), day(1), None).await;

        let entries = adapter(pool)
            .update(
                &user(),
                normal,
                EntryUpdate {
                    purchase_price: Some(120),
                    ..EntryUpdate::default()
                },
            )
            .await
            .unwrap();

        assert_eq!(entries.entries.len(), 1);
        assert_eq!(entries.entries[0].quantity, 3);
        assert_eq!(entries.entries[0].purchase_price, Some(120));
    }

    #[sqlx::test]
    async fn update_sets_a_purchase_price_on_an_entry_without_one(pool: PgPool) {
        seed(&pool).await;
        let normal = insert_entry(&pool, USER, false, 2, None, day(1), None).await;

        let entries = adapter(pool)
            .update(
                &user(),
                normal,
                EntryUpdate {
                    purchase_price: Some(75),
                    ..EntryUpdate::default()
                },
            )
            .await
            .unwrap();

        assert_eq!(entries.entries[0].purchase_price, Some(75));
    }

    #[sqlx::test]
    async fn update_applies_a_new_quantity_together_with_the_finish_change(pool: PgPool) {
        seed(&pool).await;
        let normal = insert_entry(&pool, USER, false, 4, None, day(1), None).await;
        insert_entry(&pool, USER, true, 1, None, day(1), None).await;

        let entries = adapter(pool)
            .update(
                &user(),
                normal,
                EntryUpdate {
                    quantity: Some(2),
                    foil: Some(true),
                    ..EntryUpdate::default()
                },
            )
            .await
            .unwrap();

        assert_eq!(entries.entries.len(), 1);
        assert_eq!(entries.entries[0].quantity, 3);
    }

    #[sqlx::test]
    async fn update_does_not_merge_entries_of_different_binders(pool: PgPool) {
        seed(&pool).await;
        let normal = insert_entry(&pool, USER, false, 1, None, day(1), Some("A")).await;
        insert_entry(&pool, USER, true, 1, None, day(1), Some("B")).await;

        let entries = adapter(pool)
            .update(&user(), normal, foil_update(true))
            .await
            .unwrap();

        assert_eq!(entries.entries.len(), 2);
    }

    #[sqlx::test]
    async fn update_refuses_a_merge_over_255_copies(pool: PgPool) {
        seed(&pool).await;
        let normal = insert_entry(&pool, USER, false, 200, None, day(1), None).await;
        insert_entry(&pool, USER, true, 100, None, day(1), None).await;
        let adapter = adapter(pool);

        let error = adapter
            .update(&user(), normal, foil_update(true))
            .await
            .unwrap_err();
        let untouched = adapter.find_by_card(&user(), &card()).await.unwrap();

        assert_eq!(error.code(), "invalid_quantity");
        assert_eq!(untouched.entries.len(), 2);
    }

    #[sqlx::test]
    async fn update_refuses_a_finish_change_that_frees_committed_copies(pool: PgPool) {
        seed(&pool).await;
        let normal = insert_entry(&pool, USER, false, 2, None, day(1), None).await;
        commit(&pool, "FULLY_ACCEPTED", false, 2).await;
        let adapter = adapter(pool);

        let error = adapter
            .update(&user(), normal, foil_update(true))
            .await
            .unwrap_err();
        let untouched = adapter.find_by_card(&user(), &card()).await.unwrap();

        assert_eq!(error.code(), "reserved_quantity_exceeded");
        assert!(!untouched.entries[0].foil);
    }

    #[sqlx::test]
    async fn update_allows_a_finish_change_the_other_binders_copies_cover(pool: PgPool) {
        seed(&pool).await;
        let a = insert_entry(&pool, USER, false, 1, None, day(1), Some("A")).await;
        insert_entry(&pool, USER, false, 2, None, day(1), Some("B")).await;
        commit(&pool, "ONE_ACCEPTED", false, 2).await;

        let entries = adapter(pool)
            .update(&user(), a, foil_update(true))
            .await
            .unwrap();

        assert_eq!(entries.entries.len(), 2);
    }

    // ---- isolation, absence ----

    #[sqlx::test]
    async fn update_and_remove_treat_another_players_entry_as_absent(pool: PgPool) {
        seed(&pool).await;
        let others = insert_entry(&pool, OTHER, false, 2, None, day(1), None).await;
        let adapter = adapter(pool.clone());

        let update = adapter
            .update(&user(), others, quantity_update(1))
            .await
            .unwrap_err();
        let removal = adapter.remove(&user(), others).await.unwrap_err();

        assert_eq!(update.code(), "collection_entry_not_found");
        assert_eq!(removal.code(), "collection_entry_not_found");
        let kept = adapter
            .find_by_card(&UserId::new(OTHER), &card())
            .await
            .unwrap();
        assert_eq!(kept.entries[0].quantity, 2);
    }

    #[sqlx::test]
    async fn update_and_remove_refuse_an_unknown_entry(pool: PgPool) {
        seed(&pool).await;
        let unknown = CollectionEntryId(Uuid::new_v4());
        let adapter = adapter(pool);

        let update = adapter
            .update(&user(), unknown, quantity_update(1))
            .await
            .unwrap_err();
        let removal = adapter.remove(&user(), unknown).await.unwrap_err();

        assert_eq!(update.code(), "collection_entry_not_found");
        assert_eq!(removal.code(), "collection_entry_not_found");
    }

    // ---- remove ----

    #[sqlx::test]
    async fn remove_removes_only_the_entry(pool: PgPool) {
        seed(&pool).await;
        let id = insert_entry(&pool, USER, false, 2, None, day(1), Some("A")).await;
        insert_entry(&pool, USER, false, 1, None, day(1), Some("B")).await;
        let adapter = adapter(pool);

        adapter.remove(&user(), id).await.unwrap();

        let left = adapter.find_by_card(&user(), &card()).await.unwrap();
        assert_eq!(left.entries.len(), 1);
        assert_eq!(left.entries[0].binder.as_ref().unwrap().name, "B");
    }

    #[sqlx::test]
    async fn remove_refuses_a_removal_under_the_committed_quantity(pool: PgPool) {
        seed(&pool).await;
        let id = insert_entry(&pool, USER, true, 2, None, day(1), None).await;
        commit(&pool, "FULLY_ACCEPTED", true, 1).await;
        let adapter = adapter(pool);

        let error = adapter.remove(&user(), id).await.unwrap_err();

        assert_eq!(error.code(), "reserved_quantity_exceeded");
        assert_eq!(
            adapter
                .find_by_card(&user(), &card())
                .await
                .unwrap()
                .entries
                .len(),
            1
        );
    }

    #[sqlx::test]
    async fn remove_allows_a_removal_the_other_finish_commitment_does_not_touch(pool: PgPool) {
        seed(&pool).await;
        let normal = insert_entry(&pool, USER, false, 2, None, day(1), None).await;
        insert_entry(&pool, USER, true, 1, None, day(1), None).await;
        commit(&pool, "FULLY_ACCEPTED", true, 1).await;

        let result = adapter(pool).remove(&user(), normal).await;

        assert!(result.is_ok());
    }

    // ---- move_to_binder ----

    #[sqlx::test]
    async fn move_all_into_an_empty_binder_keeps_the_entry(pool: PgPool) {
        seed(&pool).await;
        let id = insert_entry(&pool, USER, false, 3, Some(500), day(2), None).await;
        let target = insert_binder(&pool, USER, "Trade").await;

        let entries = adapter(pool)
            .move_to_binder(&user(), id, Some(BinderId(target)), None)
            .await
            .unwrap();

        assert_eq!(entries.entries.len(), 1);
        let entry = &entries.entries[0];
        assert_eq!(entry.id, id);
        assert_eq!(entry.binder.as_ref().unwrap().name, "Trade");
        assert_eq!(entry.quantity, 3);
        assert_eq!(entry.purchase_price, Some(500));
    }

    #[sqlx::test]
    async fn move_some_splits_the_entry_and_keeps_price_and_date(pool: PgPool) {
        seed(&pool).await;
        let id = insert_entry(&pool, USER, true, 4, Some(500), day(2), Some("A")).await;
        let target = insert_binder(&pool, USER, "B").await;

        let entries = adapter(pool)
            .move_to_binder(&user(), id, Some(BinderId(target)), Some(1))
            .await
            .unwrap();

        assert_eq!(entries.entries.len(), 2);
        let source = entries.entries.iter().find(|e| e.id == id).unwrap();
        assert_eq!(source.quantity, 3);
        let moved = entries.entries.iter().find(|e| e.id != id).unwrap();
        assert_eq!(moved.binder.as_ref().unwrap().name, "B");
        assert_eq!(moved.quantity, 1);
        assert!(moved.foil);
        assert_eq!(moved.purchase_price, Some(500));
        assert_eq!(moved.added_at, day(2));
    }

    #[sqlx::test]
    async fn move_out_of_any_binder(pool: PgPool) {
        seed(&pool).await;
        let id = insert_entry(&pool, USER, false, 2, None, day(1), Some("A")).await;

        let entries = adapter(pool)
            .move_to_binder(&user(), id, None, Some(1))
            .await
            .unwrap();

        assert_eq!(entries.entries.len(), 2);
        assert!(
            entries
                .entries
                .iter()
                .any(|e| e.binder.is_none() && e.quantity == 1)
        );
        assert!(
            entries
                .entries
                .iter()
                .any(|e| e.binder.is_some() && e.quantity == 1)
        );
    }

    #[sqlx::test]
    async fn move_all_merges_into_the_target_and_removes_the_source(pool: PgPool) {
        seed(&pool).await;
        let source = insert_entry(&pool, USER, false, 2, Some(1000), day(5), Some("A")).await;
        let target = insert_entry(&pool, USER, false, 1, Some(400), day(2), Some("B")).await;
        let binder = insert_binder(&pool, USER, "B").await;

        let entries = adapter(pool)
            .move_to_binder(&user(), source, Some(BinderId(binder)), None)
            .await
            .unwrap();

        assert_eq!(entries.entries.len(), 1);
        let merged = &entries.entries[0];
        assert_eq!(merged.id, target);
        assert_eq!(merged.quantity, 3);
        // (2 * 1000 + 1 * 400) / 3
        assert_eq!(merged.purchase_price, Some(800));
        assert_eq!(merged.added_at, day(2));
    }

    #[sqlx::test]
    async fn move_some_merges_into_the_target_and_leaves_the_rest(pool: PgPool) {
        seed(&pool).await;
        let source = insert_entry(&pool, USER, false, 4, None, day(1), Some("A")).await;
        let target = insert_entry(&pool, USER, false, 1, None, day(1), None).await;

        let entries = adapter(pool)
            .move_to_binder(&user(), source, None, Some(3))
            .await
            .unwrap();

        assert_eq!(entries.entries.len(), 2);
        assert_eq!(
            entries
                .entries
                .iter()
                .find(|e| e.id == source)
                .unwrap()
                .quantity,
            1
        );
        assert_eq!(
            entries
                .entries
                .iter()
                .find(|e| e.id == target)
                .unwrap()
                .quantity,
            4
        );
    }

    #[sqlx::test]
    async fn move_does_not_merge_across_finishes(pool: PgPool) {
        seed(&pool).await;
        let source = insert_entry(&pool, USER, false, 1, None, day(1), Some("A")).await;
        insert_entry(&pool, USER, true, 1, None, day(1), None).await;

        let entries = adapter(pool)
            .move_to_binder(&user(), source, None, None)
            .await
            .unwrap();

        assert_eq!(entries.entries.len(), 2);
    }

    #[sqlx::test]
    async fn move_into_the_same_binder_changes_nothing(pool: PgPool) {
        seed(&pool).await;
        let id = insert_entry(&pool, USER, false, 2, None, day(1), Some("A")).await;
        let same = insert_binder(&pool, USER, "A").await;

        let entries = adapter(pool)
            .move_to_binder(&user(), id, Some(BinderId(same)), Some(1))
            .await
            .unwrap();

        assert_eq!(entries.entries.len(), 1);
        assert_eq!(entries.entries[0].quantity, 2);
    }

    #[sqlx::test]
    async fn move_is_accepted_for_a_reserved_card(pool: PgPool) {
        seed(&pool).await;
        let id = insert_entry(&pool, USER, false, 2, None, day(1), Some("A")).await;
        commit(&pool, "FULLY_ACCEPTED", false, 2).await;

        let entries = adapter(pool)
            .move_to_binder(&user(), id, None, None)
            .await
            .unwrap();

        assert_eq!(entries.entries[0].binder, None);
        assert_eq!(entries.reserved_normal, 2);
    }

    #[sqlx::test]
    async fn move_refuses_a_quantity_over_the_entrys(pool: PgPool) {
        seed(&pool).await;
        let id = insert_entry(&pool, USER, false, 2, None, day(1), Some("A")).await;

        let error = adapter(pool)
            .move_to_binder(&user(), id, None, Some(3))
            .await
            .unwrap_err();

        assert_eq!(error.code(), "invalid_quantity");
    }

    #[sqlx::test]
    async fn move_refuses_a_merge_over_the_bound(pool: PgPool) {
        seed(&pool).await;
        let source = insert_entry(&pool, USER, false, 200, None, day(1), Some("A")).await;
        insert_entry(&pool, USER, false, 56, None, day(1), None).await;
        let adapter = adapter(pool);

        let error = adapter
            .move_to_binder(&user(), source, None, None)
            .await
            .unwrap_err();

        assert_eq!(error.code(), "invalid_quantity");
        assert_eq!(
            adapter
                .find_by_card(&user(), &card())
                .await
                .unwrap()
                .entries
                .len(),
            2
        );
    }

    #[sqlx::test]
    async fn move_refuses_an_unknown_entry_and_another_players(pool: PgPool) {
        seed(&pool).await;
        let foreign = insert_entry(&pool, OTHER, false, 1, None, day(1), None).await;
        let adapter = adapter(pool);

        for id in [foreign, CollectionEntryId(Uuid::new_v4())] {
            let error = adapter
                .move_to_binder(&user(), id, None, None)
                .await
                .unwrap_err();
            assert_eq!(error.code(), "collection_entry_not_found");
        }
    }

    #[sqlx::test]
    async fn move_refuses_an_unknown_binder_and_another_players(pool: PgPool) {
        seed(&pool).await;
        let id = insert_entry(&pool, USER, false, 1, None, day(1), None).await;
        let foreign = insert_binder(&pool, OTHER, "Bob's").await;
        let adapter = adapter(pool);

        for binder in [foreign, Uuid::new_v4()] {
            let error = adapter
                .move_to_binder(&user(), id, Some(BinderId(binder)), None)
                .await
                .unwrap_err();
            assert_eq!(error.code(), "binder_not_found");
        }
    }
}
