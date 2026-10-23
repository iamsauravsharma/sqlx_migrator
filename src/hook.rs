//! Hooks run functions at fixed points of a run
//!
//! Register a function with [`Migrator::add_hook`](crate::Migrator::add_hook)
//! and the [`HookType`] saying when it runs. It gets the connection doing the
//! work, which is the transaction of an atomic migration, so a hook can set
//! transaction scoped options such as Postgres `SET LOCAL lock_timeout`. When
//! the lock strategy holds the whole plan in one transaction every hook runs
//! inside it. A hook which fails stops the run and rolls back the transaction
//! it ran in.
//!
//! Migration hooks are not called for a fake plan since it executes nothing.
#![cfg_attr(
    feature = "postgres",
    doc = r#"
# Example
Give up on a lock held by another session after five seconds instead of
waiting for ever
```rust,no_run
use sqlx::{Executor as _, Postgres};
use sqlx_migrator::hook::{HookContext, HookFuture, HookType};
use sqlx_migrator::Migrator;

fn lock_timeout<'a>(
    connection: &'a mut sqlx::PgConnection,
    _context: &'a HookContext<'a, Postgres>,
) -> HookFuture<'a> {
    Box::pin(async move {
        connection.execute("SET LOCAL lock_timeout = '5s'").await?;
        Ok(())
    })
}

let mut migrator = Migrator::<Postgres>::default();
migrator.add_hook(HookType::BeforeMigration, lock_timeout);
// a closure works the same way
migrator.add_hook(HookType::AfterRun, |_connection, context| {
    Box::pin(async move {
        println!("ran {} migrations", context.report().map_or(0, |report| report.len()));
        Ok(())
    })
});
```
"#
)]

use std::pin::Pin;

use sqlx::Database;

use crate::error::Error;
use crate::migration::Migration;
use crate::migrator::RunReport;

/// Future returned by a hook function
pub type HookFuture<'a> = Pin<Box<dyn Future<Output = Result<(), Error>> + Send + 'a>>;

pub(crate) type BoxHook<DB> = Box<
    dyn for<'a> Fn(&'a mut <DB as Database>::Connection, &'a HookContext<'a, DB>) -> HookFuture<'a>
        + Send
        + Sync,
>;

/// When a hook function runs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HookType {
    /// Once the lock is taken, before the first migration
    BeforeRun,
    /// After the last migration, before the lock is released. The context has
    /// the [`RunReport`]
    AfterRun,
    /// Before the operations of a migration, inside its transaction when it is
    /// atomic. The context has the migration
    BeforeMigration,
    /// After the operations of a migration, inside its transaction when it is
    /// atomic, so a failure rolls the migration back. The context has the
    /// migration
    AfterMigration,
}

/// What a hook function is told about the run
pub struct HookContext<'a, DB> {
    apply: bool,
    migration: Option<&'a dyn Migration<DB>>,
    report: Option<&'a RunReport>,
}

impl<'a, DB> HookContext<'a, DB> {
    pub(crate) fn new(
        apply: bool,
        migration: Option<&'a dyn Migration<DB>>,
        report: Option<&'a RunReport>,
    ) -> Self {
        Self {
            apply,
            migration,
            report,
        }
    }

    /// `true` when migrations are applied and `false` when they are reverted
    #[must_use]
    pub fn is_apply(&self) -> bool {
        self.apply
    }

    /// The migration for [`HookType::BeforeMigration`] and
    /// [`HookType::AfterMigration`], `None` otherwise
    #[must_use]
    pub fn migration(&self) -> Option<&'a dyn Migration<DB>> {
        self.migration
    }

    /// The report for [`HookType::AfterRun`], `None` otherwise
    #[must_use]
    pub fn report(&self) -> Option<&'a RunReport> {
        self.report
    }
}
