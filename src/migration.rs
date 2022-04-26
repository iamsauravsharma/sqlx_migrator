//! Module defining migration trait

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
