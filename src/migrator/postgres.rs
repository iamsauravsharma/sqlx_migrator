use sqlx::Postgres;

use super::{DatabaseOperation, Migrator};
use crate::error::Error;
use crate::migration::{AppliedMigrationSqlRow, Migration};

/// Create migrator table query
#[must_use]
pub(crate) fn create_migrator_table_query(table_name: &str) -> String {
    format!(
        "CREATE TABLE IF NOT EXISTS {table_name} (
        id INT PRIMARY KEY NOT NULL GENERATED ALWAYS AS IDENTITY,
        app TEXT NOT NULL,
        name TEXT NOT NULL,
        applied_time TIMESTAMPTZ NOT NULL DEFAULT now(),
        UNIQUE (app, name)
    )"
    )
}

/// Drop table query
#[must_use]
pub(crate) fn drop_table_query(table_name: &str) -> String {
    format!("DROP TABLE IF EXISTS {table_name}")
}

/// Fetch rows
pub(crate) fn fetch_rows_query(table_name: &str) -> String {
    format!("SELECT id, app, name, applied_time::TEXT FROM {table_name}")
}

/// Add migration query
#[must_use]
pub(crate) fn add_migration_query(table_name: &str) -> String {
    format!("INSERT INTO {table_name}(app, name) VALUES ($1, $2)")
}

/// Delete migration query
#[must_use]
pub(crate) fn delete_migration_query(table_name: &str) -> String {
    format!("DELETE FROM {table_name} WHERE app = $1 AND name = $2")
}

/// get current database query
pub(crate) fn current_database_query() -> &'static str {
    "SELECT CURRENT_DATABASE()"
}

/// get lock database query
pub(crate) fn lock_database_query() -> &'static str {
    "SELECT pg_advisory_lock($1)"
}

/// get lock database query
pub(crate) fn unlock_database_query() -> &'static str {
    "SELECT pg_advisory_unlock($1)"
}

/// generate lock id
pub(crate) fn get_lock_id(database_name: &str, table_name: &str) -> i64 {
    let buf = format!("{database_name}/{table_name}");
    i64::from(crc32fast::hash(buf.as_bytes()))
}

#[async_trait::async_trait]
impl DatabaseOperation<Postgres> for Migrator<Postgres> {
    async fn ensure_migration_table_exists(
        &self,
        connection: &mut <Postgres as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        sqlx::query(&create_migrator_table_query(self.table_name()))
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn drop_migration_table_if_exists(
        &self,
        connection: &mut <Postgres as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        sqlx::query(&drop_table_query(self.table_name()))
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn add_migration_to_db_table(
        &self,
        connection: &mut <Postgres as sqlx::Database>::Connection,
        migration: &Box<dyn Migration<Postgres>>,
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
        connection: &mut <Postgres as sqlx::Database>::Connection,
        migration: &Box<dyn Migration<Postgres>>,
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
        connection: &mut <Postgres as sqlx::Database>::Connection,
    ) -> Result<Vec<AppliedMigrationSqlRow>, Error> {
        Ok(
            sqlx::query_as::<_, AppliedMigrationSqlRow>(&fetch_rows_query(self.table_name()))
                .fetch_all(connection)
                .await?,
        )
    }

    async fn lock(
        &self,
        connection: &mut <Postgres as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        let (database_name,): (String,) = sqlx::query_as(current_database_query())
            .fetch_one(&mut *connection)
            .await?;
        let lock_id = get_lock_id(&database_name, self.table_name());
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
        let (database_name,): (String,) = sqlx::query_as(current_database_query())
            .fetch_one(&mut *connection)
            .await?;
        let lock_id = get_lock_id(&database_name, self.table_name());
        sqlx::query(unlock_database_query())
            .bind(lock_id)
            .execute(connection)
            .await?;
        Ok(())
    }
}
