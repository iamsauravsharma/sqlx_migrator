//! Module defining migration trait
//!
//! To create own implement migration trait for type
//!
//! ### Example
//!
//! ```rust,no_run
//! use sqlx_migrator::error::Error;
//! use sqlx_migrator::migration::Migration;
//! use sqlx_migrator::operation::Operation;
//!
//! struct ExampleMigration;
//! #[async_trait::async_trait]
//! impl Migration for ExampleMigration {
//!     type Database = sqlx_migrator::sqlx::Sqlite;
//!
//!     fn app(&self) -> &str {
//!         "example"
//!     }
//!
//!     fn name(&self) -> &str {
//!         "first_migration"
//!     }
//!
//!     fn parents(&self) -> Vec<Box<dyn Migration<Database = Self::Database>>> {
//!         vec![]
//!     }
//!
//!     fn operations(&self) -> Vec<Box<dyn Operation<Database = Self::Database>>> {
//!         vec![]
//!     }
//!
//!     fn replaces(&self) -> Vec<Box<dyn Migration<Database = Self::Database>>> {
//!         vec![]
//!     }
//!
//!     fn run_before(&self) -> Vec<Box<dyn Migration<Database = Self::Database>>> {
//!         vec![]
//!     }
//!
//!     fn is_atomic(&self) -> bool {
//!         true
//!     }
//! }
//! ```

use std::hash::Hash;

use crate::operation::Operation;

/// Trait for migration
pub trait Migration: Send + Sync {
    /// Type of database to be used
    type Database: sqlx::Database;

    /// Migration app name. Can be name of folder or library where migration is
    /// located
    fn app(&self) -> &str;

    /// Migration name. Can be file name without extension
    fn name(&self) -> &str;

    /// Parents of migration (migrations that should be applied before this
    /// migration)
    fn parents(&self) -> Vec<Box<dyn Migration<Database = Self::Database>>> {
        vec![]
    }

    /// Operation performed for migration (create, drop, etc.)
    fn operations(&self) -> Vec<Box<dyn Operation<Database = Self::Database>>> {
        vec![]
    }

    /// Replace certain migrations. If any one of listed migration is applied
    /// than migration will not be applied else migration will apply instead of
    /// applying those migration.
    fn replaces(&self) -> Vec<Box<dyn Migration<Database = Self::Database>>> {
        vec![]
    }

    /// Run before certain migration. This can be helpful in condition where
    /// other library migration need to be applied after this migration
    fn run_before(&self) -> Vec<Box<dyn Migration<Database = Self::Database>>> {
        vec![]
    }

    /// Whether migration is atomic or not. By default it is true
    fn is_atomic(&self) -> bool {
        true
    }
}

impl<DB> PartialEq for dyn Migration<Database = DB>
where
    DB: sqlx::Database,
{
    fn eq(&self, other: &Self) -> bool {
        self.app() == other.app() && self.name() == other.name()
    }
}

impl<DB> Eq for dyn Migration<Database = DB> where DB: sqlx::Database {}

impl<DB> Hash for dyn Migration<Database = DB>
where
    DB: sqlx::Database,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.app().hash(state);
        self.name().hash(state);
    }
}
