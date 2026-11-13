use crate::application::error::{AppError, InfraError};
use crate::application::repository::PurchaseRepository;
use crate::domain::binder::BinderId;
use crate::domain::card::CardId;
use crate::domain::collection_entry::CardEntries;
use crate::domain::error::FunctionalError;
use crate::domain::pagination::{Paginated, Pagination};
use crate::domain::purchase::{
    NewSealedProduct, Purchase, PurchaseAmount, PurchaseChanges, PurchaseContentLine, PurchaseId,
    PurchaseKind, PurchasedCard, SealedProductType,
};
use crate::domain::set_name::{SetCode, SetName};
use crate::domain::user::UserId;
use crate::infrastructure::adapter_out::repository::collection_entry_repository_adapter::add_copies;
use crate::infrastructure::adapter_out::repository::entities::{
    card_id_from_db, card_image_from_db,
};
use crate::infrastructure::adapter_out::repository::sealed_product_content::{
    add_content_line, lock_sealed_product,
};
use async_trait::async_trait;
use chrono::NaiveDate;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

/// Postgres error code for `foreign_key_violation`.
const FOREIGN_KEY_VIOLATION: &str = "23503";

struct PurchaseRow {
    id: Uuid,
    kind: String,
    amount_cents: i32,
    purchased_on: NaiveDate,
    product_type: Option<String>,
    name: Option<String>,
    set_code: Option<String>,
    set_name: Option<String>,
    // Single card only: the one line of content and its card.
    card_set_code: Option<String>,
    card_set_name: Option<String>,
    collector_number: Option<String>,
    language_code: Option<String>,
    card_name: Option<String>,
    image_source: Option<String>,
    image_has_back: Option<bool>,
    foil: Option<bool>,
    quantity: Option<i32>,
}

fn corrupt(what: &str) -> AppError {
    InfraError::RepositoryError(format!("invalid purchase from database: {what}")).into()
}

impl TryFrom<PurchaseRow> for Purchase {
    type Error = AppError;

    fn try_from(row: PurchaseRow) -> Result<Self, AppError> {
        let kind = match (
            row.kind.as_str(),
            row.product_type,
            row.set_code,
            row.set_name,
        ) {
            ("sealed_product", Some(product_type), Some(set_code), Some(set_name)) => {
                PurchaseKind::SealedProduct {
                    product_type: SealedProductType::from_str_opt(&product_type)
                        .ok_or_else(|| corrupt(&product_type))?,
                    name: row.name,
                    set: SetName {
                        code: SetCode::try_new(set_code).map_err(|_| corrupt("set code"))?,
                        name: set_name,
                    },
                }
            }
            ("single_card", None, None, None) => match (
                row.card_set_code,
                row.card_set_name,
                row.collector_number,
                row.language_code,
                row.card_name,
                row.foil,
                row.quantity,
            ) {
                (
                    Some(set_code),
                    Some(set_name),
                    Some(collector_number),
                    Some(language_code),
                    Some(card_name),
                    Some(foil),
                    Some(quantity),
                ) => {
                    let image = card_image_from_db(
                        row.image_source.as_deref(),
                        row.image_has_back.unwrap_or(false),
                    )?;
                    let id = card_id_from_db(&set_code, collector_number, &language_code)?;
                    PurchaseKind::SingleCard {
                        card: PurchasedCard {
                            set: SetName {
                                code: id.set_code.clone(),
                                name: set_name,
                            },
                            id,
                            name: card_name,
                            image,
                        },
                        foil,
                        quantity: u32::try_from(quantity).map_err(|_| corrupt("quantity"))?,
                    }
                }
                _ => return Err(corrupt("single_card without its content")),
            },
            (kind, ..) => return Err(corrupt(kind)),
        };

        Ok(Purchase {
            id: PurchaseId(row.id),
            kind,
            amount: PurchaseAmount::try_new(row.amount_cents).map_err(|_| corrupt("amount"))?,
            purchased_on: row.purchased_on,
        })
    }
}

/// Reads a write on `purchase.set_code` refused by its foreign key as an unknown set.
fn set_not_found_on_foreign_key_violation(err: sqlx::Error) -> AppError {
    match err {
        sqlx::Error::Database(db_err)
            if db_err.code().as_deref() == Some(FOREIGN_KEY_VIOLATION) =>
        {
            FunctionalError::SetNotFound.into()
        }
        other => other.into(),
    }
}

/// Reads Purchases as `PurchaseRow`s, with their set and, for a single card, their one line of
/// content and its card. `$rest` is the end of the query, from its `WHERE`; the macro takes the
/// arguments after it. A macro because SQLx checks a query written as literals at compile time.
macro_rules! select_purchases {
    ($rest:literal, $($arg:expr),+) => {
        sqlx::query_as!(
            PurchaseRow,
            r#"
            SELECT p.id, p.kind, p.amount_cents, p.purchased_on, p.product_type, p.name,
                   p.set_code, sn.name AS "set_name?",
                   pc.set_code AS "card_set_code?", csn.name AS "card_set_name?",
                   pc.collector_number AS "collector_number?", pc.language_code AS "language_code?",
                   c.name AS "card_name?", c.image_source AS "image_source?",
                   c.image_has_back AS "image_has_back?", pc.foil AS "foil?",
                   pc.quantity AS "quantity?"
            FROM purchase p
                     LEFT JOIN set_name sn ON sn.set_code = p.set_code
                     LEFT JOIN purchase_content pc ON pc.purchase_id = p.id AND p.kind = 'single_card'
                     LEFT JOIN card c ON (c.set_code, c.collector_number, c.language_code) =
                                         (pc.set_code, pc.collector_number, pc.language_code)
                     LEFT JOIN set_name csn ON csn.set_code = pc.set_code
            "# + $rest,
            $($arg),+
        )
    };
}

pub struct PurchaseRepositoryAdapter {
    pool: Pool<Postgres>,
}

impl PurchaseRepositoryAdapter {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PurchaseRepository for PurchaseRepositoryAdapter {
    #[tracing::instrument(name = "purchase_repo.list", skip_all, fields(sentry.op = "db"))]
    async fn list(
        &self,
        user_id: &UserId,
        pagination: Pagination,
    ) -> Result<Paginated<Purchase>, AppError> {
        let rows = select_purchases!(
            r#"
            WHERE p.user_id = $1
            ORDER BY p.purchased_on DESC, p.created_at DESC, p.id DESC
            LIMIT $2 OFFSET $3
            "#,
            user_id.as_str(),
            i64::from(pagination.limit()),
            i64::from(pagination.offset())
        )
        .fetch_all(&self.pool)
        .await?;

        let total = sqlx::query_scalar!(
            r#"SELECT COUNT(*) AS "total!" FROM purchase WHERE user_id = $1"#,
            user_id.as_str()
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(Paginated {
            items: rows
                .into_iter()
                .map(Purchase::try_from)
                .collect::<Result<_, _>>()?,
            total: total.max(0) as u64,
            pagination,
        })
    }

    #[tracing::instrument(name = "purchase_repo.find", skip_all, fields(sentry.op = "db"))]
    async fn find(&self, user_id: &UserId, id: PurchaseId) -> Result<Purchase, AppError> {
        select_purchases!(
            r#"
            WHERE p.id = $1
              AND p.user_id = $2
            "#,
            id.0,
            user_id.as_str()
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or(FunctionalError::PurchaseNotFound)?
        .try_into()
    }

    #[tracing::instrument(name = "purchase_repo.content", skip_all, fields(sentry.op = "db"))]
    async fn content(
        &self,
        user_id: &UserId,
        id: PurchaseId,
    ) -> Result<Vec<PurchaseContentLine>, AppError> {
        // Ownership first: another player's Purchase is not found, not empty.
        self.find(user_id, id).await?;

        // The trend comes from the last Cardmarket prices (ADR-0013) of the finish: they do not
        // depend on the language. Read here, never stored with the Purchase.
        let rows = sqlx::query!(
            r#"
            SELECT pc.set_code, pc.collector_number, pc.language_code, pc.foil, pc.quantity,
                   c.name, c.image_source AS "image_source?", c.image_has_back,
                   lp.trend AS "trend?"
            FROM purchase_content pc
                     JOIN card c ON (c.set_code, c.collector_number, c.language_code) =
                                    (pc.set_code, pc.collector_number, pc.language_code)
                     LEFT JOIN mv_last_cardmarket_prices lp
                               ON (lp.set_code, lp.collector_number, lp.foil) =
                                  (pc.set_code, pc.collector_number, pc.foil)
            WHERE pc.purchase_id = $1
            ORDER BY c.name, pc.set_code, pc.collector_number, pc.language_code, pc.foil
            "#,
            id.0
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(PurchaseContentLine {
                    card_id: card_id_from_db(
                        &row.set_code,
                        row.collector_number,
                        &row.language_code,
                    )
                    .map_err(AppError::from)?,
                    name: row.name,
                    image: card_image_from_db(row.image_source.as_deref(), row.image_has_back)
                        .map_err(AppError::from)?,
                    foil: row.foil,
                    quantity: u32::try_from(row.quantity).map_err(|_| corrupt("quantity"))?,
                    trend_cents: row
                        .trend
                        .map(|trend| u32::try_from(trend).map_err(|_| corrupt("trend")))
                        .transpose()?,
                })
            })
            .collect()
    }

    #[tracing::instrument(
        name = "purchase_repo.add_card_to_sealed_product",
        skip_all,
        fields(sentry.op = "db")
    )]
    async fn add_card_to_sealed_product(
        &self,
        user_id: &UserId,
        id: PurchaseId,
        card_id: &CardId,
        foil: bool,
        quantity: u8,
        binder_id: Option<BinderId>,
    ) -> Result<CardEntries, AppError> {
        let mut tx = self.pool.begin().await?;

        lock_sealed_product(&mut tx, user_id, id).await?;
        let entries =
            add_copies(&mut tx, user_id, card_id, foil, quantity, binder_id, None).await?;
        add_content_line(&mut tx, id, card_id, foil, quantity).await?;

        tx.commit().await?;
        Ok(entries)
    }

    #[tracing::instrument(
        name = "purchase_repo.create_sealed_product",
        skip_all,
        fields(sentry.op = "db")
    )]
    async fn create_sealed_product(
        &self,
        user_id: &UserId,
        product: NewSealedProduct,
    ) -> Result<Purchase, AppError> {
        let id = sqlx::query_scalar!(
            r#"
            INSERT INTO purchase (user_id, kind, amount_cents, purchased_on, product_type, set_code, name)
            VALUES ($1, 'sealed_product', $2, $3, $4, $5, $6)
            RETURNING id
            "#,
            user_id.as_str(),
            product.amount.cents() as i32,
            product.purchased_on,
            product.product_type.as_str(),
            product.set_code.to_string(),
            product.name
        )
        .fetch_one(&self.pool)
        .await
        .map_err(set_not_found_on_foreign_key_violation)?;

        self.find(user_id, PurchaseId(id)).await
    }

    #[tracing::instrument(name = "purchase_repo.update", skip_all, fields(sentry.op = "db"))]
    async fn update(
        &self,
        user_id: &UserId,
        id: PurchaseId,
        changes: PurchaseChanges,
    ) -> Result<Purchase, AppError> {
        let (name_changes, name) = match &changes.name {
            Some(name) => (true, name.as_deref()),
            None => (false, None),
        };

        let updated = sqlx::query!(
            r#"
            UPDATE purchase
            SET amount_cents = COALESCE($3, amount_cents),
                purchased_on = COALESCE($4, purchased_on),
                product_type = COALESCE($5, product_type),
                set_code     = COALESCE($6, set_code),
                name         = CASE WHEN $7 THEN $8 ELSE name END
            WHERE id = $1
              AND user_id = $2
            "#,
            id.0,
            user_id.as_str(),
            changes.amount.map(|amount| amount.cents() as i32),
            changes.purchased_on,
            changes.product_type.map(SealedProductType::as_str),
            changes.set_code.map(|code| code.to_string()),
            name_changes,
            name
        )
        .execute(&self.pool)
        .await
        .map_err(set_not_found_on_foreign_key_violation)?;
        if updated.rows_affected() == 0 {
            return Err(FunctionalError::PurchaseNotFound.into());
        }

        self.find(user_id, id).await
    }

    #[tracing::instrument(name = "purchase_repo.delete", skip_all, fields(sentry.op = "db"))]
    async fn delete(&self, user_id: &UserId, id: PurchaseId) -> Result<(), AppError> {
        // Its content goes with it (`ON DELETE CASCADE`), no collection entry does.
        let deleted = sqlx::query!(
            r#"DELETE FROM purchase WHERE id = $1 AND user_id = $2"#,
            id.0,
            user_id.as_str()
        )
        .execute(&self.pool)
        .await?;
        if deleted.rows_affected() == 0 {
            return Err(FunctionalError::PurchaseNotFound.into());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::imported_card::ImportedCard;
    use crate::application::repository::{
        CardRepository, CollectionEntryRepository, TradeRepository,
    };
    use crate::domain::card::Card;
    use crate::domain::card_image::{CardImage, CardImageSource};
    use crate::domain::collection_entry::EntryUpdate;
    use crate::domain::language_code::LanguageCode;
    use crate::domain::purchase::NewSingleCardPurchase;
    use crate::domain::rarity_code::RarityCode;
    use crate::domain::trade::{Party, TradeId};
    use crate::domain::user::User;
    use crate::infrastructure::adapter_out::repository::card_repository_adapter::CardRepositoryAdapter;
    use crate::infrastructure::adapter_out::repository::collection_entry_repository_adapter::CollectionEntryRepositoryAdapter;
    use crate::infrastructure::adapter_out::repository::common_repository_tests::{
        insert_binder, insert_card, insert_card_without_cardmarket_id, insert_collection_entry,
        insert_set, insert_trade, insert_trade_card, insert_user, mark_trade_party_confirmed,
        refresh_view,
    };
    use crate::infrastructure::adapter_out::repository::trade_repository_adapter::TradeRepositoryAdapter;
    use chrono::Utc;
    use sqlx::PgPool;

    fn user(id: &str) -> UserId {
        UserId::new(id)
    }

    fn day(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, day).unwrap()
    }

    fn booster(day: NaiveDate, cents: i32) -> NewSealedProduct {
        NewSealedProduct {
            product_type: SealedProductType::Booster,
            set_code: SetCode::new("FDN"),
            name: None,
            amount: PurchaseAmount::try_new(cents).unwrap(),
            purchased_on: day,
        }
    }

    async fn setup(pool: &PgPool) -> PurchaseRepositoryAdapter {
        insert_user(pool, "user-1", "User1").await;
        insert_user(pool, "user-2", "User2").await;
        // FDN is seeded by the first migration.
        insert_set(pool, "DSK").await;
        PurchaseRepositoryAdapter::new(pool.clone())
    }

    fn page(page: u32, page_size: u32) -> Pagination {
        Pagination::try_new(page, page_size, 1_000).unwrap()
    }

    #[sqlx::test]
    async fn create_records_the_sealed_product_with_its_set(pool: PgPool) {
        let repository = setup(&pool).await;
        let mut product = booster(day(13), 499);
        product.name = Some("Deck Bleu".to_string());
        product.product_type = SealedProductType::PreconstructedDeck;

        let purchase = repository
            .create_sealed_product(&user("user-1"), product)
            .await
            .unwrap();

        assert_eq!(
            purchase.kind,
            PurchaseKind::SealedProduct {
                product_type: SealedProductType::PreconstructedDeck,
                name: Some("Deck Bleu".to_string()),
                set: SetName {
                    code: SetCode::new("FDN"),
                    name: "Foundations".to_string()
                },
            }
        );
        assert_eq!(purchase.amount.cents(), 499);
        assert_eq!(purchase.purchased_on, day(13));
        assert_eq!(
            repository.find(&user("user-1"), purchase.id).await.unwrap(),
            purchase
        );
    }

    #[sqlx::test]
    async fn create_refuses_an_unknown_set(pool: PgPool) {
        let repository = setup(&pool).await;
        let mut product = booster(day(13), 499);
        product.set_code = SetCode::new("ZZZ");

        let result = repository
            .create_sealed_product(&user("user-1"), product)
            .await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::SetNotFound))
        ));
    }

    #[sqlx::test]
    async fn a_set_code_the_database_holds_wrong_is_a_server_error(pool: PgPool) {
        let repository = setup(&pool).await;
        insert_set(&pool, "AB").await;
        sqlx::query(
            "INSERT INTO purchase (user_id, kind, product_type, set_code, amount_cents, purchased_on)
             VALUES ('user-1', 'sealed_product', 'BOOSTER', 'AB', 499, '2026-11-13')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let result = repository.list(&user("user-1"), page(0, 10)).await;

        assert!(matches!(result, Err(AppError::Infra(_))));
    }

    #[sqlx::test]
    async fn list_goes_from_the_most_recent_purchase_date_then_creation(pool: PgPool) {
        let repository = setup(&pool).await;
        let old = repository
            .create_sealed_product(&user("user-1"), booster(day(1), 100))
            .await
            .unwrap();
        let recent = repository
            .create_sealed_product(&user("user-1"), booster(day(12), 200))
            .await
            .unwrap();
        // Same day as `recent`, recorded after it: it comes first.
        let recent_later = repository
            .create_sealed_product(&user("user-1"), booster(day(12), 300))
            .await
            .unwrap();
        // Saved last, yet bought before the two others of the 12th.
        let middle = repository
            .create_sealed_product(&user("user-1"), booster(day(5), 400))
            .await
            .unwrap();

        let listed = repository.list(&user("user-1"), page(0, 10)).await.unwrap();

        let ids: Vec<_> = listed.items.iter().map(|p| p.id).collect();
        assert_eq!(ids, vec![recent_later.id, recent.id, middle.id, old.id]);
        assert_eq!(listed.total, 4);
    }

    #[sqlx::test]
    async fn list_is_paginated_and_counts_every_purchase(pool: PgPool) {
        let repository = setup(&pool).await;
        for d in 1..=5 {
            repository
                .create_sealed_product(&user("user-1"), booster(day(d), 100))
                .await
                .unwrap();
        }

        let second = repository.list(&user("user-1"), page(1, 2)).await.unwrap();

        let days: Vec<_> = second.items.iter().map(|p| p.purchased_on).collect();
        assert_eq!(days, vec![day(3), day(2)]);
        assert_eq!(second.total, 5);
        assert_eq!(second.pagination.page(), 1);
    }

    #[sqlx::test]
    async fn a_player_sees_only_their_own_purchases(pool: PgPool) {
        let repository = setup(&pool).await;
        let theirs = repository
            .create_sealed_product(&user("user-2"), booster(day(1), 100))
            .await
            .unwrap();

        let listed = repository.list(&user("user-1"), page(0, 10)).await.unwrap();

        assert!(listed.items.is_empty());
        assert_eq!(listed.total, 0);
        assert!(matches!(
            repository.find(&user("user-1"), theirs.id).await,
            Err(AppError::Functional(FunctionalError::PurchaseNotFound))
        ));
    }

    #[sqlx::test]
    async fn update_replaces_the_given_fields_and_keeps_the_others(pool: PgPool) {
        let repository = setup(&pool).await;
        let mut product = booster(day(1), 100);
        product.name = Some("Deck".to_string());
        let purchase = repository
            .create_sealed_product(&user("user-1"), product)
            .await
            .unwrap();

        let updated = repository
            .update(
                &user("user-1"),
                purchase.id,
                PurchaseChanges {
                    amount: Some(PurchaseAmount::try_new(650).unwrap()),
                    product_type: Some(SealedProductType::Display),
                    set_code: Some(SetCode::new("DSK")),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert_eq!(updated.amount.cents(), 650);
        assert_eq!(updated.purchased_on, day(1));
        assert_eq!(
            updated.kind,
            PurchaseKind::SealedProduct {
                product_type: SealedProductType::Display,
                name: Some("Deck".to_string()),
                set: SetName {
                    code: SetCode::new("DSK"),
                    name: "Set DSK".to_string()
                },
            }
        );
    }

    #[sqlx::test]
    async fn update_clears_the_name_when_asked(pool: PgPool) {
        let repository = setup(&pool).await;
        let mut product = booster(day(1), 100);
        product.name = Some("Deck".to_string());
        let purchase = repository
            .create_sealed_product(&user("user-1"), product)
            .await
            .unwrap();

        let updated = repository
            .update(
                &user("user-1"),
                purchase.id,
                PurchaseChanges {
                    name: Some(None),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        match updated.kind {
            PurchaseKind::SealedProduct { name, .. } => assert_eq!(name, None),
            other => panic!("Expected a sealed product, got {other:?}"),
        }
    }

    #[sqlx::test]
    async fn update_refuses_an_unknown_set_and_keeps_the_purchase(pool: PgPool) {
        let repository = setup(&pool).await;
        let purchase = repository
            .create_sealed_product(&user("user-1"), booster(day(1), 100))
            .await
            .unwrap();

        let result = repository
            .update(
                &user("user-1"),
                purchase.id,
                PurchaseChanges {
                    set_code: Some(SetCode::new("ZZZ")),
                    ..Default::default()
                },
            )
            .await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::SetNotFound))
        ));
        assert_eq!(
            repository.find(&user("user-1"), purchase.id).await.unwrap(),
            purchase
        );
    }

    #[sqlx::test]
    async fn a_player_cannot_update_or_delete_the_purchase_of_another(pool: PgPool) {
        let repository = setup(&pool).await;
        let theirs = repository
            .create_sealed_product(&user("user-2"), booster(day(1), 100))
            .await
            .unwrap();

        let updated = repository
            .update(
                &user("user-1"),
                theirs.id,
                PurchaseChanges {
                    amount: Some(PurchaseAmount::try_new(1).unwrap()),
                    ..Default::default()
                },
            )
            .await;
        let deleted = repository.delete(&user("user-1"), theirs.id).await;

        assert!(matches!(
            updated,
            Err(AppError::Functional(FunctionalError::PurchaseNotFound))
        ));
        assert!(matches!(
            deleted,
            Err(AppError::Functional(FunctionalError::PurchaseNotFound))
        ));
        assert_eq!(
            repository.find(&user("user-2"), theirs.id).await.unwrap(),
            theirs
        );
    }

    #[sqlx::test]
    async fn delete_removes_the_purchase_and_its_content_but_no_collection_entry(pool: PgPool) {
        let repository = setup(&pool).await;
        insert_card_without_cardmarket_id(&pool, "FDN", "1", "FR", "Card").await;
        insert_collection_entry(&pool, "FDN", "1", "FR", false, "user-1", 2, 100, Utc::now()).await;
        let purchase = repository
            .create_sealed_product(&user("user-1"), booster(day(1), 499))
            .await
            .unwrap();
        sqlx::query(
            r#"INSERT INTO purchase_content (purchase_id, set_code, collector_number, language_code, foil, quantity)
               VALUES ($1, 'FDN', '1', 'FR', false, 2)"#,
        )
        .bind(purchase.id.0)
        .execute(&pool)
        .await
        .unwrap();

        repository
            .delete(&user("user-1"), purchase.id)
            .await
            .unwrap();

        let contents: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM purchase_content")
            .fetch_one(&pool)
            .await
            .unwrap();
        let entries: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM collection_entry")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(contents, 0);
        assert_eq!(entries, 1);
        assert!(matches!(
            repository.find(&user("user-1"), purchase.id).await,
            Err(AppError::Functional(FunctionalError::PurchaseNotFound))
        ));
    }

    fn card() -> CardId {
        CardId::new("FDN", "1", LanguageCode::FR)
    }

    async fn seed_card(pool: &PgPool, number: &str, name: &str) {
        insert_card_without_cardmarket_id(pool, "FDN", number, "FR", name).await;
    }

    async fn count(pool: &PgPool, table: &str) -> i64 {
        sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT COUNT(*) FROM {table}")))
            .fetch_one(pool)
            .await
            .unwrap()
    }

    async fn sealed_product(repository: &PurchaseRepositoryAdapter, owner: &str) -> PurchaseId {
        repository
            .create_sealed_product(&user(owner), booster(day(1), 499))
            .await
            .unwrap()
            .id
    }

    #[sqlx::test]
    async fn attaching_in_several_goes_adds_up_the_quantity_of_a_card_in_a_finish(pool: PgPool) {
        let repository = setup(&pool).await;
        seed_card(&pool, "1", "Card").await;
        seed_card(&pool, "2", "Other").await;
        let id = sealed_product(&repository, "user-1").await;
        let user = user("user-1");

        for (card_id, foil, quantity) in [
            (card(), false, 2),
            (card(), false, 3),
            (card(), true, 1),
            (CardId::new("FDN", "2", LanguageCode::FR), false, 1),
        ] {
            repository
                .add_card_to_sealed_product(&user, id, &card_id, foil, quantity, None)
                .await
                .unwrap();
        }

        let content = repository.content(&user, id).await.unwrap();
        let lines: Vec<_> = content
            .iter()
            .map(|line| {
                (
                    line.card_id.collector_number.as_str(),
                    line.foil,
                    line.quantity,
                )
            })
            .collect();
        assert_eq!(
            lines,
            vec![("1", false, 5), ("1", true, 1), ("2", false, 1)]
        );
    }

    #[sqlx::test]
    async fn attached_copies_enter_the_chosen_binder_without_purchase_price(pool: PgPool) {
        let repository = setup(&pool).await;
        seed_card(&pool, "1", "Card").await;
        let binder = BinderId(insert_binder(&pool, "user-1", "Trade").await);
        let id = sealed_product(&repository, "user-1").await;

        let entries = repository
            .add_card_to_sealed_product(&user("user-1"), id, &card(), false, 3, Some(binder))
            .await
            .unwrap();

        assert_eq!(entries.entries.len(), 1);
        assert_eq!(entries.entries[0].quantity, 3);
        assert_eq!(entries.entries[0].purchase_price, None);
        assert_eq!(
            entries.entries[0].binder.as_ref().map(|b| b.id),
            Some(binder)
        );
    }

    #[sqlx::test]
    async fn a_failed_entry_write_leaves_the_content_untouched(pool: PgPool) {
        let repository = setup(&pool).await;
        seed_card(&pool, "1", "Card").await;
        let id = sealed_product(&repository, "user-1").await;
        // The binder of another player is refused after the content was written.
        let others = BinderId(insert_binder(&pool, "user-2", "Bob's").await);

        let error = repository
            .add_card_to_sealed_product(&user("user-1"), id, &card(), false, 1, Some(others))
            .await
            .unwrap_err();

        assert_eq!(error.code(), "binder_not_found");
        assert_eq!(count(&pool, "purchase_content").await, 0);
        assert_eq!(count(&pool, "collection_entry").await, 0);
    }

    #[sqlx::test]
    async fn a_merge_over_255_copies_leaves_the_content_untouched(pool: PgPool) {
        let repository = setup(&pool).await;
        seed_card(&pool, "1", "Card").await;
        insert_collection_entry(
            &pool,
            "FDN",
            "1",
            "FR",
            false,
            "user-1",
            255,
            100,
            Utc::now(),
        )
        .await;
        let id = sealed_product(&repository, "user-1").await;

        let error = repository
            .add_card_to_sealed_product(&user("user-1"), id, &card(), false, 1, None)
            .await
            .unwrap_err();

        assert_eq!(error.code(), "invalid_quantity");
        assert_eq!(count(&pool, "purchase_content").await, 0);
    }

    #[sqlx::test]
    async fn a_player_cannot_attach_cards_to_the_purchase_of_another(pool: PgPool) {
        let repository = setup(&pool).await;
        seed_card(&pool, "1", "Card").await;
        let theirs = sealed_product(&repository, "user-2").await;

        let error = repository
            .add_card_to_sealed_product(&user("user-1"), theirs, &card(), false, 1, None)
            .await
            .unwrap_err();

        assert_eq!(error.code(), "purchase_not_found");
        assert_eq!(count(&pool, "purchase_content").await, 0);
        assert_eq!(count(&pool, "collection_entry").await, 0);
    }

    #[sqlx::test]
    async fn a_single_card_purchase_takes_no_attachment(pool: PgPool) {
        let repository = setup(&pool).await;
        seed_card(&pool, "1", "Card").await;
        let id: Uuid = sqlx::query_scalar(
            r#"INSERT INTO purchase (user_id, kind, amount_cents, purchased_on)
               VALUES ('user-1', 'single_card', 200, '2026-11-13') RETURNING id"#,
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        let error = repository
            .add_card_to_sealed_product(&user("user-1"), PurchaseId(id), &card(), false, 1, None)
            .await
            .unwrap_err();

        assert_eq!(error.code(), "purchase_not_found");
        assert_eq!(count(&pool, "collection_entry").await, 0);
    }

    #[sqlx::test]
    async fn content_reads_the_trend_of_the_finish_and_none_when_cardmarket_has_none(pool: PgPool) {
        let repository = setup(&pool).await;
        insert_card(&pool, "FDN", "1", "FR", "Card", 10).await;
        seed_card(&pool, "2", "No price").await;
        sqlx::query(
            "INSERT INTO cardmarket_price (id_produit, date, trend, trend_foil) VALUES (10, '2026-11-12', 120, 900)",
        )
        .execute(&pool)
        .await
        .unwrap();
        refresh_view(&pool).await;
        let id = sealed_product(&repository, "user-1").await;
        let user = user("user-1");
        for (card_id, foil) in [
            (card(), false),
            (card(), true),
            (CardId::new("FDN", "2", LanguageCode::FR), false),
        ] {
            repository
                .add_card_to_sealed_product(&user, id, &card_id, foil, 2, None)
                .await
                .unwrap();
        }

        let content = repository.content(&user, id).await.unwrap();

        let trends: Vec<_> = content
            .iter()
            .map(|line| (line.name.as_str(), line.foil, line.trend_cents))
            .collect();
        assert_eq!(
            trends,
            vec![
                ("Card", false, Some(120)),
                ("Card", true, Some(900)),
                ("No price", false, None),
            ]
        );
    }

    #[sqlx::test]
    async fn content_of_the_purchase_of_another_player_is_not_found(pool: PgPool) {
        let repository = setup(&pool).await;
        let theirs = sealed_product(&repository, "user-2").await;

        let result = repository.content(&user("user-1"), theirs).await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::PurchaseNotFound))
        ));
    }

    /// Takes the entry `user-1` holds of `card()` away by every way an entry goes, through the
    /// repositories the use cases call: moved into another binder, handed over by a Trade that
    /// completes, removed by its new owner, then `user-1`'s Collection re-imported.
    async fn take_the_entry_away_every_way(pool: &PgPool, foil: bool, quantity: u8) {
        let entries = CollectionEntryRepositoryAdapter::new(pool.clone());
        let giver = user("user-1");
        let receiver = user("user-2");

        let binder = BinderId(insert_binder(pool, "user-1", "Elsewhere").await);
        let held = entries.find_by_card(&giver, &card()).await.unwrap();
        entries
            .move_to_binder(&giver, held.entries[0].id, Some(binder), None)
            .await
            .unwrap();

        let trade_id = Uuid::new_v4();
        insert_trade(pool, trade_id, "user-1", "user-2", "FULLY_ACCEPTED").await;
        mark_trade_party_confirmed(pool, trade_id, true).await;
        insert_trade_card(
            pool,
            trade_id,
            "FDN",
            "1",
            "FR",
            foil,
            "user-1",
            i32::from(quantity),
        )
        .await;
        let trades = TradeRepositoryAdapter::new(pool.clone());
        let trade = trades.find_by_id(TradeId(trade_id)).await.unwrap().unwrap();
        trades
            .apply_transition(&trade.confirm(Party::Respondent).unwrap())
            .await
            .unwrap();

        let received = entries.find_by_card(&receiver, &card()).await.unwrap();
        entries
            .remove(&receiver, received.entries[0].id)
            .await
            .unwrap();

        let importer = User {
            id: giver,
            name: None,
            username: None,
            avatar_url: None,
        };
        let cards = CardRepositoryAdapter::new(pool.clone());
        cards.delete_all(importer.clone()).await.unwrap();
        cards
            .save_all(
                &importer,
                &[ImportedCard {
                    card: Card::new(
                        "FDN",
                        "Foundations",
                        "1",
                        LanguageCode::FR,
                        foil,
                        "Card",
                        RarityCode::C,
                        quantity,
                        100,
                    ),
                    binder_name: None,
                }],
            )
            .await
            .unwrap();
    }

    #[sqlx::test]
    async fn the_content_and_the_amount_survive_a_move_a_trade_transfer_a_removal_and_an_import(
        pool: PgPool,
    ) {
        let repository = setup(&pool).await;
        seed_card(&pool, "1", "Card").await;
        let id = sealed_product(&repository, "user-1").await;
        let user = user("user-1");
        repository
            .add_card_to_sealed_product(&user, id, &card(), false, 2, None)
            .await
            .unwrap();
        let before = (
            repository.find(&user, id).await.unwrap(),
            repository.content(&user, id).await.unwrap(),
        );

        take_the_entry_away_every_way(&pool, false, 2).await;

        let after = (
            repository.find(&user, id).await.unwrap(),
            repository.content(&user, id).await.unwrap(),
        );
        assert_eq!(before, after);
        assert_eq!(count(&pool, "purchase").await, 1);
    }

    /// Adds `quantity` copies of the Foundations card 1 in French at `price` cents, which records
    /// a single card Purchase, and returns it.
    async fn buy_single_card(
        pool: &PgPool,
        repository: &PurchaseRepositoryAdapter,
        user_id: &str,
        foil: bool,
        quantity: u8,
        price: i32,
    ) -> Purchase {
        let card_id = CardId::new("FDN", "1", LanguageCode::FR);
        let price = PurchaseAmount::try_new(price).unwrap();
        let purchase = NewSingleCardPurchase::try_new(price, quantity, day(13)).unwrap();
        CollectionEntryRepositoryAdapter::new(pool.clone())
            .add(
                &user(user_id),
                &card_id,
                foil,
                quantity,
                None,
                Some(purchase),
            )
            .await
            .unwrap();
        repository
            .list(&user(user_id), page(0, 1))
            .await
            .unwrap()
            .items
            .remove(0)
    }

    #[sqlx::test]
    async fn a_single_card_purchase_is_read_with_its_card_finish_and_quantity(pool: PgPool) {
        let repository = setup(&pool).await;
        insert_card_without_cardmarket_id(&pool, "FDN", "1", "FR", "Card").await;
        sqlx::query(
            "UPDATE card SET image_source = 'gatherer_localized', image_has_back = TRUE
             WHERE set_code = 'FDN'",
        )
        .execute(&pool)
        .await
        .unwrap();

        let purchase = buy_single_card(&pool, &repository, "user-1", true, 4, 200).await;

        assert_eq!(purchase.amount.cents(), 800);
        assert_eq!(purchase.purchased_on, day(13));
        assert_eq!(
            purchase.kind,
            PurchaseKind::SingleCard {
                card: PurchasedCard {
                    id: CardId::new("FDN", "1", LanguageCode::FR),
                    name: "Card".to_string(),
                    set: SetName {
                        code: SetCode::new("FDN"),
                        name: "Foundations".to_string()
                    },
                    image: Some(CardImage {
                        source: CardImageSource::GathererLocalized,
                        has_back: true
                    }),
                },
                foil: true,
                quantity: 4,
            }
        );
        assert_eq!(
            repository.find(&user("user-1"), purchase.id).await.unwrap(),
            purchase
        );
    }

    #[sqlx::test]
    async fn a_single_card_purchase_keeps_its_amount_and_content_when_the_entry_goes(pool: PgPool) {
        let repository = setup(&pool).await;
        insert_card_without_cardmarket_id(&pool, "FDN", "1", "FR", "Card").await;
        let purchase = buy_single_card(&pool, &repository, "user-1", false, 2, 150).await;

        take_the_entry_away_every_way(&pool, false, 2).await;

        assert_eq!(
            repository.find(&user("user-1"), purchase.id).await.unwrap(),
            purchase
        );
        assert_eq!(count(&pool, "purchase").await, 1);
    }

    #[sqlx::test]
    async fn a_single_card_purchase_is_edited_in_amount_and_date_only(pool: PgPool) {
        let repository = setup(&pool).await;
        insert_card_without_cardmarket_id(&pool, "FDN", "1", "FR", "Card").await;
        let purchase = buy_single_card(&pool, &repository, "user-1", false, 2, 150).await;

        let updated = repository
            .update(
                &user("user-1"),
                purchase.id,
                PurchaseChanges {
                    amount: Some(PurchaseAmount::try_new(500).unwrap()),
                    purchased_on: Some(day(2)),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert_eq!(updated.amount.cents(), 500);
        assert_eq!(updated.purchased_on, day(2));
        assert_eq!(updated.kind, purchase.kind);
    }

    #[sqlx::test]
    async fn correcting_the_purchase_price_of_an_entry_touches_no_purchase(pool: PgPool) {
        let repository = setup(&pool).await;
        insert_card_without_cardmarket_id(&pool, "FDN", "1", "FR", "Card").await;
        let purchase = buy_single_card(&pool, &repository, "user-1", false, 2, 150).await;
        let entries = CollectionEntryRepositoryAdapter::new(pool.clone());
        let held = entries
            .find_by_card(&user("user-1"), &card())
            .await
            .unwrap();

        let updated = entries
            .update(
                &user("user-1"),
                held.entries[0].id,
                EntryUpdate {
                    purchase_price: Some(999),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert_eq!(updated.entries[0].purchase_price, Some(999));
        assert_eq!(
            repository.find(&user("user-1"), purchase.id).await.unwrap(),
            purchase
        );
        assert_eq!(count(&pool, "purchase").await, 1);
    }

    #[sqlx::test]
    async fn a_single_card_purchase_is_listed_among_the_sealed_products(pool: PgPool) {
        let repository = setup(&pool).await;
        insert_card_without_cardmarket_id(&pool, "FDN", "1", "FR", "Card").await;
        let sealed = repository
            .create_sealed_product(&user("user-1"), booster(day(5), 499))
            .await
            .unwrap();
        let single = buy_single_card(&pool, &repository, "user-1", false, 1, 150).await;

        let listed = repository.list(&user("user-1"), page(0, 10)).await.unwrap();

        let ids: Vec<_> = listed.items.iter().map(|p| p.id).collect();
        assert_eq!(ids, vec![single.id, sealed.id]);
        assert_eq!(listed.total, 2);
    }
}
