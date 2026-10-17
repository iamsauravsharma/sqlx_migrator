//! Backend agnostic scenarios and the `Harness` each backend implements to
//! run them.

use std::sync::Arc;
use std::time::Duration;

use sqlx::{AssertSqlSafe, Database};
use sqlx_migrator::error::Error;
use sqlx_migrator::migration::Migration;
use sqlx_migrator::migrator::{
    DatabaseOperation as _, Info as _, LockStrategy, Migrate, Migrator, Plan,
};
use sqlx_migrator::operation::Operation;
use sqlx_migrator::sync::{OldMigrator, Synchronize as _};

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
}

impl SqlMigration {
    pub(crate) fn new(app: &str, name: &str) -> Self {
        Self {
            app: app.to_string(),
            name: name.to_string(),
            parents: Vec::new(),
            operations: Vec::new(),
        }
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
pub(crate) fn tables(test: &str) -> [String; 4] {
    [
        format!("{test}_first"),
        format!("{test}_second"),
        format!("{test}_failed"),
        format!("_{test}_sqlx_migrator_migrations"),
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
    type Db: Database<Arguments: sqlx::IntoArguments<Self::Db>, Connection: Send>;

    /// Connects and removes everything a previous run of `test` left behind.
    async fn setup(&self, test: &str) -> <Self::Db as Database>::Connection;

    /// Opens another connection to the same database.
    async fn connect(&self, test: &str) -> <Self::Db as Database>::Connection;

    /// Removes files or other state once a scenario is done.
    fn teardown(&self, _test: &str) {}

    /// Migrator whose migration table is unique to `test`.
    fn migrator(&self, test: &str) -> Migrator<Self::Db>;

    /// Migration table name `migrator` is expected to use.
    fn table_name(&self, test: &str) -> String {
        format!("_{test}_sqlx_migrator_migrations")
    }

    /// Whether `lock` makes a second connection wait.
    fn locks(&self) -> bool;

    /// Whether DDL is rolled back with the transaction.
    fn transactional_ddl(&self) -> bool;
}

async fn table_exists<DB>(connection: &mut DB::Connection, table: &str) -> bool
where
    DB: Database<Arguments: sqlx::IntoArguments<DB>>,
    for<'c> &'c mut DB::Connection: sqlx::Executor<'c, Database = DB>,
{
    sqlx::query(AssertSqlSafe(format!("SELECT 1 FROM {table} WHERE 1 = 0")))
        .execute(connection)
        .await
        .is_ok()
}

async fn applied<DB>(connection: &mut DB::Connection, migrator: &Migrator<DB>) -> Vec<String>
where
    DB: Database,
    Migrator<DB>: Migrate<DB>,
{
    migrator
        .fetch_applied_migration_from_db(connection)
        .await
        .unwrap()
        .iter()
        .map(|row| row.name().to_string())
        .collect()
}

/// Applies the chain, checks the ledger from a second connection, reverts
/// everything and drops the migration table.
pub(crate) async fn apply_and_revert<H>(harness: &H, test: &str)
where
    H: Harness,
    (String, String): Operation<H::Db>,
    Migrator<H::Db>: Migrate<H::Db>,
    for<'c> &'c mut <H::Db as Database>::Connection: sqlx::Executor<'c, Database = H::Db>,
{
    let mut connection = harness.setup(test).await;
    let migrator = harness.migrator(test);
    assert_eq!(migrator.table_name(), harness.table_name(test));

    let report = migrator
        .run(&mut connection, &Plan::apply_all())
        .await
        .unwrap();
    assert_eq!(report.len(), 2);
    assert!(table_exists::<H::Db>(&mut connection, &format!("{test}_second")).await);
    assert_eq!(
        applied(&mut connection, &migrator).await,
        vec!["first", "second"]
    );
    let rows = migrator
        .fetch_applied_migration_from_db(&mut connection)
        .await
        .unwrap();
    assert!(rows[0].id() < rows[1].id());
    assert_ne!(rows[0].applied_time(), "");
    migrator.check_pending(&mut connection).await.unwrap();

    let mut other = harness.connect(test).await;
    assert_eq!(
        applied(&mut other, &migrator).await,
        vec!["first", "second"]
    );
    let report = migrator.run(&mut other, &Plan::revert_all()).await.unwrap();
    assert_eq!(report.len(), 2);
    assert!(!table_exists::<H::Db>(&mut other, &format!("{test}_first")).await);
    assert_eq!(applied(&mut other, &migrator).await, Vec::<String>::new());
    assert!(matches!(
        migrator.check_pending(&mut other).await,
        Err(Error::PendingMigrationPresent)
    ));

    migrator
        .drop_migration_table_if_exists(&mut other)
        .await
        .unwrap();
    assert!(!table_exists::<H::Db>(&mut other, &migrator.table_name()).await);
    harness.teardown(test);
}

/// A fake run records rows without executing operations.
pub(crate) async fn fake_apply<H>(harness: &H, test: &str)
where
    H: Harness,
    (String, String): Operation<H::Db>,
    Migrator<H::Db>: Migrate<H::Db>,
    for<'c> &'c mut <H::Db as Database>::Connection: sqlx::Executor<'c, Database = H::Db>,
{
    let mut connection = harness.setup(test).await;
    let migrator = harness.migrator(test);
    let report = migrator
        .run(&mut connection, &Plan::apply_all().fake(true))
        .await
        .unwrap();
    assert!(report.is_fake());
    assert_eq!(
        applied(&mut connection, &migrator).await,
        vec!["first", "second"]
    );
    assert!(!table_exists::<H::Db>(&mut connection, &format!("{test}_first")).await);
    harness.teardown(test);
}

/// A failing atomic migration never records a ledger row. Whether its DDL
/// survives depends on the backend, which is what
/// `supports_transactional_ddl` reports.
pub(crate) async fn failed_atomic_migration<H>(harness: &H, test: &str)
where
    H: Harness,
    (String, String): Operation<H::Db>,
    Migrator<H::Db>: Migrate<H::Db>,
    for<'c> &'c mut <H::Db as Database>::Connection: sqlx::Executor<'c, Database = H::Db>,
{
    let mut connection = harness.setup(test).await;
    let migrator = harness.migrator(test);
    let mut migrator = migrator;
    migrator
        .add_migration(Box::new(
            SqlMigration::new("main", "failing")
                .parent("main", "second")
                .operation(
                    &format!("CREATE TABLE {test}_failed (id INTEGER PRIMARY KEY)"),
                    &format!("DROP TABLE {test}_failed"),
                )
                .operation("THIS IS NOT SQL", "SELECT 1"),
        ))
        .unwrap();
    assert!(matches!(
        migrator.run(&mut connection, &Plan::apply_all()).await,
        Err(Error::Sqlx(_))
    ));
    assert_eq!(
        table_exists::<H::Db>(&mut connection, &format!("{test}_failed")).await,
        !harness.transactional_ddl()
    );
    assert_eq!(
        applied(&mut connection, &migrator).await,
        vec!["first", "second"]
    );
    assert_eq!(
        migrator.supports_transactional_ddl(&connection),
        harness.transactional_ddl()
    );
    harness.teardown(test);
}

/// With the database strategy a second connection waits for the lock on
/// backends which lock, and never waits with `LockStrategy::None`.
pub(crate) async fn lock<H>(harness: &H, test: &str)
where
    H: Harness,
    (String, String): Operation<H::Db>,
    Migrator<H::Db>: Migrate<H::Db>,
    for<'c> &'c mut <H::Db as Database>::Connection: sqlx::Executor<'c, Database = H::Db>,
{
    let mut connection = harness.setup(test).await;
    let migrator = harness.migrator(test);
    let migrator = Arc::new(migrator);
    let mut second = harness.connect(test).await;
    migrator.lock(&mut connection).await.unwrap();
    if harness.locks() {
        let waiting = {
            let migrator = Arc::clone(&migrator);
            tokio::spawn(async move {
                migrator.lock(&mut second).await.unwrap();
                migrator.unlock(&mut second).await.unwrap();
            })
        };
        tokio::time::sleep(Duration::from_secs(1)).await;
        assert!(!waiting.is_finished(), "second lock should wait");
        migrator.unlock(&mut connection).await.unwrap();
        tokio::time::timeout(Duration::from_secs(10), waiting)
            .await
            .expect("second lock should proceed once the first is released")
            .unwrap();
    } else {
        tokio::time::timeout(Duration::from_secs(5), migrator.lock(&mut second))
            .await
            .expect("lock should not block")
            .unwrap();
        migrator.unlock(&mut second).await.unwrap();
        migrator.unlock(&mut connection).await.unwrap();
    }

    let migrator = harness.migrator(test).set_lock_strategy(LockStrategy::None);
    let mut second = harness.connect(test).await;
    migrator.lock(&mut connection).await.unwrap();
    tokio::time::timeout(Duration::from_secs(5), migrator.lock(&mut second))
        .await
        .expect("lock should not block")
        .unwrap();
    migrator.unlock(&mut second).await.unwrap();
    migrator.unlock(&mut connection).await.unwrap();
    harness.teardown(test);
}

/// Two runs racing on separate connections apply every migration exactly
/// once. Backends which lock serialise the runs so both succeed; without a
/// lock one run may fail on what the other already created, but atomic
/// migrations keep the database consistent.
pub(crate) async fn concurrent_runs<H>(harness: &H, test: &str)
where
    H: Harness,
    (String, String): Operation<H::Db>,
    Migrator<H::Db>: Migrate<H::Db>,
    for<'c> &'c mut <H::Db as Database>::Connection: sqlx::Executor<'c, Database = H::Db>,
{
    let mut connection = harness.setup(test).await;
    let migrator = harness.migrator(test);
    let migrator = Arc::new(migrator);
    let mut second = harness.connect(test).await;
    // both runs must find the migration table, otherwise they race on
    // creating it
    migrator
        .ensure_migration_table_exists(&mut connection)
        .await
        .unwrap();
    let a = {
        let migrator = Arc::clone(&migrator);
        tokio::spawn(async move { migrator.run(&mut connection, &Plan::apply_all()).await })
    };
    let b = {
        let migrator = Arc::clone(&migrator);
        tokio::spawn(async move { migrator.run(&mut second, &Plan::apply_all()).await })
    };
    let results = [a.await.unwrap(), b.await.unwrap()];
    if harness.locks() {
        let lengths = results
            .iter()
            .map(|r| r.as_ref().unwrap().len())
            .collect::<Vec<_>>();
        assert_eq!(lengths.iter().sum::<usize>(), 2, "{lengths:?}");
    } else {
        assert!(results.iter().any(Result::is_ok));
    }

    let mut check = harness.connect(test).await;
    assert_eq!(
        applied(&mut check, &migrator).await,
        vec!["first", "second"]
    );
    assert!(table_exists::<H::Db>(&mut check, &format!("{test}_first")).await);
    assert!(table_exists::<H::Db>(&mut check, &format!("{test}_second")).await);
    harness.teardown(test);
}

/// Syncing from an old migrator records only migrations the new migrator
/// knows and is idempotent.
pub(crate) async fn sync<H>(harness: &H, test: &str)
where
    H: Harness,
    (String, String): Operation<H::Db>,
    Migrator<H::Db>: Migrate<H::Db>,
    for<'c> &'c mut <H::Db as Database>::Connection: sqlx::Executor<'c, Database = H::Db>,
{
    let mut connection = harness.setup(test).await;
    let migrator = harness.migrator(test);
    migrator
        .ensure_migration_table_exists(&mut connection)
        .await
        .unwrap();
    let old = OldMigratorStub(vec![
        ("main".to_string(), "first".to_string()),
        ("main".to_string(), "unknown".to_string()),
    ]);
    migrator.sync(&mut connection, &old).await.unwrap();
    migrator.sync(&mut connection, &old).await.unwrap();
    assert_eq!(applied(&mut connection, &migrator).await, vec!["first"]);
    let report = migrator
        .run(&mut connection, &Plan::apply_all().fake(true))
        .await
        .unwrap();
    assert_eq!(report.len(), 1);
    assert_eq!(report.records()[0].name(), "second");
    harness.teardown(test);
}

/// Prune removes ledger rows without a matching migration and nothing
/// else, and the ledger rejects duplicate rows.
pub(crate) async fn prune<H>(harness: &H, test: &str)
where
    H: Harness,
    (String, String): Operation<H::Db>,
    Migrator<H::Db>: Migrate<H::Db>,
    for<'c> &'c mut <H::Db as Database>::Connection: sqlx::Executor<'c, Database = H::Db>,
{
    let mut connection = harness.setup(test).await;
    let migrator = harness.migrator(test);
    migrator
        .run(&mut connection, &Plan::apply_all().fake(true))
        .await
        .unwrap();
    migrator
        .add_migration_to_db_table(&mut connection, &("ghost", "removed"))
        .await
        .unwrap();
    assert_eq!(migrator.prune(&mut connection).await.unwrap(), 1);
    assert_eq!(migrator.prune(&mut connection).await.unwrap(), 0);
    assert_eq!(
        applied(&mut connection, &migrator).await,
        vec!["first", "second"]
    );
    assert!(matches!(
        migrator
            .add_migration_to_db_table(&mut connection, &("main", "first"))
            .await,
        Err(Error::Sqlx(_))
    ));
    harness.teardown(test);
}

/// Declares one test per scenario, running it against every harness in
/// `$harnesses` and prefixing test names (and so table names) with `$prefix`.
/// With `ignore = "reason"` the tests are marked `#[ignore]` and only run
/// with `--include-ignored`, for backends which need an external database.
macro_rules! backend_tests {
    ($harnesses:expr, $prefix:literal) => {
        crate::common::backend_tests!(@tests $harnesses, $prefix, );
    };
    ($harnesses:expr, $prefix:literal, ignore = $reason:literal) => {
        crate::common::backend_tests!(@tests $harnesses, $prefix, #[ignore = $reason]);
    };
    (@tests $harnesses:expr, $prefix:literal, $(#[$ignore:meta])?) => {
        $(#[$ignore])?
        #[tokio::test]
        async fn apply_and_revert() {
            for harness in $harnesses {
                crate::common::apply_and_revert(&harness, concat!($prefix, "_apply")).await;
            }
        }

        $(#[$ignore])?
        #[tokio::test]
        async fn fake_apply() {
            for harness in $harnesses {
                crate::common::fake_apply(&harness, concat!($prefix, "_fake")).await;
            }
        }

        $(#[$ignore])?
        #[tokio::test]
        async fn failed_atomic_migration() {
            for harness in $harnesses {
                crate::common::failed_atomic_migration(&harness, concat!($prefix, "_failed"))
                    .await;
            }
        }

        $(#[$ignore])?
        #[tokio::test(flavor = "multi_thread")]
        async fn lock() {
            for harness in $harnesses {
                crate::common::lock(&harness, concat!($prefix, "_lock")).await;
            }
        }

        $(#[$ignore])?
        #[tokio::test(flavor = "multi_thread")]
        async fn concurrent_runs() {
            for harness in $harnesses {
                crate::common::concurrent_runs(&harness, concat!($prefix, "_concurrent")).await;
            }
        }

        $(#[$ignore])?
        #[tokio::test]
        async fn sync() {
            for harness in $harnesses {
                crate::common::sync(&harness, concat!($prefix, "_sync")).await;
            }
        }

        $(#[$ignore])?
        #[tokio::test]
        async fn prune() {
            for harness in $harnesses {
                crate::common::prune(&harness, concat!($prefix, "_prune")).await;
            }
        }
    };
}

pub(crate) use backend_tests;
