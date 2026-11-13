//! The SQL the Purchase and the collection entry adapters share to feed the content of a sealed
//! product.

use crate::application::error::AppError;
use crate::domain::card::CardId;
use crate::domain::error::FunctionalError;
use crate::domain::purchase::PurchaseId;
use crate::domain::user::UserId;
use sqlx::PgConnection;

/// `PurchaseNotFound` unless `id` is a sealed product of the player. The Purchase is locked, so a
/// deletion of it waits for the content lines of the caller's transaction.
pub(super) async fn lock_sealed_product(
    conn: &mut PgConnection,
    user_id: &UserId,
    id: PurchaseId,
) -> Result<(), AppError> {
    let sealed_product = sqlx::query_scalar!(
        r#"
        SELECT id FROM purchase
        WHERE id = $1 AND user_id = $2 AND kind = 'sealed_product'
        FOR SHARE
        "#,
        id.0,
        user_id.as_str()
    )
    .fetch_optional(conn)
    .await?;
    if sealed_product.is_none() {
        return Err(FunctionalError::PurchaseNotFound.into());
    }
    Ok(())
}

/// Like [`lock_sealed_product`], without the lock: a read for a caller that writes nothing.
pub(super) async fn ensure_sealed_product(
    conn: &mut PgConnection,
    user_id: &UserId,
    id: PurchaseId,
) -> Result<(), AppError> {
    let owned = sqlx::query_scalar!(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM purchase
            WHERE id = $1 AND user_id = $2 AND kind = 'sealed_product'
        ) AS "owned!"
        "#,
        id.0,
        user_id.as_str()
    )
    .fetch_one(conn)
    .await?;
    if owned {
        Ok(())
    } else {
        Err(FunctionalError::PurchaseNotFound.into())
    }
}

/// Adds the card, its finish and the quantity to the content of the sealed product `id`: a line
/// already there gets the quantity added.
pub(super) async fn add_content_line(
    conn: &mut PgConnection,
    id: PurchaseId,
    card_id: &CardId,
    foil: bool,
    quantity: u8,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"
        INSERT INTO purchase_content
            (purchase_id, set_code, collector_number, language_code, foil, quantity)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (purchase_id, set_code, collector_number, language_code, foil)
            DO UPDATE SET quantity = purchase_content.quantity + EXCLUDED.quantity
        "#,
        id.0,
        card_id.set_code.to_string(),
        card_id.collector_number,
        card_id.language_code.to_string(),
        foil,
        i32::from(quantity)
    )
    .execute(conn)
    .await?;
    Ok(())
}
