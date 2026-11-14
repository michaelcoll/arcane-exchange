use crate::application::error::AppError;
use crate::application::repository::SetNameRepository;
use crate::domain::set_name::{SetCode, SetName};
use crate::domain::user::UserId;
use crate::infrastructure::adapter_out::repository::entities::SetNameEntity;
use async_trait::async_trait;
use sqlx::{Pool, Postgres, QueryBuilder};

pub struct SetNameRepositoryAdapter {
    pool: Pool<Postgres>,
}

impl SetNameRepositoryAdapter {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SetNameRepository for SetNameRepositoryAdapter {
    #[tracing::instrument(name = "set_names_repo.save_all", skip_all, fields(sentry.op = "db"))]
    async fn save_all(&self, sets: &[SetName]) -> Result<(), AppError> {
        if sets.is_empty() {
            return Ok(());
        }

        let mut qb: QueryBuilder<Postgres> =
            QueryBuilder::new("INSERT INTO set_name (set_code, name)");
        qb.push_values(sets, |mut b, set| {
            b.push_bind(set.code.to_string())
                .push_bind(set.name.clone());
        });
        // DO NOTHING, not DO UPDATE: an already-known set's name is never overwritten by a later
        // import.
        qb.push("ON CONFLICT (set_code) DO NOTHING");
        qb.build().execute(&self.pool).await?;

        Ok(())
    }

    /// The whole catalog of sets, the most recent first; a set without a release date comes after
    /// the dated ones. The name breaks ties. With an `owner`, only the sets they own a card of.
    #[tracing::instrument(name = "set_names_repo.find_all", skip_all, fields(sentry.op = "db"))]
    async fn find_all(&self, owner: Option<UserId>) -> Result<Vec<SetName>, AppError> {
        Ok(sqlx::query_as!(
            SetNameEntity,
            r#"SELECT sn.set_code, sn.name
               FROM set_name sn
               WHERE $1::text IS NULL
                  OR EXISTS (SELECT 1 FROM collection_entry ce
                             WHERE ce.set_code = sn.set_code AND ce.user_id = $1)
               ORDER BY sn.released_at DESC NULLS LAST, sn.name"#,
            owner.as_ref().map(UserId::as_str)
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|e| SetName {
            code: SetCode::new(e.set_code),
            name: e.name,
        })
        .collect())
    }

    #[tracing::instrument(name = "set_names_repo.find_by_code", skip_all, fields(sentry.op = "db"))]
    async fn find_by_code(&self, code: SetCode) -> Result<Option<SetName>, AppError> {
        Ok(sqlx::query_as!(
            SetNameEntity,
            "SELECT set_code, name FROM set_name WHERE set_code = $1",
            code.to_string()
        )
        .fetch_optional(&self.pool)
        .await?
        .map(|e| SetName {
            code: SetCode::new(e.set_code),
            name: e.name,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::user::UserId;
    use crate::infrastructure::adapter_out::repository::common_repository_tests::{
        insert_card_without_cardmarket_id, insert_collection_entry, insert_set, insert_user,
    };
    use chrono::Utc;
    use sqlx::PgPool;

    async fn rename_and_date_set(pool: &PgPool, set_code: &str, name: &str, date: Option<&str>) {
        sqlx::query("UPDATE set_name SET name = $2, released_at = $3::date WHERE set_code = $1")
            .bind(set_code)
            .bind(name)
            .bind(date)
            .execute(pool)
            .await
            .unwrap();
    }

    async fn listed_codes(
        adapter: &SetNameRepositoryAdapter,
        owner: Option<UserId>,
    ) -> Vec<String> {
        adapter
            .find_all(owner)
            .await
            .unwrap()
            .into_iter()
            .map(|set| set.code.to_string())
            .collect()
    }

    #[sqlx::test]
    async fn find_all_lists_a_set_nobody_owns_a_card_of(pool: PgPool) {
        let adapter = SetNameRepositoryAdapter::new(pool.clone());
        insert_set(&pool, "OWN").await;
        insert_set(&pool, "EMP").await;
        insert_card_without_cardmarket_id(&pool, "OWN", "1", "en", "Owned Card").await;
        insert_user(&pool, "user-1", "User1").await;
        insert_collection_entry(&pool, "OWN", "1", "en", false, "user-1", 1, 100, Utc::now()).await;

        let codes = listed_codes(&adapter, None).await;

        assert!(codes.contains(&"EMP".to_string()), "{codes:?}");
        assert!(codes.contains(&"OWN".to_string()), "{codes:?}");
    }

    #[sqlx::test]
    async fn find_all_owned_lists_only_the_sets_the_caller_owns_a_card_of(pool: PgPool) {
        let adapter = SetNameRepositoryAdapter::new(pool.clone());
        for code in ["MIN", "OTH", "NON"] {
            insert_set(&pool, code).await;
            insert_card_without_cardmarket_id(&pool, code, "1", "en", "Card").await;
        }
        insert_user(&pool, "user-1", "User1").await;
        insert_user(&pool, "user-2", "User2").await;
        insert_collection_entry(&pool, "MIN", "1", "en", false, "user-1", 1, 100, Utc::now()).await;
        insert_collection_entry(&pool, "OTH", "1", "en", false, "user-2", 1, 100, Utc::now()).await;

        let codes = listed_codes(&adapter, Some(UserId::new("user-1"))).await;

        assert_eq!(codes, vec!["MIN"]);
    }

    #[sqlx::test]
    async fn find_all_owned_keeps_the_catalog_order(pool: PgPool) {
        let adapter = SetNameRepositoryAdapter::new(pool.clone());
        for code in ["OLD", "NEW", "UND"] {
            insert_set(&pool, code).await;
            insert_card_without_cardmarket_id(&pool, code, "1", "en", "Card").await;
        }
        rename_and_date_set(&pool, "OLD", "Old", Some("2001-01-01")).await;
        rename_and_date_set(&pool, "NEW", "New", Some("2024-02-09")).await;
        rename_and_date_set(&pool, "UND", "Undated", None).await;
        insert_user(&pool, "user-1", "User1").await;
        for code in ["OLD", "UND", "NEW"] {
            insert_collection_entry(&pool, code, "1", "en", false, "user-1", 1, 100, Utc::now())
                .await;
        }

        let codes = listed_codes(&adapter, Some(UserId::new("user-1"))).await;

        assert_eq!(codes, vec!["NEW", "OLD", "UND"]);
    }

    #[sqlx::test]
    async fn find_all_lists_the_most_recent_first_then_by_name_and_undated_last(pool: PgPool) {
        let adapter = SetNameRepositoryAdapter::new(pool.clone());
        sqlx::query("DELETE FROM set_name")
            .execute(&pool)
            .await
            .unwrap();
        for code in ["OLD", "NEWB", "NEWA", "UNDB", "UNDA"] {
            insert_set(&pool, code).await;
        }
        rename_and_date_set(&pool, "OLD", "Old", Some("2001-01-01")).await;
        rename_and_date_set(&pool, "NEWB", "B new", Some("2024-02-09")).await;
        rename_and_date_set(&pool, "NEWA", "A new", Some("2024-02-09")).await;
        rename_and_date_set(&pool, "UNDB", "B undated", None).await;
        rename_and_date_set(&pool, "UNDA", "A undated", None).await;

        assert_eq!(
            listed_codes(&adapter, None).await,
            vec!["NEWA", "NEWB", "OLD", "UNDA", "UNDB"]
        );
    }

    #[sqlx::test]
    async fn save_all_inserts_new_sets_and_ignores_existing_ones(pool: PgPool) {
        let adapter = SetNameRepositoryAdapter::new(pool.clone());

        adapter
            .save_all(&[
                SetName {
                    code: SetCode::new("ECC"),
                    name: "Lorwyn Eclipsed Commander".to_string(),
                },
                SetName {
                    // "ECL" already exists (seeded in migrations) under a different name.
                    code: SetCode::new("ECL"),
                    name: "Renamed".to_string(),
                },
            ])
            .await
            .unwrap();

        let ecc = adapter
            .find_by_code(SetCode::new("ECC"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ecc.name, "Lorwyn Eclipsed Commander");

        let ecl = adapter
            .find_by_code(SetCode::new("ECL"))
            .await
            .unwrap()
            .unwrap();
        assert_ne!(
            ecl.name, "Renamed",
            "an existing set's name is never overwritten"
        );
    }

    #[sqlx::test]
    async fn save_all_does_nothing_for_an_empty_slice(pool: PgPool) {
        let adapter = SetNameRepositoryAdapter::new(pool);
        adapter.save_all(&[]).await.unwrap();
    }
}
