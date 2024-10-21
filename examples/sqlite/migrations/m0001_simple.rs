use sqlx::{Sqlite, SqliteConnection};
use sqlx_migrator::error::Error;
use sqlx_migrator::migration::Migration;
use sqlx_migrator::operation::Operation;

pub(crate) struct M0001Operation;

#[async_trait::async_trait]
impl Operation<Sqlite> for M0001Operation {
    async fn up(&self, connection: &mut SqliteConnection) -> Result<(), Error> {
        sqlx::query("CREATE TABLE sample (id INTEGER PRIMARY KEY, name TEXT)")
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn down(&self, connection: &mut SqliteConnection) -> Result<(), Error> {
        sqlx::query("DROP TABLE sample").execute(connection).await?;
        Ok(())
    }
}

pub(crate) struct M0001Migration;

#[async_trait::async_trait]
impl Migration<Sqlite> for M0001Migration {
    fn app(&self) -> &'static str {
        "main"
    }

    fn name(&self) -> &'static str {
        "m0001_simple"
    }

    fn parents(&self) -> Vec<Box<dyn Migration<Sqlite>>> {
        vec![]
    }

    fn operations(&self) -> Vec<Box<dyn Operation<Sqlite>>> {
        vec![Box::new(M0001Operation)]
    }
}
