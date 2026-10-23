//! Backend agnostic scenarios and the `Harness` each backend implements to
//! run them.

use std::sync::Arc;
use std::time::Duration;

use sqlx::Database;
use sqlx_migrator::error::Error;
use sqlx_migrator::migration::Migration;
use sqlx_migrator::migrator::{Migrator, Plan, SupportedDatabase};
use sqlx_migrator::operation::Operation;
use sqlx_migrator::sync::OldMigrator;
use time::OffsetDateTime;

mod basic;
mod ledger;
mod locking;

pub(crate) use self::basic::*;
pub(crate) use self::ledger::*;
pub(crate) use self::locking::*;

/// Returns the connection url from `variable`. Tests needing one are marked
/// `#[ignore]` and only run with `--include-ignored`, so a missing variable is
/// a configuration error rather than a reason to skip silently.
#[expect(clippy::panic, reason = "a missing url is a test setup error")]
pub(crate) fn database_url(variable: &str) -> String {
    std::env::var(variable)
        .ok()
        .filter(|url| !url.is_empty())
        .unwrap_or_else(|| panic!("{variable} must be set to run this test"))
}

/// Migration defined by plain SQL strings so each test can use its own table
/// names and stay isolated from tests running in parallel.
pub(crate) struct SqlMigration {
    app: String,
    name: String,
    parents: Vec<(String, String)>,
    operations: Vec<(String, String)>,
    atomic: bool,
}

impl SqlMigration {
    pub(crate) fn new(app: &str, name: &str) -> Self {
        Self {
            app: app.to_string(),
            name: name.to_string(),
            parents: Vec::new(),
            operations: Vec::new(),
            atomic: true,
        }
    }

    pub(crate) fn non_atomic(mut self) -> Self {
        self.atomic = false;
        self
    }

    pub(crate) fn parent(mut self, app: &str, name: &str) -> Self {
        self.parents.push((app.to_string(), name.to_string()));
        self
    }

    pub(crate) fn operation(mut self, up: &str, down: &str) -> Self {
        self.operations.push((up.to_string(), down.to_string()));
        self
    }
}

impl<DB> Migration<DB> for SqlMigration
where
    DB: Database,
    (String, String): Operation<DB>,
{
    fn app(&self) -> &str {
        &self.app
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn parents(&self) -> Vec<Box<dyn Migration<DB>>> {
        self.parents
            .iter()
            .map(|(app, name)| Box::new((app.clone(), name.clone())) as Box<dyn Migration<DB>>)
            .collect()
    }

    fn operations(&self) -> Vec<Box<dyn Operation<DB>>> {
        self.operations
            .iter()
            .map(|(up, down)| Box::new((up.clone(), down.clone())) as Box<dyn Operation<DB>>)
            .collect()
    }

    fn is_atomic(&self) -> bool {
        self.atomic
    }
}

/// Two migration chain creating `{test}_first` and `{test}_second` tables.
pub(crate) fn chain<DB>(test: &str) -> Vec<Box<dyn Migration<DB>>>
where
    DB: Database,
    (String, String): Operation<DB>,
{
    vec![
        Box::new(SqlMigration::new("main", "first").operation(
            &format!("CREATE TABLE {test}_first (id INTEGER PRIMARY KEY)"),
            &format!("DROP TABLE {test}_first"),
        )),
        Box::new(
            SqlMigration::new("main", "second")
                .parent("main", "first")
                .operation(
                    &format!("CREATE TABLE {test}_second (id INTEGER PRIMARY KEY)"),
                    &format!("DROP TABLE {test}_second"),
                ),
        ),
    ]
}

/// Tables a test may leave behind, to be dropped by `Harness::setup`.
pub(crate) fn tables(test: &str) -> [String; 6] {
    [
        format!("{test}_first"),
        format!("{test}_second"),
        format!("{test}_failed"),
        format!("_{test}_sqlx_migrator_migrations"),
        format!("_{test}_sqlx_migrator_meta"),
        format!("_{test}_sqlx_migrator_lock"),
    ]
}

/// Old migrator stub reporting a fixed list of applied migrations
pub(crate) struct OldMigratorStub(pub(crate) Vec<(String, String)>);

#[async_trait::async_trait]
impl<DB> OldMigrator<DB> for OldMigratorStub
where
    DB: Database,
{
    async fn applied_migrations(
        &self,
        _connection: &mut <DB as Database>::Connection,
    ) -> Result<Vec<Box<dyn Migration<DB>>>, Error> {
        Ok(self
            .0
            .iter()
            .map(|(app, name)| Box::new((app.clone(), name.clone())) as Box<dyn Migration<DB>>)
            .collect())
    }
}

/// Path of a `SQLite` database file for `test`, unique per process so parallel
/// test binaries never share a file.
#[cfg(feature = "sqlite")]
pub(crate) fn sqlite_path(test: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "sqlx_migrator_{test}_{}.sqlite3",
        std::process::id()
    ))
}

/// What a backend has to provide for the shared scenarios to run against it.
#[async_trait::async_trait]
pub(crate) trait Harness: Send + Sync + 'static {
    type Db: SupportedDatabase;

    /// Connects and removes everything a previous run of `test` left behind.
    async fn setup(&self, test: &str) -> <Self::Db as Database>::Connection;

    /// Opens another connection to the same database.
    async fn connect(&self, test: &str) -> <Self::Db as Database>::Connection;

    /// Removes files or other state once a scenario is done.
    fn teardown(&self, _test: &str) {}

    /// Migrator whose migration table is unique to `test`.
    fn migrator(&self, test: &str) -> Migrator<Self::Db>;

    /// Migrator like `migrator` which locks with a row in a lock table.
    fn row_lock_migrator(&self, test: &str) -> Migrator<Self::Db>;

    /// Migration table name `migrator` is expected to use.
    fn table_name(&self, test: &str) -> String {
        format!("_{test}_sqlx_migrator_migrations")
    }

    /// Statement which creates the migration table with only the columns of
    /// the first internal migration.
    fn legacy_table_sql(&self, table: &str) -> String;

    /// Statements which take and release the advisory lock, binding the
    /// migration table name, `None` when the backend does not lock.
    fn lock_statements(&self) -> Option<(&'static str, &'static str)>;

    /// Whether DDL is rolled back with the transaction.
    fn transactional_ddl(&self) -> bool;
}

pub(crate) async fn table_exists<DB: SupportedDatabase>(
    connection: &mut DB::Connection,
    table: &str,
) -> bool {
    DB::execute(connection, &format!("SELECT 1 FROM {table} WHERE 1 = 0"))
        .await
        .is_ok()
}

/// Inserts a migration table row directly, bypassing the migrator
async fn insert_row<DB: SupportedDatabase>(
    connection: &mut DB::Connection,
    table: &str,
    app: &str,
    name: &str,
) -> Result<(), Error> {
    DB::execute(
        connection,
        &format!("INSERT INTO {table}(app, name) VALUES ('{app}', '{name}')"),
    )
    .await
}

pub(crate) async fn applied<DB>(
    connection: &mut DB::Connection,
    migrator: &Migrator<DB>,
) -> Vec<String>
where
    DB: SupportedDatabase,
{
    migrator
        .fetch_applied_migration_from_db(connection)
        .await
        .unwrap()
        .iter()
        .map(|row| row.name().to_string())
        .collect()
}

/// Declares one test per scenario, running it against `$harness` and
/// prefixing test names (and so table names) with `$prefix`.
/// With `ignore = "reason"` the tests are marked `#[ignore]` and only run
/// with `--include-ignored`, for backends which need an external database.
macro_rules! backend_tests {
    ($harness:expr, $prefix:literal) => {
        crate::common::backend_tests!(@tests $harness, $prefix, );
    };
    ($harness:expr, $prefix:literal, ignore = $reason:literal) => {
        crate::common::backend_tests!(@tests $harness, $prefix, #[ignore = $reason]);
    };
    (@tests $harness:expr, $prefix:literal, $(#[$ignore:meta])?) => {
        $(#[$ignore])?
        #[tokio::test]
        async fn apply_and_revert() {
            crate::common::apply_and_revert(&$harness, concat!($prefix, "_apply")).await;
        }

        $(#[$ignore])?
        #[tokio::test]
        async fn fake_apply() {
            crate::common::fake_apply(&$harness, concat!($prefix, "_fake")).await;
        }

        $(#[$ignore])?
        #[tokio::test]
        async fn failed_atomic_migration() {
            crate::common::failed_atomic_migration(&$harness, concat!($prefix, "_failed")).await;
        }

        $(#[$ignore])?
        #[tokio::test]
        async fn grouped_plan() {
            crate::common::grouped_plan(&$harness, concat!($prefix, "_grouped")).await;
        }

        $(#[$ignore])?
        #[tokio::test(flavor = "multi_thread")]
        async fn lock() {
            crate::common::lock(&$harness, concat!($prefix, "_lock")).await;
        }

        $(#[$ignore])?
        #[tokio::test(flavor = "multi_thread")]
        async fn row_lock() {
            crate::common::row_lock(&$harness, concat!($prefix, "_rowlock")).await;
        }

        $(#[$ignore])?
        #[tokio::test(flavor = "multi_thread")]
        async fn concurrent_runs() {
            crate::common::concurrent_runs(&$harness, concat!($prefix, "_concurrent")).await;
        }

        $(#[$ignore])?
        #[tokio::test]
        async fn sync() {
            crate::common::sync(&$harness, concat!($prefix, "_sync")).await;
        }

        $(#[$ignore])?
        #[tokio::test]
        async fn prune() {
            crate::common::prune(&$harness, concat!($prefix, "_prune")).await;
        }

        $(#[$ignore])?
        #[tokio::test]
        async fn failed_migration() {
            crate::common::failed_migration(&$harness, concat!($prefix, "_ledger")).await;
        }

        $(#[$ignore])?
        #[tokio::test]
        async fn internal_migrations() {
            crate::common::internal_migrations(&$harness, concat!($prefix, "_tables")).await;
        }
    };
}

pub(crate) use backend_tests;
