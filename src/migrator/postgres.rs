use sqlx::Postgres;

use super::dialect::{Lock, Names, Sql};
use super::internal_migration::InternalMigrations;
use super::{Dialect, Migrator};

/// Create schema query
#[must_use]
fn create_schema_query(schema_name: &str) -> String {
    format!("CREATE SCHEMA IF NOT EXISTS {schema_name}")
}

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
        id INT PRIMARY KEY NOT NULL GENERATED ALWAYS AS IDENTITY,
        app TEXT NOT NULL,
        name TEXT NOT NULL,
        applied_time TIMESTAMPTZ NOT NULL DEFAULT now(),
        UNIQUE (app, name)
    )"
    )
}

/// Internal migrations for the given table names
fn internal_migrations(table_name: &str, meta_table_name: &str) -> InternalMigrations {
    let create_meta_table = format!(
        "CREATE TABLE IF NOT EXISTS {meta_table_name} (name TEXT PRIMARY KEY NOT NULL, \
         applied_time TIMESTAMPTZ NOT NULL DEFAULT now())"
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
    format!("SELECT id, app, name, applied_time::TEXT FROM {table_name}")
}

/// Add migration query
#[must_use]
fn add_migration_query(table_name: &str) -> String {
    format!("INSERT INTO {table_name}(app, name) VALUES ($1, $2)")
}

/// Delete migration query
#[must_use]
fn delete_migration_query(table_name: &str) -> String {
    format!("DELETE FROM {table_name} WHERE app = $1 AND name = $2")
}

/// Built in SQL, see [`Dialect::postgres`]
fn sql(names: &Names) -> Sql {
    // advisory locks are scoped to the connected database already
    let lock = Lock::new(
        "SELECT pg_advisory_lock(hashtext($1))",
        "SELECT pg_advisory_unlock(hashtext($1))",
    );
    Sql {
        create_schema: names.schema().map(create_schema_query),
        internal: internal_migrations(&names.table(), &names.meta_table()),
        drop_tables: format!(
            "{}; {}",
            drop_table_query(&names.table()),
            drop_table_query(&names.meta_table())
        ),
        fetch_rows: fetch_rows_query(&names.table()),
        add_migration: add_migration_query(&names.table()),
        delete_migration: delete_migration_query(&names.table()),
        lock: Some(lock),
    }
}

/// Built in dialect, see [`Dialect::postgres`]
pub(crate) fn dialect(names: Names) -> Dialect<Postgres> {
    Dialect::new(names, sql)
}

impl Default for Migrator<Postgres> {
    /// Creates a migrator with the built in dialect, see [`Dialect::postgres`]
    fn default() -> Self {
        Self::new(Dialect::postgres())
    }
}
