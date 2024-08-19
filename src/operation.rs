//! Module for defining the `Operation` trait
//!
//! This module provides the `Operation` trait, allowing users to define
//! database operations that can be executed as part of a migration process.
//! These operations can be applied (`up`) or optionally reverted (`down`).
#![cfg_attr(
    feature = "sqlite",
    doc = r##"
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
        state: &(),
    ) -> Result<(), Error> {
        // Do some operations
        Ok(())
    }

    // By default operation is irreversible and cannot be reversed if you want to support
    // reverse of migration than add down function as well
    async fn down(
        &self,
        connection: &mut sqlx::SqliteConnection,
        state: &(),
    ) -> Result<(), Error> {
        // Do some operations
        Ok(())
    }
}
```
"##
)]

use crate::error::Error;

/// Trait for defining a database operation.
///
/// An Operation represents action that can be applied to or reverted from a
/// database during a migration. Each operation can have an up method for
/// applying the change and an optional down method for rolling it back.
///
/// Operations can also specify whether they are "destructible," meaning that
/// they require user confirmation before being applied, due to potential data
/// loss or irreversible changes
#[allow(clippy::module_name_repetitions)]
#[async_trait::async_trait]
pub trait Operation<DB, State = ()>: Send + Sync
where
    DB: sqlx::Database,
    State: Send + Sync,
{
    /// The up method executes the operation when applying the migration.
    ///
    /// This method is called when the migration is being applied to the
    /// database. Implement this method to define the changes you want to
    /// apply.
    async fn up(
        &self,
        connection: &mut <DB as sqlx::Database>::Connection,
        state: &State,
    ) -> Result<(), Error>;

    /// The `down` method reverses the operation when rolling back the
    /// migration.
    ///
    /// This method is called when the migration is being rolled back. Implement
    /// this method if you want to make the operation reversible. If not
    /// implemented, the operation is considered irreversible.
    async fn down(
        &self,
        connection: &mut <DB as sqlx::Database>::Connection,
        state: &State,
    ) -> Result<(), Error> {
        let _connection = connection;
        let _state = state;
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

#[cfg(all(
    any(feature = "postgres", feature = "mysql", feature = "sqlite"),
    feature = "any"
))]
#[async_trait::async_trait]
impl<U, D> Operation<sqlx::Any> for (U, D)
where
    U: AsRef<str> + Send + Sync,
    D: AsRef<str> + Send + Sync,
{
    async fn up(&self, connection: &mut sqlx::AnyConnection, _state: &()) -> Result<(), Error> {
        sqlx::query(self.0.as_ref()).execute(connection).await?;
        Ok(())
    }

    async fn down(&self, connection: &mut sqlx::AnyConnection, _state: &()) -> Result<(), Error> {
        sqlx::query(self.1.as_ref()).execute(connection).await?;
        Ok(())
    }
}

#[cfg(feature = "mysql")]
#[async_trait::async_trait]
impl<U, D> Operation<sqlx::MySql> for (U, D)
where
    U: AsRef<str> + Send + Sync,
    D: AsRef<str> + Send + Sync,
{
    async fn up(&self, connection: &mut sqlx::MySqlConnection, _state: &()) -> Result<(), Error> {
        sqlx::query(self.0.as_ref()).execute(connection).await?;
        Ok(())
    }

    async fn down(&self, connection: &mut sqlx::MySqlConnection, _state: &()) -> Result<(), Error> {
        sqlx::query(self.1.as_ref()).execute(connection).await?;
        Ok(())
    }
}

#[cfg(feature = "postgres")]
#[async_trait::async_trait]
impl<U, D> Operation<sqlx::Postgres> for (U, D)
where
    U: AsRef<str> + Send + Sync,
    D: AsRef<str> + Send + Sync,
{
    async fn up(&self, connection: &mut sqlx::PgConnection, _state: &()) -> Result<(), Error> {
        sqlx::query(self.0.as_ref()).execute(connection).await?;
        Ok(())
    }

    async fn down(&self, connection: &mut sqlx::PgConnection, _state: &()) -> Result<(), Error> {
        sqlx::query(self.1.as_ref()).execute(connection).await?;
        Ok(())
    }
}

#[cfg(feature = "sqlite")]
#[async_trait::async_trait]
impl<U, D> Operation<sqlx::Sqlite> for (U, D)
where
    U: AsRef<str> + Send + Sync,
    D: AsRef<str> + Send + Sync,
{
    async fn up(&self, connection: &mut sqlx::SqliteConnection, _state: &()) -> Result<(), Error> {
        sqlx::query(self.0.as_ref()).execute(connection).await?;
        Ok(())
    }

    async fn down(
        &self,
        connection: &mut sqlx::SqliteConnection,
        _state: &(),
    ) -> Result<(), Error> {
        sqlx::query(self.1.as_ref()).execute(connection).await?;
        Ok(())
    }
}
