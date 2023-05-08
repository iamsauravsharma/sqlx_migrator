use sqlx::{MySql, Pool};

use super::{DatabaseOperation, Migrate, Migrator};
use crate::error::Error;
use crate::migration::{AppliedMigrationSqlRow, Migration};

/// create migrator table query
#[must_use]
pub(crate) fn create_migrator_table_query() -> &'static str {
    "CREATE TABLE IF NOT EXISTS _sqlx_migrator_migrations (
        id INT PRIMARY KEY NOT NULL AUTO_INCREMENT,
        app VARCHAR(384) NOT NULL,
        name VARCHAR(384) NOT NULL,
        applied_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
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

/// add migration query
#[must_use]
pub(crate) fn add_migration_query() -> &'static str {
    "INSERT INTO _sqlx_migrator_migrations(app, name) VALUES (?, ?)"
}

/// delete migration query
#[must_use]
pub(crate) fn delete_migration_query() -> &'static str {
    "DELETE FROM _sqlx_migrator_migrations WHERE app = ? AND name = ?"
}

/// Lock database
/// # Errors
/// Failed to obtain lock of database
pub(crate) async fn lock_database(pool: &Pool<MySql>) -> Result<(), Error> {
    let row: (String,) = sqlx::query_as("SELECT DATABASE()").fetch_one(pool).await?;
    let lock_id = crc32fast::hash(row.0.as_bytes()).to_string();
    sqlx::query("SELECT GET_LOCK(?, -1)")
        .bind(lock_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// unlock database
/// # Errors
/// Failed to unlock database
pub(crate) async fn unlock_database(pool: &Pool<MySql>) -> Result<(), Error> {
    let row: (String,) = sqlx::query_as("SELECT DATABASE()").fetch_one(pool).await?;
    let lock_id = crc32fast::hash(row.0.as_bytes()).to_string();
    sqlx::query("SELECT RELEASE_LOCK(?)")
        .bind(lock_id)
        .execute(pool)
        .await?;
    Ok(())
}

#[async_trait::async_trait]
impl DatabaseOperation<MySql> for Migrator<MySql> {
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
        migration: &Box<dyn Migration<MySql>>,
        connection: &mut <MySql as sqlx::Database>::Connection,
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
        migration: &Box<dyn Migration<MySql>>,
        connection: &mut <MySql as sqlx::Database>::Connection,
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

    async fn lock(&self) -> Result<(), Error> {
        lock_database(&self.pool).await
    }

    async fn unlock(&self) -> Result<(), Error> {
        unlock_database(&self.pool).await
    }
}

impl Migrate<MySql> for Migrator<MySql> {}
