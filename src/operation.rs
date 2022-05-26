//! Operation module

use sqlx::Transaction;

use crate::error::Error;

/// Trait for operation
#[async_trait::async_trait]
pub trait Operation: Send + Sync {
    /// Database type to be used
    type Database: sqlx::Database;
    /// Up command to be executed during migration apply
    async fn up(&self, transaction: &mut Transaction<Self::Database>) -> Result<(), Error>;
    /// Down command to be executed during migration rollback
    async fn down(&self, transaction: &mut Transaction<Self::Database>) -> Result<(), Error>;
}
