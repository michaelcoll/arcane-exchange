use crate::application::error::{AppError, InfraError};
use crate::application::repository::PurchaseRepository;
use crate::domain::error::FunctionalError;
use crate::domain::pagination::{Paginated, Pagination};
use crate::domain::purchase::{
    NewSealedProduct, Purchase, PurchaseAmount, PurchaseChanges, PurchaseId, PurchaseKind,
    SealedProductType,
};
use crate::domain::set_name::{SetCode, SetName};
use crate::domain::user::UserId;
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
        let rows = sqlx::query_as!(
            PurchaseRow,
            r#"
            SELECT p.id, p.kind, p.amount_cents, p.purchased_on, p.product_type, p.name,
                   p.set_code, sn.name AS "set_name?"
            FROM purchase p
                     LEFT JOIN set_name sn ON sn.set_code = p.set_code
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
        sqlx::query_as!(
            PurchaseRow,
            r#"
            SELECT p.id, p.kind, p.amount_cents, p.purchased_on, p.product_type, p.name,
                   p.set_code, sn.name AS "set_name?"
            FROM purchase p
                     LEFT JOIN set_name sn ON sn.set_code = p.set_code
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
    use crate::infrastructure::adapter_out::repository::common_repository_tests::{
        insert_card_without_cardmarket_id, insert_collection_entry, insert_set, insert_user,
    };
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

    #[sqlx::test]
    async fn removing_the_players_collection_entries_leaves_a_purchase_untouched(pool: PgPool) {
        let repository = setup(&pool).await;
        insert_card_without_cardmarket_id(&pool, "FDN", "1", "FR", "Card").await;
        insert_collection_entry(&pool, "FDN", "1", "FR", false, "user-1", 2, 100, Utc::now()).await;
        let purchase = repository
            .create_sealed_product(&user("user-1"), booster(day(1), 499))
            .await
            .unwrap();

        sqlx::query("DELETE FROM collection_entry")
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(
            repository.find(&user("user-1"), purchase.id).await.unwrap(),
            purchase
        );
    }
}
