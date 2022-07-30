//! Module defining migration trait
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

    /// Full name of migration. Determined from app and name combination.
    /// Default value is {app}/{name}.
    fn full_name(&self) -> String {
        format!("{}/{}", self.app(), self.name())
    }
}

impl<DB> PartialEq for dyn Migration<Database = DB>
where
    DB: sqlx::Database,
{
    fn eq(&self, other: &Self) -> bool {
        self.full_name() == other.full_name()
    }
}

impl<DB> Eq for dyn Migration<Database = DB> where DB: sqlx::Database {}

impl<DB> Hash for dyn Migration<Database = DB>
where
    DB: sqlx::Database,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.full_name().hash(state);
    }
}
