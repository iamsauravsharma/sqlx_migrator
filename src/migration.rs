//! Module for defining the [`Migration`] trait, which represents a database
//! migration.
//!
//! This module provides the necessary abstractions for defining migrations
#![cfg_attr(
    feature = "sqlite",
    doc = r#"
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
"#
)]

use std::hash::Hash;

use crate::operation::Operation;

/// Trait representing a database migration.
///
/// A migration consists of a series of operations that modify the database
/// schema or data. Each migration is uniquely identified by its application
/// name and migration name. It can also have dependencies on other migrations
/// through parents, replacements, and run-before relationships.
///
/// Migration trait is implemented for `(A,N) where A: AsRef<str>, N:
/// AsRef<str>` where A is the app name and N is the name of the migration. You
/// can use this to create a virtual migration which can be used to represent a
/// migration which is not present in the codebase but is present in the
/// database. This is useful when you want to represent a migration which is
/// applied outside the codebase and cannot be imported in the codebase.
pub trait Migration<DB>: Send + Sync {
    /// Returns the application name associated with the migration.
    /// This can be the name of the folder or library where the migration is
    /// located.
    ///
    /// This value is used in combination with the migration name to uniquely
    /// identify a migration.
    fn app(&self) -> &str;

    /// Returns the migration name, typically the file name without the
    /// extension.
    ///
    /// This value, together with the application name, uniquely identifies a
    /// migration.
    fn name(&self) -> &str;

    /// Returns the list of parent migrations that must be applied before this
    /// migration can be applied.
    ///
    /// This can be useful for defining dependencies between migrations.
    fn parents(&self) -> Vec<Box<dyn Migration<DB>>>;

    /// Returns the list of operations that make up the migration.
    ///
    /// These operations will be executed when the migration is applied or
    /// reverted.
    fn operations(&self) -> Vec<Box<dyn Operation<DB>>>;

    /// Returns the list of migrations that this migration replaces.
    ///
    /// If any of these migrations have been applied, this migration will not be
    /// applied. If not, it will either be applied or reverted in place of
    /// those migrations.
    ///
    /// The default implementation returns an empty vector.
    fn replaces(&self) -> Vec<Box<dyn Migration<DB>>> {
        vec![]
    }

    /// Returns the list of migrations that should be run before this migration
    /// when applying. but when reverting this migration these migrations
    /// should be reverted after this migration is reverted.
    ///
    /// This can be useful when a migration from another library needs to be
    /// applied after this migration or reverted before this migration.
    ///
    /// The default implementation returns an empty vector.
    fn run_before(&self) -> Vec<Box<dyn Migration<DB>>> {
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
    /// are ignored expect its application name and its own name to check with
    /// non virtual migration so such non virtual migration can be used in its
    /// place.
    fn is_virtual(&self) -> bool {
        false
    }
}

impl<DB> PartialEq for dyn Migration<DB> {
    fn eq(&self, other: &Self) -> bool {
        self.app() == other.app() && self.name() == other.name()
    }
}

impl<DB> Eq for dyn Migration<DB> {}

impl<DB> Hash for dyn Migration<DB> {
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

    fn parents(&self) -> Vec<Box<dyn Migration<DB>>> {
        vec![]
    }

    fn operations(&self) -> Vec<Box<dyn Operation<DB>>> {
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

    /// Return id value present on database
    #[must_use]
    pub fn id(&self) -> i32 {
        self.id
    }

    /// Return migration app name stored inside database
    #[must_use]
    pub fn app(&self) -> &str {
        &self.app
    }

    /// Return migration name stored inside database
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return migration applied time
    #[must_use]
    pub fn applied_time(&self) -> &str {
        &self.applied_time
    }
}

impl<DB> PartialEq<Box<dyn Migration<DB>>> for AppliedMigrationSqlRow {
    fn eq(&self, other: &Box<dyn Migration<DB>>) -> bool {
        self.app == other.app() && self.name == other.name()
    }
}
