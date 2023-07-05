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

/// fetch rows
pub(crate) async fn fetch_rows(pool: &Pool<MySql>) -> Result<Vec<AppliedMigrationSqlRow>, Error> {
    Ok(
        sqlx::query_as("SELECT id, app, name, applied_time FROM _sqlx_migrator_migrations")
            .fetch_all(pool)
            .await?,
    )
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

/// get lock id
pub(crate) async fn lock_id(pool: &Pool<MySql>) -> Result<String, Error> {
    let (database_name,): (String,) = sqlx::query_as("SELECT DATABASE()").fetch_one(pool).await?;
    Ok(crc32fast::hash(database_name.as_bytes()).to_string())
}

/// get lock database query
/// # Errors
/// Failed to lock database
pub(crate) fn lock_database_query() -> &'static str {
    "SELECT GET_LOCK(?, -1)"
}

/// get lock database query
/// # Errors
/// Failed to lock database
pub(crate) fn unlock_database_query() -> &'static str {
    "SELECT RELEASE_LOCK(?)"
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
        fetch_rows(&self.pool).await
    }

    async fn lock(
        &self,
        connection: &mut <MySql as sqlx::Database>::Connection,
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
        connection: &mut <MySql as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        let lock_id = lock_id(&self.pool).await?;
        sqlx::query(unlock_database_query())
            .bind(lock_id)
            .execute(connection)
            .await?;
        Ok(())
    }
}

impl Migrate<MySql> for Migrator<MySql> {}
