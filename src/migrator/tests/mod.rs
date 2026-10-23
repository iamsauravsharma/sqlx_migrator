use std::ops::{Deref, DerefMut};

use sqlx::{Database, Sqlite, SqlitePool};
use time::OffsetDateTime;

use super::{Dialect, LockStrategy, MigrationVecResult, Migrator};
use crate::error::Error;
use crate::hook::{HookContext, HookFuture, HookType};
use crate::migration::{AppliedMigrationSqlRow, Migration};
use crate::migrator::Plan;
use crate::operation::Operation;
use crate::sync::OldMigrator;
use crate::vec_box;

/// Verifies that the table prefix is accepted when it holds lowercase,
/// digits and underscores and rejected otherwise (uppercase, special chars,
/// Migrator with an in memory list of applied rows, so the planner tests
/// run without a database.
#[derive(Default)]
struct TestMigrator {
    migrator: Migrator<Sqlite>,
    applied_migrations: Vec<AppliedMigrationSqlRow>,
}

impl Deref for TestMigrator {
    type Target = Migrator<Sqlite>;

    fn deref(&self) -> &Self::Target {
        &self.migrator
    }
}

impl DerefMut for TestMigrator {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.migrator
    }
}

impl TestMigrator {
    fn add_applied_migrations(&mut self, migrations: Vec<Box<dyn Migration<Sqlite>>>) {
        for migration in migrations {
            self.add_applied_migration(migration.as_ref());
        }
    }

    fn add_applied_migration(&mut self, migration: &dyn Migration<Sqlite>) {
        let id = i32::try_from(self.applied_migrations.len()).unwrap();
        self.applied_migrations.push(AppliedMigrationSqlRow::new(
            id,
            migration.app(),
            migration.name(),
        ));
    }
}

/// The in memory rows double as an old migrator for the sync tests.
#[async_trait::async_trait]
impl OldMigrator<Sqlite> for TestMigrator {
    async fn applied_migrations(
        &self,
        _connection: &mut <Sqlite as Database>::Connection,
    ) -> Result<Vec<Box<dyn Migration<Sqlite>>>, Error> {
        Ok(self
            .applied_migrations
            .iter()
            .map(|row| {
                Box::new((row.app().to_string(), row.name().to_string()))
                    as Box<dyn Migration<Sqlite>>
            })
            .collect())
    }
}

/// Convenience macro for implementing `Migration<Sqlite>` for a test struct.
///
/// Arguments in order: `type`, `name`, `parents`, `replaces`, `run_before`.
/// The `app` is always `"test"` and `operations` is always a single no-op
/// SQL pair so plan-generation tests stay focused on ordering logic.
macro_rules! migration {
    ($op:ty, $name:literal, $parents:expr, $replaces:expr, $run_before:expr) => {
        impl crate::migration::Migration<sqlx::Sqlite> for $op {
            fn app(&self) -> &str {
                "test"
            }

            fn name(&self) -> &str {
                $name
            }

            fn parents(&self) -> Vec<Box<dyn crate::migration::Migration<sqlx::Sqlite>>> {
                $parents
            }

            fn operations(&self) -> Vec<Box<dyn crate::operation::Operation<sqlx::Sqlite>>> {
                vec_box![("", "")]
            }

            fn replaces(&self) -> Vec<Box<dyn crate::migration::Migration<sqlx::Sqlite>>> {
                $replaces
            }

            fn run_before(&self) -> Vec<Box<dyn crate::migration::Migration<sqlx::Sqlite>>> {
                $run_before
            }
        }
    };
}

/// Creates an in-memory `SQLite` connection for use in async tests.
async fn make_conn() -> sqlx::pool::PoolConnection<Sqlite> {
    SqlitePool::connect("sqlite::memory:")
        .await
        .unwrap()
        .acquire()
        .await
        .unwrap()
}

/// Generates a migration plan for the given `plan` against `migrator` and
/// its in memory applied rows.
fn apply_plan<'a>(migrator: &'a TestMigrator, plan: &Plan) -> MigrationVecResult<'a, Sqlite> {
    migrator.generate_migration_plan(Some(plan), &migrator.applied_migrations)
}

/// Adds all `migration_list` entries to `migrator`, then returns the full
/// apply-all plan. This is the most common setup pattern for plan tests.
fn generate_apply_all_plan(
    migrator: &mut TestMigrator,
    migration_list: Vec<Box<dyn Migration<Sqlite>>>,
) -> MigrationVecResult<'_, Sqlite> {
    migrator.add_migrations(migration_list)?;
    apply_plan(migrator, &Plan::apply_all())
}

/// Extracts the `name()` of every migration in a plan into a `Vec<&str>`.
///
/// Using names instead of comparing `Box<dyn Migration<Sqlite>>` values gives
/// much cleaner failure messages (e.g. `["a", "b", "c"]` vs raw pointer
/// comparisons), and is sufficient because every test uses unique names.
#[expect(
    clippy::borrowed_box,
    reason = "plan slices hold &Box<dyn Migration> from generate_migration_plan"
)]
fn plan_names<'a>(plan: &'a [&'a Box<dyn Migration<Sqlite>>]) -> Vec<&'a str> {
    plan.iter().map(|m| m.name()).collect()
}

// ── Internal migrations of the migration table ───────────────────────────────

/// Runs a raw statement on the connection, for setting up tampered
/// states the public API cannot produce.
async fn raw(conn: &mut <Sqlite as Database>::Connection, sql: String) {
    sqlx::query(sqlx::AssertSqlSafe(sql))
        .execute(conn)
        .await
        .unwrap();
}

async fn sqlite_table_exists(conn: &mut <Sqlite as Database>::Connection, table: &str) -> bool {
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT 1 FROM {table} WHERE 1 = 0"
    )))
    .execute(conn)
    .await
    .is_ok()
}

// ── Failed migrations and ledger columns ─────────────────────────────────────

/// Migration of the `main` app running plain SQL, optionally outside of a
/// transaction.
struct SqlMigration {
    name: &'static str,
    parent: Option<&'static str>,
    operations: Vec<(&'static str, &'static str)>,
    atomic: bool,
}

impl SqlMigration {
    fn new(name: &'static str, parent: Option<&'static str>, up: &'static str) -> Self {
        Self {
            name,
            parent,
            operations: vec![(up, "SELECT 1")],
            atomic: true,
        }
    }

    fn non_atomic(mut self) -> Self {
        self.atomic = false;
        self
    }

    fn operation(mut self, up: &'static str) -> Self {
        self.operations.push((up, "SELECT 1"));
        self
    }
}

impl Migration<Sqlite> for SqlMigration {
    fn app(&self) -> &'static str {
        "main"
    }

    fn name(&self) -> &str {
        self.name
    }

    fn parents(&self) -> Vec<Box<dyn Migration<Sqlite>>> {
        self.parent
            .map(|parent| Box::new(("main", parent)) as Box<dyn Migration<Sqlite>>)
            .into_iter()
            .collect()
    }

    fn operations(&self) -> Vec<Box<dyn Operation<Sqlite>>> {
        self.operations
            .iter()
            .map(|(up, down)| {
                Box::new((up.to_string(), down.to_string())) as Box<dyn Operation<Sqlite>>
            })
            .collect()
    }

    fn is_atomic(&self) -> bool {
        self.atomic
    }
}

fn migrator_of(migrations: Vec<SqlMigration>) -> Migrator<Sqlite> {
    let mut migrator = Migrator::<Sqlite>::default();
    migrator
        .add_migrations(
            migrations
                .into_iter()
                .map(|migration| Box::new(migration) as Box<dyn Migration<Sqlite>>)
                .collect(),
        )
        .unwrap();
    migrator
}

mod graph;
mod hooks;
mod ledger;
mod lock;
mod model;
mod plan;
mod relations;
