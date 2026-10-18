use sqlx::Sqlite;

use super::dialect::{Names, Sql};
use super::internal_migration::InternalMigrations;
use super::{Dialect, Migrator};

/// Internal migrations in order, see [`super::internal_migration`]
fn internal_migration_steps(table_name: &str) -> Vec<(&'static str, String)> {
    vec![
        (
            "0001_create_migrations_table",
            create_migrations_table(table_name),
        ),
        (
            "0002_add_success_column",
            format!("ALTER TABLE {table_name} ADD COLUMN success BOOLEAN NOT NULL DEFAULT TRUE"),
        ),
        (
            "0003_add_execution_time_column",
            format!(
                "ALTER TABLE {table_name} ADD COLUMN execution_time_ms BIGINT NOT NULL DEFAULT 0"
            ),
        ),
        (
            "0004_add_batch_column",
            format!(
                "ALTER TABLE {table_name} ADD COLUMN batch INT NOT NULL DEFAULT 0; UPDATE \
                 {table_name} SET batch = id"
            ),
        ),
    ]
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
fn internal_migrations(table_name: &str, meta_table_name: &str) -> InternalMigrations {
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

/// Drop table query
#[must_use]
fn drop_table_query(table_name: &str) -> String {
    format!("DROP TABLE IF EXISTS {table_name}")
}

/// Fetch rows query
#[must_use]
fn fetch_rows_query(table_name: &str) -> String {
    format!(
        "SELECT id, app, name, CAST(applied_time AS TEXT) AS applied_time, success, \
         execution_time_ms, batch FROM {table_name}"
    )
}

/// Add migration query
#[must_use]
fn add_migration_query(table_name: &str) -> String {
    format!(
        "INSERT INTO {table_name}(app, name, execution_time_ms, batch) VALUES ($1, $2, CAST($3 AS \
         INTEGER), CAST($4 AS INTEGER))"
    )
}

/// Start migration query, records a migration which is running as failed
#[must_use]
fn start_migration_query(table_name: &str) -> String {
    format!(
        "INSERT INTO {table_name}(app, name, success, batch) VALUES ($1, $2, FALSE, CAST($3 AS \
         INTEGER))"
    )
}

/// Finish migration query, marks a running migration as successful
#[must_use]
fn finish_migration_query(table_name: &str) -> String {
    format!(
        "UPDATE {table_name} SET success = TRUE, execution_time_ms = CAST($1 AS INTEGER) WHERE \
         app = $2 AND name = $3"
    )
}

/// Delete migration query
#[must_use]
fn delete_migration_query(table_name: &str) -> String {
    format!("DELETE FROM {table_name} WHERE app = $1 AND name = $2")
}

/// Delete failed migrations query
#[must_use]
fn delete_failed_migrations_query(table_name: &str) -> String {
    format!("DELETE FROM {table_name} WHERE success = FALSE")
}

/// Built in SQL, see [`Dialect::sqlite`]
fn sql(names: &Names) -> Sql {
    Sql {
        create_schema: None,
        internal: internal_migrations(&names.table(), &names.meta_table()),
        drop_tables: format!(
            "{}; {}",
            drop_table_query(&names.table()),
            drop_table_query(&names.meta_table())
        ),
        fetch_rows: fetch_rows_query(&names.table()),
        add_migration: add_migration_query(&names.table()),
        start_migration: start_migration_query(&names.table()),
        finish_migration: finish_migration_query(&names.table()),
        delete_migration: delete_migration_query(&names.table()),
        delete_failed_migrations: delete_failed_migrations_query(&names.table()),
        lock: None,
    }
}

/// Built in dialect, see [`Dialect::sqlite`]
pub(crate) fn dialect(names: Names) -> Dialect<Sqlite> {
    Dialect::new(names, sql)
}

impl Default for Migrator<Sqlite> {
    /// Creates a migrator with the built in dialect, see [`Dialect::sqlite`]
    fn default() -> Self {
        Self::new(Dialect::sqlite())
    }
}
