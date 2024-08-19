//! Module for defining the `Migration` trait, which represents a database
//! migration.
//!
//! This module provides the necessary abstractions for defining migrations
#![cfg_attr(
    feature = "sqlite",
    doc = r##"
To create own implement migration trait for type

### Example
```rust,no_run
use sqlx_migrator::error::Error;
use sqlx_migrator::migration::Migration;
use sqlx_migrator::operation::Operation;
use sqlx::Sqlite;

struct ExampleMigration;

impl Migration<Sqlite> for ExampleMigration {
    fn app(&self) -> &str {
        "example"
    }

    fn name(&self) -> &str {
        "first_migration"
    }

    fn parents(&self) -> Vec<Box<dyn Migration<Sqlite>>> {
        vec![]
    }

    fn operations(&self) -> Vec<Box<dyn Operation<Sqlite>>> {
        vec![]
    }

    fn replaces(&self) -> Vec<Box<dyn Migration<Sqlite>>> {
        vec![]
    }

    fn run_before(&self) -> Vec<Box<dyn Migration<Sqlite>>> {
        vec![]
    }

    fn is_atomic(&self) -> bool {
        true
    }

    fn is_virtual(&self) -> bool {
        false
    }
}
```
"##
)]

use std::hash::Hash;

use crate::operation::Operation;

/// Trait for defining database migration
///
/// A migration represents a set of operations that can be applied to or
/// reverted from a database. Each migration has an associated application name,
/// migration name, and may depend on other migrations.
///
/// Migrations can also replace existing migrations, enforce ordering with
/// run before and parents, and control atomicity and virtualization.
#[allow(clippy::module_name_repetitions)]
pub trait Migration<DB, State = ()>: Send + Sync {
    /// Returns the application name associated with the migration.
    /// This can be the name of the folder or library where the migration is
    /// located.
    fn app(&self) -> &str;

    /// Returns the migration name, typically the file name without the
    /// extension.
    fn name(&self) -> &str;

    /// Returns the list of parent migrations.
    ///
    ///  Parent migrations must be applied before this migration can be applied.
    /// If no parent migrations are required, return an empty vector.
    fn parents(&self) -> Vec<Box<dyn Migration<DB, State>>>;

    /// Returns the operations associated with this migration.
    ///
    /// A migration can include multiple operations (e.g., create, drop) that
    /// are related.
    fn operations(&self) -> Vec<Box<dyn Operation<DB, State>>>;

    /// Returns the list of migrations that this migration replaces.
    ///
    /// If any of these migrations have been applied, this migration will not be
    /// applied. Instead, it will either be applied or reverted in place of
    /// those migrations.
    ///
    /// The default implementation returns an empty vector.
    fn replaces(&self) -> Vec<Box<dyn Migration<DB, State>>> {
        vec![]
    }

    /// Returns the list of migrations that this migration must run before(when
    /// applying) or after (when reverting).
    ///
    /// This can be useful when a migration from another library needs to be
    /// applied after this migration or reverted before this migration.
    ///
    /// The default implementation returns an empty vector.
    fn run_before(&self) -> Vec<Box<dyn Migration<DB, State>>> {
        vec![]
    }

    /// Indicates whether the migration is atomic.
    /// By default, this function returns `true`, meaning the migration is
    /// atomic.
    ///
    /// If the migration is non-atomic, all its operations will be non-atomic as
    /// well. For migrations requiring mixed atomicity, it's recommended to
    /// split them into separate migrations, each handling atomic and
    /// non-atomic operations respectively.
    fn is_atomic(&self) -> bool {
        true
    }

    /// Indicates whether the migration is virtual.
    /// By default, this function returns `false`, meaning the migration is not
    /// virtual.
    ///
    /// A virtual migration serves as a reference to another migration with the
    /// same app and name. If the migration is virtual, all other methods
    /// are ignored.
    fn is_virtual(&self) -> bool {
        false
    }
}

impl<DB, State> PartialEq for dyn Migration<DB, State> {
    fn eq(&self, other: &Self) -> bool {
        self.app() == other.app() && self.name() == other.name()
    }
}

impl<DB, State> Eq for dyn Migration<DB, State> {}

impl<DB, State> Hash for dyn Migration<DB, State> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.app().hash(state);
        self.name().hash(state);
    }
}

impl<DB, A, N> Migration<DB> for (A, N)
where
    A: AsRef<str> + Send + Sync,
    N: AsRef<str> + Send + Sync,
{
    fn app(&self) -> &str {
        self.0.as_ref()
    }

    fn name(&self) -> &str {
        self.1.as_ref()
    }

    fn parents(&self) -> Vec<Box<dyn Migration<DB, ()>>> {
        vec![]
    }

    fn operations(&self) -> Vec<Box<dyn Operation<DB, ()>>> {
        vec![]
    }

    fn is_virtual(&self) -> bool {
        true
    }
}

/// Struct representing a migration row from the database.
///
/// This struct corresponds to the id, app, name, and applied time fields in the
/// database. It is used to list the migrations that have been applied.
#[derive(sqlx::FromRow, Clone)]
pub struct AppliedMigrationSqlRow {
    id: i32,
    app: String,
    name: String,
    applied_time: String,
}

impl AppliedMigrationSqlRow {
    #[cfg(test)]
    pub(crate) fn new(id: i32, app: &str, name: &str) -> Self {
        Self {
            id,
            app: app.to_string(),
            name: name.to_string(),
            applied_time: String::new(),
        }
    }
}

impl AppliedMigrationSqlRow {
    /// Return id value present on database
    #[must_use]
    pub fn id(&self) -> i32 {
        self.id
    }

    /// Return migration applied time
    #[must_use]
    pub fn applied_time(&self) -> &str {
        &self.applied_time
    }
}

impl<DB, State> PartialEq<Box<dyn Migration<DB, State>>> for AppliedMigrationSqlRow {
    fn eq(&self, other: &Box<dyn Migration<DB, State>>) -> bool {
        self.app == other.app() && self.name == other.name()
    }
}
