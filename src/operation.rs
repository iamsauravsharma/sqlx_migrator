//! Operation module
//!
//! To create own operation implement trait for type
//!
//! ### Example
//!
//! ```rust,no_run
//! use sqlx_migrator::error::Error;
//! use sqlx_migrator::operation::Operation;
//! use sqlx_migrator::sqlx::Sqlite;
//!
//! struct ExampleOperation;
//! #[async_trait::async_trait]
//! impl Operation<Sqlite> for ExampleOperation {
//!     async fn up(
//!         &self,
//!         connection: &mut <Sqlite as sqlx::Database>::Connection,
//!     ) -> Result<(), Error> {
//!         // Do some operations
//!         Ok(())
//!     }
//!
//!     // By default operation is irreversible and cannot be reversed if you want to support
//!     // reverse of migration than add down function as well
//!     async fn down(
//!         &self,
//!         connection: &mut <Sqlite as sqlx::Database>::Connection,
//!     ) -> Result<(), Error> {
//!         // Do some operations
//!         Ok(())
//!     }
//! }
//! ```
use crate::error::Error;

/// Trait for operation
#[async_trait::async_trait]
pub trait Operation<DB>: Send + Sync
where
    DB: sqlx::Database,
{
    /// Up command to be executed during migration apply
    async fn up(&self, connection: &mut <DB as sqlx::Database>::Connection) -> Result<(), Error>;
    /// Down command to be executed during migration rollback. If it is not
    /// implemented than operation is irreversible operation.
    async fn down(&self, connection: &mut <DB as sqlx::Database>::Connection) -> Result<(), Error> {
        // use connection from parameter for default implementation
        let _ = connection;
        return Err(Error::IrreversibleOperation);
    }
}