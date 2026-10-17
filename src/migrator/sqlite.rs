use sqlx::{AssertSqlSafe, Database, Sqlite};

use super::internal_migration::{self, InternalMigrations};
use super::{DatabaseOperation, Migrator};
use crate::error::Error;
use crate::migration::{AppliedMigrationSqlRow, Migration};

/// Internal migrations in order, see [`super::internal_migration`]
fn internal_migration_steps(table_name: &str) -> Vec<(&'static str, String)> {
    vec![(
        "0001_create_migrations_table",
        create_migrations_table(table_name),
    )]
}

fn create_migrations_table(table_name: &str) -> String {
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

/// Internal migrations for the given table names
pub(crate) fn internal_migrations(table_name: &str, meta_table_name: &str) -> InternalMigrations {
    let create_meta_table = format!(
        "CREATE TABLE IF NOT EXISTS {meta_table_name} (name TEXT PRIMARY KEY NOT NULL, \
         applied_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP)"
    );
    let fetch_applied = format!("SELECT name FROM {meta_table_name}");
    let insert_applied =
        format!("INSERT INTO {meta_table_name}(name) VALUES ($1) ON CONFLICT (name) DO NOTHING");
    InternalMigrations::new(
        create_meta_table,
        fetch_applied,
        insert_applied,
        internal_migration_steps(table_name),
    )
}

/// Drop table
#[must_use]
pub(crate) fn drop_table_query(table_name: &str) -> AssertSqlSafe<String> {
    AssertSqlSafe(format!("DROP TABLE IF EXISTS {table_name}"))
}

/// fetch rows
///
/// `applied_time` is cast to `TEXT` because the `Any` driver cannot decode the
/// `TIMESTAMP` column type the table declares.
#[must_use]
pub(crate) fn fetch_rows_query(table_name: &str) -> AssertSqlSafe<String> {
    AssertSqlSafe(format!(
        "SELECT id, app, name, CAST(applied_time AS TEXT) AS applied_time FROM {table_name}"
    ))
}

/// add migration query
#[must_use]
pub(crate) fn add_migration_query(table_name: &str) -> AssertSqlSafe<String> {
    AssertSqlSafe(format!(
        "INSERT INTO {table_name}(app, name) VALUES ($1, $2)"
    ))
}

/// delete migration query
#[must_use]
pub(crate) fn delete_migration_query(table_name: &str) -> AssertSqlSafe<String> {
    AssertSqlSafe(format!(
        "DELETE FROM {table_name} WHERE app = $1 AND name = $2"
    ))
}

#[async_trait::async_trait]
impl DatabaseOperation<Sqlite> for Migrator<Sqlite> {
    async fn ensure_migration_table_exists(
        &self,
        connection: &mut <Sqlite as Database>::Connection,
    ) -> Result<(), Error> {
        internal_migration::ensure_tables::<Sqlite>(
            connection,
            &internal_migrations(&self.table_name(), &self.meta_table_name()),
        )
        .await
    }

    async fn drop_migration_table_if_exists(
        &self,
        connection: &mut <Sqlite as Database>::Connection,
    ) -> Result<(), Error> {
        sqlx::query(drop_table_query(&self.table_name()))
            .execute(&mut *connection)
            .await?;
        sqlx::query(drop_table_query(&self.meta_table_name()))
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn add_migration_to_db_table(
        &self,
        connection: &mut <Sqlite as Database>::Connection,
        migration: &dyn Migration<Sqlite>,
    ) -> Result<(), Error> {
        sqlx::query(add_migration_query(&self.table_name()))
            .bind(migration.app())
            .bind(migration.name())
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn delete_migration_from_db_table(
        &self,
        connection: &mut <Sqlite as Database>::Connection,
        migration: &dyn Migration<Sqlite>,
    ) -> Result<(), Error> {
        sqlx::query(delete_migration_query(&self.table_name()))
            .bind(migration.app())
            .bind(migration.name())
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn fetch_applied_migration_from_db(
        &self,
        connection: &mut <Sqlite as Database>::Connection,
    ) -> Result<Vec<AppliedMigrationSqlRow>, Error> {
        Ok(
            sqlx::query_as::<_, AppliedMigrationSqlRow>(fetch_rows_query(&self.table_name()))
                .fetch_all(connection)
                .await?,
        )
    }

    async fn lock(&self, _connection: &mut <Sqlite as Database>::Connection) -> Result<(), Error> {
        // SQLite serialises writes at the engine level. True advisory locking
        // is not available, so concurrent migrators against the same
        // file are not protected here. Use a single migrator instance
        // per database file.
        Ok(())
    }

    async fn unlock(
        &self,
        _connection: &mut <Sqlite as Database>::Connection,
    ) -> Result<(), Error> {
        Ok(())
    }
}
