//! Module defining migration trait
use std::hash::Hash;

use crate::operation::Operation;

/// Trait for migration
pub trait Migration: Send + Sync {
    // Type of database
    type Database: sqlx::Database;

    /// Migration name
    fn name(&self) -> String;

    /// Parents of migration
    fn parents(&self) -> Vec<Box<dyn Migration<Database = Self::Database>>>;

    /// Operation performed for migration
    fn operations(&self) -> Vec<Box<dyn Operation<Database = Self::Database>>>;
}

impl<DB> PartialEq for dyn Migration<Database = DB>
where
    DB: sqlx::Database,
{
    fn eq(&self, other: &Self) -> bool {
        self.name() == other.name()
    }
}

impl<DB> Eq for dyn Migration<Database = DB> where DB: sqlx::Database {}

impl<DB> Hash for dyn Migration<Database = DB>
where
    DB: sqlx::Database,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name().hash(state);
    }
}
