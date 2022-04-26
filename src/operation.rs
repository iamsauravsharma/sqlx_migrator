//! Operation module

use sqlx::Transaction;

use crate::error::Error;

/// Trait for operation
#[async_trait::async_trait]
pub trait Operation: Send + Sync {
    type Database: sqlx::Database;
    /// Up command
    async fn up(&self, transaction: &mut Transaction<Self::Database>) -> Result<(), Error>;
    /// Down command
    async fn down(&self, transaction: &mut Transaction<Self::Database>) -> Result<(), Error>;
}
