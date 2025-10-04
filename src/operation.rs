//! Module for defining the [`Operation`] trait
//!
//! This module provides the [`Operation`] trait, allowing users to define
//! database operations that can be executed as part of a migration process.
//! These operations can be applied (`up`) or optionally reverted (`down`).
#![cfg_attr(
    feature = "sqlite",
    doc = "
To create own operation implement trait for type

### Example
```rust,no_run
use sqlx_migrator::error::Error;
use sqlx_migrator::operation::Operation;
use sqlx::Sqlite;

struct ExampleOperation;

#[async_trait::async_trait]
impl Operation<Sqlite> for ExampleOperation {
    async fn up(
        &self,
        connection: &mut sqlx::SqliteConnection,
    ) -> Result<(), Error> {
        // Do some operations
        Ok(())
    }

    // By default operation is irreversible and cannot be reversed if you want to support
    // reverse of migration than add down function as well
    async fn down(
        &self,
        connection: &mut sqlx::SqliteConnection,
    ) -> Result<(), Error> {
        // Do some operations
        Ok(())
    }
}
```
"
)]

use sqlx::Database;

use crate::error::Error;

/// Trait representing a database migration operation.
///
/// Operations are the building blocks of a migration, defining the specific
/// changes to be made to the database schema or data. Each operation must
/// implement the `up` method to apply the operation, and can optionally
/// implement the `down` method to reverse the operation.
#[async_trait::async_trait]
pub trait Operation<DB>: Send + Sync
where
    DB: Database,
{
    /// The up method applies the operation to the database.
    ///
    /// This method is called when the migration is being applied to the
    /// database. Implement this method to define the changes you want to
    /// apply.
    async fn up(&self, connection: &mut <DB as Database>::Connection) -> Result<(), Error>;

    /// The down method reverts the operation, if possible.
    ///
    /// Down operations are optional and may not be implemented for all. If
    /// not implemented, the operation is considered irreversible. If you want
    /// to make an operation reversible, implement this method to define how
    /// to revert the changes made in the `up` method.
    async fn down(&self, connection: &mut <DB as Database>::Connection) -> Result<(), Error> {
        let _connection = connection;
        return Err(Error::IrreversibleOperation);
    }

    /// Indicates whether the `up` operation is destructible.
    ///
    /// If the operation is destructible, the user will be prompted for
    /// confirmation before running the migration via the CLI, due to the
    /// potential for data loss or irreversible changes. By default, `up`
    /// operations are considered non-destructible. Note that `down` operations
    /// are always considered destructible and cannot be changed.
    fn is_destructible(&self) -> bool {
        false
    }
}

#[async_trait::async_trait]
impl<DB, U, D> Operation<DB> for (U, D)
where
    DB: Database,
    U: AsRef<str> + Send + Sync,
    D: AsRef<str> + Send + Sync,
    for<'c> &'c mut <DB as Database>::Connection: sqlx::Executor<'c, Database = DB>,
    for<'q> <DB as Database>::Arguments<'q>: sqlx::IntoArguments<'q, DB>,
{
    async fn up(&self, connection: &mut <DB as Database>::Connection) -> Result<(), Error> {
        sqlx::query(self.0.as_ref())
            .execute(connection)
            .await
            .map_err(Error::from)?;
        Ok(())
    }

    async fn down(&self, connection: &mut <DB as Database>::Connection) -> Result<(), Error> {
        sqlx::query(self.1.as_ref())
            .execute(connection)
            .await
            .map_err(Error::from)?;
        Ok(())
    }
}
