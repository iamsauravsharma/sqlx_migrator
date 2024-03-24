//! Module defining migration trait
#![cfg_attr(
    feature = "sqlite",
    doc = r##"
To create own implement migration trait for type

### Example
```rust,no_run
use sqlx_migrator::error::Error;
use sqlx_migrator::migration::Migration;
use sqlx_migrator::operation::Operation;
use sqlx_migrator::sqlx::Sqlite;

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
}
```
"##
)]

use std::hash::Hash;

use crate::operation::Operation;

/// Trait for migration
#[allow(clippy::module_name_repetitions)]
pub trait Migration<DB, State = ()>: Send + Sync {
    /// Migration app name. Can be name of folder or library where migration is
    /// located
    fn app(&self) -> &str;

    /// Migration name. Can be file name without extension
    fn name(&self) -> &str;

    /// Parents of migration (migrations that should be applied before this
    /// migration)
    fn parents(&self) -> Vec<Box<dyn Migration<DB, State>>>;

    /// Operation performed for migration (create, drop, etc.)
    fn operations(&self) -> Vec<Box<dyn Operation<DB, State>>>;

    /// Replace certain migrations. If any one of listed migration is applied
    /// than migration will not be applied else migration will apply/revert
    /// instead of applying/reverting those migration.
    fn replaces(&self) -> Vec<Box<dyn Migration<DB, State>>> {
        vec![]
    }

    /// Run before(for applying)/after(for reverting) certain migration. This
    /// can be helpful in condition where other library migration need to be
    /// applied after this migration or reverted before this migration
    fn run_before(&self) -> Vec<Box<dyn Migration<DB, State>>> {
        vec![]
    }

    /// Whether migration is atomic or not. By default it is atomic so this
    /// function returns `true`
    fn is_atomic(&self) -> bool {
        true
    }

    /// Whether migration is virtual or not. By default migration are not
    /// virtual. If migration is virtual than it expects another migration with
    /// same name present inside migration list which is not virtual.
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

/// Migration struct created from sql table. Struct contains 4 fields which maps
/// to `id`, `app`, `name`, `applied_time` sql fields. It is used to list
/// applied migrations
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
