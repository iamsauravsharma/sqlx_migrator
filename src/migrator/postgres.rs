use sqlx::{Pool, Postgres};

use super::{DatabaseOperation, Migrate, Migrator};
use crate::error::Error;
use crate::migration::{AppliedMigrationSqlRow, Migration};

/// Create migrator table query
#[must_use]
pub(crate) fn create_migrator_table_query() -> &'static str {
    "CREATE TABLE IF NOT EXISTS _sqlx_migrator_migrations (
        id INT PRIMARY KEY NOT NULL GENERATED ALWAYS AS IDENTITY,
        app TEXT NOT NULL,
        name TEXT NOT NULL,
        applied_time TIMESTAMPTZ NOT NULL DEFAULT now(),
        UNIQUE (app, name)
    )"
}

/// Drop table query
#[must_use]
pub(crate) fn drop_table_query() -> &'static str {
    "DROP TABLE IF EXISTS _sqlx_migrator_migrations"
}

/// Fetch row query
#[must_use]
pub(crate) fn fetch_row_query() -> &'static str {
    "SELECT id, app, name, applied_time FROM _sqlx_migrator_migrations"
}

/// Add migration query
#[must_use]
pub(crate) fn add_migration_query() -> &'static str {
    "INSERT INTO _sqlx_migrator_migrations(app, name) VALUES ($1, $2)"
}

/// Delete migration query
#[must_use]
pub(crate) fn delete_migration_query() -> &'static str {
    "DELETE FROM _sqlx_migrator_migrations WHERE app = $1 AND name = $2"
}

/// get lock id
pub(crate) async fn lock_id(pool: &Pool<Postgres>) -> Result<i64, Error> {
    let (database,): (String,) = sqlx::query_as("SELECT CURRENT_DATABASE()")
        .fetch_one(pool)
        .await?;
    Ok(i64::from(crc32fast::hash(database.as_bytes())))
}

/// get lock database query
/// # Errors
/// Failed to lock database
pub(crate) fn lock_database_query() -> &'static str {
    "SELECT pg_advisory_lock($1)"
}

/// get lock database query
/// # Errors
/// Failed to lock database
pub(crate) fn unlock_database_query() -> &'static str {
    "SELECT pg_advisory_unlock($1)"
}

#[async_trait::async_trait]
impl DatabaseOperation<Postgres> for Migrator<Postgres> {
    async fn ensure_migration_table_exists(&self) -> Result<(), Error> {
        sqlx::query(create_migrator_table_query())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn drop_migration_table_if_exists(&self) -> Result<(), Error> {
        sqlx::query(drop_table_query()).execute(&self.pool).await?;
        Ok(())
    }

    async fn add_migration_to_db_table(
        &self,
        migration: &Box<dyn Migration<Postgres>>,
        connection: &mut <Postgres as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        sqlx::query(add_migration_query())
            .bind(migration.app())
            .bind(migration.name())
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn delete_migration_from_db_table(
        &self,
        migration: &Box<dyn Migration<Postgres>>,
        connection: &mut <Postgres as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        sqlx::query(delete_migration_query())
            .bind(migration.app())
            .bind(migration.name())
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn fetch_applied_migration_from_db(&self) -> Result<Vec<AppliedMigrationSqlRow>, Error> {
        let rows = sqlx::query_as(fetch_row_query())
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    async fn lock(
        &self,
        connection: &mut <Postgres as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        let lock_id = lock_id(&self.pool).await?;
        sqlx::query(lock_database_query())
            .bind(lock_id)
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn unlock(
        &self,
        connection: &mut <Postgres as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        let lock_id = lock_id(&self.pool).await?;
        sqlx::query(unlock_database_query())
            .bind(lock_id)
            .execute(connection)
            .await?;
        Ok(())
    }
}

impl Migrate<Postgres> for Migrator<Postgres> {}
