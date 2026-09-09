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
        // DO NOTHING, not DO UPDATE: matches `save()`'s existing exists_by_code + save guard —
        // an already-known set's name is never overwritten by a later import.
        qb.push("ON CONFLICT (set_code) DO NOTHING");
        qb.build().execute(&self.pool).await?;

        Ok(())
    }

    #[tracing::instrument(name = "set_names_repo.find_all", skip_all, fields(sentry.op = "db"))]
    async fn find_all(&self) -> Result<Vec<SetName>, AppError> {
        Ok(
            sqlx::query_as!(SetNameEntity, "SELECT * FROM set_name ORDER BY name")
                .fetch_all(&self.pool)
                .await?
                .into_iter()
                .map(|e| SetName {
                    code: SetCode::new(e.set_code),
                    name: e.name,
                })
                .collect(),
        )
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
    use sqlx::PgPool;

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
