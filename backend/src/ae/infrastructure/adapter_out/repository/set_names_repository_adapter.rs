use crate::application::error::AppError;
use crate::application::repository::SetNameRepository;
use crate::domain::set_name::{SetCode, SetName};
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

    /// Only the sets of which a card is owned: `set_name` also holds sets nobody owns a card of.
    #[tracing::instrument(name = "set_names_repo.find_all", skip_all, fields(sentry.op = "db"))]
    async fn find_all(&self) -> Result<Vec<SetName>, AppError> {
        Ok(sqlx::query_as!(
            SetNameEntity,
            r#"SELECT sn.set_code, sn.name
               FROM set_name sn
               WHERE EXISTS (SELECT 1 FROM collection_entry ce WHERE ce.set_code = sn.set_code)
               ORDER BY sn.name"#
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
            "SELECT * FROM set_name WHERE set_code = $1",
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
    use crate::infrastructure::adapter_out::repository::common_repository_tests::{
        insert_card_without_cardmarket_id, insert_collection_entry, insert_set, insert_user,
    };
    use chrono::Utc;
    use sqlx::PgPool;

    #[sqlx::test]
    async fn find_all_only_lists_sets_with_an_owned_card(pool: PgPool) {
        let adapter = SetNameRepositoryAdapter::new(pool.clone());
        insert_set(&pool, "OWN").await;
        insert_set(&pool, "ORP").await;
        insert_set(&pool, "EMP").await;
        insert_card_without_cardmarket_id(&pool, "OWN", "1", "en", "Owned Card").await;
        insert_card_without_cardmarket_id(&pool, "ORP", "1", "en", "Orphan Card").await;
        insert_user(&pool, "user-1", "User1").await;
        insert_collection_entry(&pool, "OWN", "1", "en", false, "user-1", 1, 100, Utc::now()).await;

        let codes: Vec<String> = adapter
            .find_all()
            .await
            .unwrap()
            .into_iter()
            .map(|set| set.code.to_string())
            .collect();

        assert_eq!(
            codes,
            vec!["OWN"],
            "a set whose cards are all unowned, or without card, is not listed"
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
