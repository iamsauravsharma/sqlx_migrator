//! Operation module
#![cfg_attr(
    feature = "sqlite",
    doc = r##"
To create own operation implement trait for type

### Example
```rust,no_run
use sqlx_migrator::error::Error;
use sqlx_migrator::operation::Operation;
use sqlx_migrator::sqlx::Sqlite;

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

/// Trait for operation
#[allow(clippy::module_name_repetitions)]
#[async_trait::async_trait]
pub trait Operation<DB, State = ()>: Send + Sync
where
    DB: sqlx::Database,
    State: Send + Sync,
{
    /// Up command to be executed during migration apply
    async fn up(
        &self,
        connection: &mut <DB as sqlx::Database>::Connection,
        state: &State,
    ) -> Result<(), Error>;

    /// Down command to be executed during migration rollback. If it is not
    /// implemented than operation is irreversible operation.
    async fn down(
        &self,
        connection: &mut <DB as sqlx::Database>::Connection,
        state: &State,
    ) -> Result<(), Error> {
        let _connection = connection;
        let _state = state;
        return Err(Error::IrreversibleOperation);
    }

    /// Whether up operation is destructible or not. If operation is
    /// destructible than user should answer before running migration through
    /// cli. By default up operation are false. Down operation are always
    /// destructible and cannot be changed
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
