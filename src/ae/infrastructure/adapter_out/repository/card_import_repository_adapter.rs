use crate::application::error::AppError;
use crate::application::repository::CardImportRepository;
use crate::domain::card_import::{CardImport, CardImportId, CardImportLineError, CardImportStatus};
use crate::domain::error::FunctionalError;
use crate::domain::user::UserId;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::types::Json;
use sqlx::{Pool, Postgres};

/// Postgres error code for `unique_violation`.
const UNIQUE_VIOLATION: &str = "23505";

/// JSON shape stored in the `line_errors` JSONB column. Kept private to this adapter rather than
/// deriving `Serialize`/`Deserialize` on the domain type.
#[derive(serde::Serialize, serde::Deserialize)]
struct LineErrorJson {
    line: i64,
    field: String,
    value: String,
}

impl From<&CardImportLineError> for LineErrorJson {
    fn from(e: &CardImportLineError) -> Self {
        Self {
            line: e.line as i64,
            field: e.field.clone(),
            value: e.value.clone(),
        }
    }
}

impl From<LineErrorJson> for CardImportLineError {
    fn from(e: LineErrorJson) -> Self {
        Self {
            line: e.line as usize,
            field: e.field,
            value: e.value,
        }
    }
}

pub struct CardImportRepositoryAdapter {
    pool: Pool<Postgres>,
}

impl CardImportRepositoryAdapter {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

struct CardImportRow {
    id: uuid::Uuid,
    user_id: String,
    status: String,
    source_lines: i32,
    total_lines: i32,
    processed_lines: i32,
    line_errors: Json<Vec<LineErrorJson>>,
    line_error_count: i32,
    error_message: Option<String>,
    created_at: DateTime<Utc>,
    finished_at: Option<DateTime<Utc>>,
}

impl TryFrom<CardImportRow> for CardImport {
    type Error = AppError;

    fn try_from(row: CardImportRow) -> Result<Self, Self::Error> {
        Ok(CardImport {
            id: CardImportId(row.id),
            user_id: UserId::new(row.user_id),
            status: CardImportStatus::try_new(&row.status)?,
            source_lines: row.source_lines as u32,
            total_lines: row.total_lines as u32,
            processed_lines: row.processed_lines as u32,
            line_errors: row.line_errors.0.into_iter().map(Into::into).collect(),
            line_error_count: row.line_error_count as u32,
            error_message: row.error_message,
            created_at: row.created_at,
            finished_at: row.finished_at,
        })
    }
}

#[async_trait]
impl CardImportRepository for CardImportRepositoryAdapter {
    #[tracing::instrument(name = "card_import_repo.create", skip_all, fields(sentry.op = "db"))]
    async fn create(&self, import: &CardImport) -> Result<(), AppError> {
        let line_errors: Vec<LineErrorJson> = import.line_errors.iter().map(Into::into).collect();

        let result = sqlx::query!(
            r#"
            INSERT INTO card_import
                (id, user_id, status, source_lines, total_lines, processed_lines,
                 line_errors, line_error_count, error_message, created_at, finished_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
            import.id.0,
            import.user_id.as_str(),
            import.status.to_string(),
            import.source_lines as i32,
            import.total_lines as i32,
            import.processed_lines as i32,
            Json(line_errors) as _,
            import.line_error_count as i32,
            import.error_message,
            import.created_at,
            import.finished_at
        )
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(db_err))
                if db_err.code().as_deref() == Some(UNIQUE_VIOLATION) =>
            {
                Err(AppError::Functional(FunctionalError::ImportAlreadyRunning))
            }
            Err(e) => Err(e.into()),
        }
    }

    #[tracing::instrument(name = "card_import_repo.find_by_id", skip_all, fields(sentry.op = "db"))]
    async fn find_by_id(&self, id: &CardImportId) -> Result<Option<CardImport>, AppError> {
        let row = sqlx::query_as!(
            CardImportRow,
            r#"
            SELECT id, user_id, status, source_lines, total_lines, processed_lines,
                   line_errors as "line_errors: Json<Vec<LineErrorJson>>",
                   line_error_count, error_message, created_at, finished_at
            FROM card_import
            WHERE id = $1
            "#,
            id.0
        )
        .fetch_optional(&self.pool)
        .await?;

        row.map(CardImport::try_from).transpose()
    }

    #[tracing::instrument(name = "card_import_repo.list_by_user", skip_all, fields(sentry.op = "db"))]
    async fn list_by_user(&self, user_id: &UserId) -> Result<Vec<CardImport>, AppError> {
        let rows = sqlx::query_as!(
            CardImportRow,
            r#"
            SELECT id, user_id, status, source_lines, total_lines, processed_lines,
                   line_errors as "line_errors: Json<Vec<LineErrorJson>>",
                   line_error_count, error_message, created_at, finished_at
            FROM card_import
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#,
            user_id.as_str()
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(CardImport::try_from).collect()
    }

    #[tracing::instrument(name = "card_import_repo.mark_running", skip_all, fields(sentry.op = "db"))]
    async fn mark_running(&self, id: &CardImportId) -> Result<(), AppError> {
        sqlx::query!(
            "UPDATE card_import SET status = 'running' WHERE id = $1",
            id.0
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    #[tracing::instrument(name = "card_import_repo.update_progress", skip_all, fields(sentry.op = "db"))]
    async fn update_progress(
        &self,
        id: &CardImportId,
        processed_lines: u32,
    ) -> Result<(), AppError> {
        sqlx::query!(
            "UPDATE card_import SET processed_lines = $2 WHERE id = $1",
            id.0,
            processed_lines as i32
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    #[tracing::instrument(name = "card_import_repo.finish", skip_all, fields(sentry.op = "db"))]
    async fn finish(
        &self,
        id: &CardImportId,
        status: CardImportStatus,
        error_message: Option<&str>,
    ) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            UPDATE card_import
            SET status = $2, error_message = $3, finished_at = now()
            WHERE id = $1
            "#,
            id.0,
            status.to_string(),
            error_message
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    #[tracing::instrument(name = "card_import_repo.fail_all_active", skip_all, fields(sentry.op = "db"))]
    async fn fail_all_active(&self, reason: &str) -> Result<u64, AppError> {
        let result = sqlx::query!(
            r#"
            UPDATE card_import
            SET status = 'failed', error_message = $1, finished_at = now()
            WHERE status IN ('pending', 'running')
            "#,
            reason
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    #[tracing::instrument(name = "card_import_repo.purge_old", skip_all, fields(sentry.op = "db"))]
    async fn purge_old(&self, user_id: &UserId, keep: i64) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            DELETE FROM card_import
            WHERE id IN (
                SELECT id FROM card_import
                WHERE user_id = $1 AND status IN ('completed', 'failed')
                ORDER BY created_at DESC
                OFFSET $2
            )
            "#,
            user_id.as_str(),
            keep
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::adapter_out::repository::common_repository_tests::insert_user;
    use sqlx::PgPool;

    fn new_import(user_id: &str) -> CardImport {
        CardImport {
            id: CardImportId::new(),
            user_id: UserId::new(user_id),
            status: CardImportStatus::Pending,
            source_lines: 10,
            total_lines: 8,
            processed_lines: 0,
            line_errors: vec![],
            line_error_count: 0,
            error_message: None,
            created_at: Utc::now(),
            finished_at: None,
        }
    }

    #[sqlx::test]
    async fn create_then_find_by_id_round_trips(pool: PgPool) {
        insert_user(&pool, "u1", "user one").await;
        let adapter = CardImportRepositoryAdapter::new(pool);

        let import = new_import("u1");
        adapter.create(&import).await.unwrap();

        let found = adapter.find_by_id(&import.id).await.unwrap().unwrap();
        assert_eq!(found.status, CardImportStatus::Pending);
        assert_eq!(found.total_lines, 8);
    }

    #[sqlx::test]
    async fn list_by_user_orders_most_recent_first(pool: PgPool) {
        insert_user(&pool, "u1", "user one").await;
        let adapter = CardImportRepositoryAdapter::new(pool);

        let base = Utc::now();
        let mut ids = Vec::with_capacity(3);
        for i in 0..3 {
            let mut import = new_import("u1");
            import.created_at = base + chrono::Duration::hours(i);
            ids.push(import.id);
            adapter.create(&import).await.unwrap();
            // Finish it so the next `create` isn't rejected by the one-active-import constraint.
            adapter
                .finish(&import.id, CardImportStatus::Completed, None)
                .await
                .unwrap();
        }

        let listed = adapter.list_by_user(&UserId::new("u1")).await.unwrap();
        let listed_ids: Vec<CardImportId> = listed.iter().map(|i| i.id).collect();
        let expected_ids: Vec<CardImportId> = ids.into_iter().rev().collect();
        assert_eq!(
            listed_ids, expected_ids,
            "most recently created import first"
        );
    }

    #[sqlx::test]
    async fn create_rejects_second_active_import_for_same_user(pool: PgPool) {
        insert_user(&pool, "u1", "user one").await;
        let adapter = CardImportRepositoryAdapter::new(pool);

        adapter.create(&new_import("u1")).await.unwrap();
        let second = adapter.create(&new_import("u1")).await;

        assert!(matches!(
            second,
            Err(AppError::Functional(FunctionalError::ImportAlreadyRunning))
        ));
    }

    #[sqlx::test]
    async fn create_allows_active_imports_for_distinct_users(pool: PgPool) {
        insert_user(&pool, "u1", "user one").await;
        insert_user(&pool, "u2", "user two").await;
        let adapter = CardImportRepositoryAdapter::new(pool);

        adapter.create(&new_import("u1")).await.unwrap();
        adapter.create(&new_import("u2")).await.unwrap();
    }

    #[sqlx::test]
    async fn purge_old_keeps_only_the_most_recent_completed_imports(pool: PgPool) {
        insert_user(&pool, "u1", "user one").await;
        let adapter = CardImportRepositoryAdapter::new(pool);

        // Unambiguous, strictly increasing `created_at` (an hour apart) so the assertions below
        // cannot pass by accident if `purge_old` or `list_by_user` had the ordering backwards.
        let base = Utc::now();
        let mut ids = Vec::with_capacity(11);
        for i in 0..11 {
            let mut import = new_import("u1");
            import.status = CardImportStatus::Completed;
            import.created_at = base + chrono::Duration::hours(i);
            ids.push(import.id);
            adapter.create(&import).await.unwrap();
            adapter
                .finish(&import.id, CardImportStatus::Completed, None)
                .await
                .unwrap();
        }

        adapter.purge_old(&UserId::new("u1"), 10).await.unwrap();

        let remaining = adapter.list_by_user(&UserId::new("u1")).await.unwrap();
        assert_eq!(remaining.len(), 10);
        // The oldest (index 0) was purged; the 10 kept are the most recent, most-recent-first.
        let remaining_ids: Vec<CardImportId> = remaining.iter().map(|i| i.id).collect();
        let expected_ids: Vec<CardImportId> = ids[1..].iter().rev().copied().collect();
        assert_eq!(remaining_ids, expected_ids);
    }

    #[sqlx::test]
    async fn fail_all_active_marks_pending_and_running_as_failed(pool: PgPool) {
        insert_user(&pool, "u1", "user one").await;
        let adapter = CardImportRepositoryAdapter::new(pool);

        let import = new_import("u1");
        adapter.create(&import).await.unwrap();
        adapter.mark_running(&import.id).await.unwrap();

        let affected = adapter.fail_all_active("server restarted").await.unwrap();
        assert_eq!(affected, 1);

        let found = adapter.find_by_id(&import.id).await.unwrap().unwrap();
        assert_eq!(found.status, CardImportStatus::Failed);
        assert_eq!(found.error_message.as_deref(), Some("server restarted"));

        // The active slot is now free.
        adapter.create(&new_import("u1")).await.unwrap();
    }

    #[sqlx::test]
    async fn update_progress_is_visible_to_a_later_read(pool: PgPool) {
        insert_user(&pool, "u1", "user one").await;
        let adapter = CardImportRepositoryAdapter::new(pool);

        let import = new_import("u1");
        adapter.create(&import).await.unwrap();

        adapter.update_progress(&import.id, 3).await.unwrap();
        let first = adapter.find_by_id(&import.id).await.unwrap().unwrap();

        adapter.update_progress(&import.id, 8).await.unwrap();
        let second = adapter.find_by_id(&import.id).await.unwrap().unwrap();

        assert!(second.processed_lines >= first.processed_lines);
        assert_eq!(second.processed_lines, 8);
    }
}
