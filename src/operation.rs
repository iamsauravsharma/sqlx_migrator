//! Operation module

use crate::error::Error;

/// Trait for operation
#[async_trait::async_trait]
pub trait Operation: Send + Sync {
    /// Database type to be used
    type Database: sqlx::Database;
    /// Up command to be executed during migration apply
    async fn up(
        &self,
        connection: &mut <Self::Database as sqlx::Database>::Connection,
    ) -> Result<(), Error>;
    /// Down command to be executed during migration rollback
    async fn down(
        &self,
        connection: &mut <Self::Database as sqlx::Database>::Connection,
    ) -> Result<(), Error>;
}
