use sqlx::Postgres;

use super::dialect::{Lock, Names, RowLock, Sql, TransactionLocks};
use super::internal_migration::InternalMigrations;
use super::{Dialect, Migrator};

/// Create schema query
#[must_use]
fn create_schema_query(schema_name: &str) -> String {
    format!("CREATE SCHEMA IF NOT EXISTS {schema_name}")
}

/// Internal migrations in order, see [`super::internal_migration`]
fn internal_migration_steps(table_name: &str) -> Vec<(&'static str, String)> {
    vec![
        (
            "0001_create_migrations_table",
            create_migrations_table(table_name),
        ),
        (
            "0002_add_success_column",
            format!(
                "ALTER TABLE {table_name} ADD COLUMN IF NOT EXISTS success BOOLEAN NOT NULL \
                 DEFAULT TRUE"
            ),
        ),
        (
            "0003_add_execution_time_column",
            format!(
                "ALTER TABLE {table_name} ADD COLUMN IF NOT EXISTS execution_time_ns BIGINT NOT \
                 NULL DEFAULT 0"
            ),
        ),
        (
            "0004_add_batch_column",
            format!(
                "ALTER TABLE {table_name} ADD COLUMN IF NOT EXISTS batch INT NOT NULL DEFAULT 0; \
                 UPDATE {table_name} SET batch = id"
            ),
        ),
    ]
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
    format!(
        "SELECT id, app, name, applied_time::TEXT, success, execution_time_ns, batch FROM \
         {table_name}"
    )
}

/// Add migration query
#[must_use]
fn add_migration_query(table_name: &str) -> String {
    format!(
        "INSERT INTO {table_name}(app, name, execution_time_ns, batch) VALUES ($1, $2, CAST($3 AS \
         BIGINT), CAST($4 AS INT))"
    )
}

/// Start migration query, records a migration which is running as failed
#[must_use]
fn start_migration_query(table_name: &str) -> String {
    format!(
        "INSERT INTO {table_name}(app, name, success, batch) VALUES ($1, $2, FALSE, CAST($3 AS \
         INT))"
    )
}

/// Finish migration query, marks a running migration as successful
#[must_use]
fn finish_migration_query(table_name: &str) -> String {
    format!(
        "UPDATE {table_name} SET success = TRUE, execution_time_ns = CAST($1 AS BIGINT) WHERE app \
         = $2 AND name = $3"
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

/// Row lock statements for the given lock table name
fn row_lock(lock_table: &str) -> RowLock {
    RowLock {
        create_table: format!(
            "CREATE TABLE IF NOT EXISTS {lock_table} (name TEXT PRIMARY KEY NOT NULL, locked_at \
             TIMESTAMPTZ NOT NULL DEFAULT now())"
        ),
        acquire: format!("INSERT INTO {lock_table}(name) VALUES ($1)"),
        release: format!("DELETE FROM {lock_table} WHERE name = $1"),
    }
}

/// Lock table query, holds the lock until the transaction ends
#[must_use]
fn lock_table_query(table_name: &str, mode: &str) -> String {
    format!("LOCK TABLE {table_name} IN {mode} MODE")
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
        start_migration: start_migration_query(&names.table()),
        finish_migration: finish_migration_query(&names.table()),
        delete_migration: delete_migration_query(&names.table()),
        delete_failed_migrations: delete_failed_migrations_query(&names.table()),
        lock: Some(lock),
        row_lock: row_lock(&names.lock_table()),
        transaction_locks: Some(TransactionLocks {
            advisory: "SELECT pg_advisory_xact_lock(hashtext($1))".to_string(),
            // reading a catalog table makes the session take in what was
            // committed while it waited
            after_lock: "SELECT 1 FROM pg_catalog.pg_namespace LIMIT 1".to_string(),
            share_update_exclusive: lock_table_query(&names.table(), "SHARE UPDATE EXCLUSIVE"),
            access_exclusive: lock_table_query(&names.table(), "ACCESS EXCLUSIVE"),
        }),
        drop_lock_table: drop_table_query(&names.lock_table()),
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
