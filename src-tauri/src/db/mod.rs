//! SQLite access layer. Two logical databases:
//!   * the **app registry** (`migrations/app`) — workspaces + global settings,
//!   * each **workspace** (`migrations/workspace`) — sources, documents, chunks…
//!
//! SQL strings live below this module (in `repo`, landing with M1). Everything
//! above the db layer calls typed helpers, never raw SQL.

pub mod repo;

use std::path::Path;

use sqlx::migrate::Migrator;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::SqlitePool;

use crate::error::AppResult;

/// Migrations for the app-level registry database.
pub static APP_MIGRATOR: Migrator = sqlx::migrate!("migrations/app");

/// Migrations for a per-workspace database.
pub static WORKSPACE_MIGRATOR: Migrator = sqlx::migrate!("migrations/workspace");

/// Tuned connect options: foreign keys on, WAL journaling, NORMAL sync.
/// `create_if_missing` lets first launch create the file.
fn connect_options(db_path: &Path) -> SqliteConnectOptions {
    SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
}

/// Open a pooled connection to a SQLite file at `db_path`.
pub async fn open_pool(db_path: &Path) -> AppResult<SqlitePool> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options(db_path))
        .await?;
    Ok(pool)
}

/// Open the app registry database and apply its migrations.
pub async fn open_app_db(db_path: &Path) -> AppResult<SqlitePool> {
    let pool = open_pool(db_path).await?;
    APP_MIGRATOR.run(&pool).await?;
    Ok(pool)
}

/// Open a workspace database and apply its migrations.
pub async fn open_workspace_db(db_path: &Path) -> AppResult<SqlitePool> {
    let pool = open_pool(db_path).await?;
    WORKSPACE_MIGRATOR.run(&pool).await?;
    Ok(pool)
}

/// Open an in-memory pool (single shared connection) for tests. The connection
/// must stay in the pool for the in-memory database to persist across queries.
#[cfg(test)]
async fn open_memory_pool() -> AppResult<SqlitePool> {
    use std::str::FromStr;
    let opts = SqliteConnectOptions::from_str("sqlite::memory:")
        .expect("valid sqlite memory url")
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await?;
    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn table_names(pool: &SqlitePool) -> Vec<String> {
        sqlx::query_scalar::<_, String>(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlx_%' AND name NOT LIKE 'sqlite_%' ORDER BY name",
        )
        .fetch_all(pool)
        .await
        .expect("query table names")
    }

    #[tokio::test]
    async fn app_migrations_create_registry_tables() {
        let pool = open_memory_pool().await.unwrap();
        APP_MIGRATOR.run(&pool).await.unwrap();
        let tables = table_names(&pool).await;
        assert!(tables.contains(&"workspace".to_string()));
        assert!(tables.contains(&"app_setting".to_string()));
    }

    #[tokio::test]
    async fn workspace_migrations_create_all_tables() {
        let pool = open_memory_pool().await.unwrap();
        WORKSPACE_MIGRATOR.run(&pool).await.unwrap();
        let tables = table_names(&pool).await;
        for expected in [
            "source",
            "document",
            "chunk",
            "embedding",
            "conversation",
            "message",
            "citation",
        ] {
            assert!(
                tables.contains(&expected.to_string()),
                "missing table {expected}; got {tables:?}"
            );
        }
    }

    #[tokio::test]
    async fn foreign_keys_cascade_on_source_delete() {
        let pool = open_memory_pool().await.unwrap();
        WORKSPACE_MIGRATOR.run(&pool).await.unwrap();

        sqlx::query(
            "INSERT INTO source (id, kind, uri, status) VALUES ('s1','folder','/tmp','ready')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO document (id, source_id, path_or_url, content_hash) VALUES ('d1','s1','/tmp/a.md','hash')")
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query("DELETE FROM source WHERE id='s1'")
            .execute(&pool)
            .await
            .unwrap();

        let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM document")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(remaining, 0, "documents should cascade-delete with source");
    }
}
