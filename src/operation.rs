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

use std::pin::Pin;

use sqlx::{AssertSqlSafe, Database};

use crate::error::Error;

/// Future returned by the functions of a [`FnOperation`]
pub type OperationFuture<'a> = Pin<Box<dyn Future<Output = Result<(), Error>> + Send + 'a>>;

type OperationFn<DB> =
    Box<dyn for<'a> Fn(&'a mut <DB as Database>::Connection) -> OperationFuture<'a> + Send + Sync>;

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
    async fn down(&self, _connection: &mut <DB as Database>::Connection) -> Result<(), Error> {
        Err(Error::IrreversibleOperation)
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
    <DB as Database>::Arguments: sqlx::IntoArguments<DB>,
    for<'c> &'c mut <DB as Database>::Connection: sqlx::Executor<'c, Database = DB>,
{
    /// # Warning
    /// The SQL strings are executed directly via [`AssertSqlSafe`]. The caller
    /// is responsible for ensuring the strings do not contain untrusted or
    /// unsanitised input.
    async fn up(&self, connection: &mut <DB as Database>::Connection) -> Result<(), Error> {
        let query = self.0.as_ref().to_string();
        sqlx::query(AssertSqlSafe(query))
            .execute(connection)
            .await
            .map_err(Error::from)?;
        Ok(())
    }

    async fn down(&self, connection: &mut <DB as Database>::Connection) -> Result<(), Error> {
        let query = self.1.as_ref().to_string();
        sqlx::query(AssertSqlSafe(query))
            .execute(connection)
            .await
            .map_err(Error::from)?;
        Ok(())
    }
}

/// An irreversible operation built from a single SQL string.
///
/// Only `up` is implemented; reverting a migration containing this operation
/// fails with [`Error::IrreversibleOperation`]. Use a `(up, down)` tuple for a
/// reversible operation.
///
/// # Warning
/// The SQL string is executed directly via [`AssertSqlSafe`]. The caller is
/// responsible for ensuring the string does not contain untrusted or
/// unsanitised input.
#[async_trait::async_trait]
impl<DB> Operation<DB> for &str
where
    DB: Database,
    <DB as Database>::Arguments: sqlx::IntoArguments<DB>,
    for<'c> &'c mut <DB as Database>::Connection: sqlx::Executor<'c, Database = DB>,
{
    async fn up(&self, connection: &mut <DB as Database>::Connection) -> Result<(), Error> {
        sqlx::query(AssertSqlSafe((*self).to_string()))
            .execute(connection)
            .await
            .map_err(Error::from)?;
        Ok(())
    }
}

/// An irreversible operation built from a single owned SQL string.
///
/// See the `&str` implementation for details.
#[async_trait::async_trait]
impl<DB> Operation<DB> for String
where
    DB: Database,
    <DB as Database>::Arguments: sqlx::IntoArguments<DB>,
    for<'c> &'c mut <DB as Database>::Connection: sqlx::Executor<'c, Database = DB>,
{
    async fn up(&self, connection: &mut <DB as Database>::Connection) -> Result<(), Error> {
        sqlx::query(AssertSqlSafe(self.clone()))
            .execute(connection)
            .await
            .map_err(Error::from)?;
        Ok(())
    }
}

/// An operation made of functions, see [`from_fn`] and
/// [`from_fn_irreversible`].
pub struct FnOperation<DB: Database> {
    up: OperationFn<DB>,
    down: Option<OperationFn<DB>>,
    destructible: bool,
}

impl<DB: Database> FnOperation<DB> {
    /// Sets whether `up` is destructible, see [`Operation::is_destructible`]
    #[must_use]
    pub fn destructible(mut self, destructible: bool) -> Self {
        self.destructible = destructible;
        self
    }
}

/// Builds a reversible operation from two functions which get the connection
/// and return a boxed future, usually written as `Box::pin(async move { .. })`.
/// The database has to be named since the closures cannot tell it, for example
/// `from_fn::<Postgres>(..)`.
///
/// To run more than one query pass `&mut *connection` to each.
///
/// # Example
/// ```rust
/// # #[cfg(feature = "sqlite")]
/// # fn main() {
/// use sqlx::Sqlite;
/// use sqlx_migrator::operation::from_fn;
///
/// let operation = from_fn::<Sqlite>(
///     |connection| {
///         Box::pin(async move {
///             sqlx::query("CREATE TABLE sample (id INTEGER)")
///                 .execute(connection)
///                 .await?;
///             Ok(())
///         })
///     },
///     |connection| {
///         Box::pin(async move {
///             sqlx::query("DROP TABLE sample").execute(connection).await?;
///             Ok(())
///         })
///     },
/// );
/// # }
/// # #[cfg(not(feature = "sqlite"))]
/// # fn main() {}
/// ```
pub fn from_fn<DB: Database>(
    up: impl for<'a> Fn(&'a mut <DB as Database>::Connection) -> OperationFuture<'a>
    + Send
    + Sync
    + 'static,
    down: impl for<'a> Fn(&'a mut <DB as Database>::Connection) -> OperationFuture<'a>
    + Send
    + Sync
    + 'static,
) -> FnOperation<DB> {
    FnOperation {
        up: Box::new(up),
        down: Some(Box::new(down)),
        destructible: false,
    }
}

/// Builds an irreversible operation from a function, see [`from_fn`].
/// Reverting a migration containing it fails with
/// [`Error::IrreversibleOperation`].
pub fn from_fn_irreversible<DB: Database>(
    up: impl for<'a> Fn(&'a mut <DB as Database>::Connection) -> OperationFuture<'a>
    + Send
    + Sync
    + 'static,
) -> FnOperation<DB> {
    FnOperation {
        up: Box::new(up),
        down: None,
        destructible: false,
    }
}

#[async_trait::async_trait]
impl<DB: Database> Operation<DB> for FnOperation<DB> {
    async fn up(&self, connection: &mut <DB as Database>::Connection) -> Result<(), Error> {
        (self.up)(connection).await
    }

    async fn down(&self, connection: &mut <DB as Database>::Connection) -> Result<(), Error> {
        match &self.down {
            Some(down) => down(connection).await,
            None => Err(Error::IrreversibleOperation),
        }
    }

    fn is_destructible(&self) -> bool {
        self.destructible
    }
}
