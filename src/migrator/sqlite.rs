use sqlx::Sqlite;

use super::{DatabaseOperation, Migrator};
use crate::error::Error;
use crate::migration::{AppliedMigrationSqlRow, Migration};

/// create migrator table
#[must_use]
pub(crate) fn create_migrator_table_query(table_name: &str) -> String {
    format!(
        "CREATE TABLE IF NOT EXISTS {table_name} (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        app TEXT NOT NULL,
        name TEXT NOT NULL,
        applied_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        UNIQUE (app, name)
    )"
    )
}

/// Drop table
#[must_use]
pub(crate) fn drop_table_query(table_name: &str) -> String {
    format!("DROP TABLE IF EXISTS {table_name}")
}

/// fetch rows
pub(crate) fn fetch_rows_query(table_name: &str) -> String {
    format!("SELECT id, app, name, applied_time FROM {table_name}")
}

/// add migration query
#[must_use]
pub(crate) fn add_migration_query(table_name: &str) -> String {
    format!("INSERT INTO {table_name}(app, name) VALUES ($1, $2)")
}

/// delete migration query
#[must_use]
pub(crate) fn delete_migration_query(table_name: &str) -> String {
    format!("DELETE FROM {table_name} WHERE app = $1 AND name = $2")
}

#[async_trait::async_trait]
impl<State> DatabaseOperation<Sqlite, State> for Migrator<Sqlite, State>
where
    State: Send + Sync,
{
    async fn ensure_migration_table_exists(
        &self,
        connection: &mut <Sqlite as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        sqlx::query(&create_migrator_table_query(self.table_name()))
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn drop_migration_table_if_exists(
        &self,
        connection: &mut <Sqlite as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        sqlx::query(&drop_table_query(self.table_name()))
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn add_migration_to_db_table(
        &self,
        connection: &mut <Sqlite as sqlx::Database>::Connection,
        migration: &Box<dyn Migration<Sqlite, State>>,
    ) -> Result<(), Error> {
        sqlx::query(&add_migration_query(self.table_name()))
            .bind(migration.app())
            .bind(migration.name())
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn delete_migration_from_db_table(
        &self,
        connection: &mut <Sqlite as sqlx::Database>::Connection,
        migration: &Box<dyn Migration<Sqlite, State>>,
    ) -> Result<(), Error> {
        sqlx::query(&delete_migration_query(self.table_name()))
            .bind(migration.app())
            .bind(migration.name())
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn fetch_applied_migration_from_db(
        &self,
        connection: &mut <Sqlite as sqlx::Database>::Connection,
    ) -> Result<Vec<AppliedMigrationSqlRow>, Error> {
        Ok(
            sqlx::query_as::<_, AppliedMigrationSqlRow>(&fetch_rows_query(self.table_name()))
                .fetch_all(connection)
                .await?,
        )
    }

    async fn lock(
        &self,
        _connection: &mut <Sqlite as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        Ok(())
    }

    async fn unlock(
        &self,
        _connection: &mut <Sqlite as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        Ok(())
    }
}
