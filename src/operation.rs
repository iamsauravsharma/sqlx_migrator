use sqlx::postgres::Postgres;
use sqlx::Transaction;

use crate::Error;

/// Trait of operation
#[async_trait::async_trait]
pub trait Operation {
    /// Up command
    async fn up(&self, transaction: &mut Transaction<Postgres>) -> Result<(), Error>;
    /// Down command
    async fn down(&self, transaction: &mut Transaction<Postgres>) -> Result<(), Error>;
}
