use sqlx_migrator::error::Error;
use sqlx_migrator::migration::Migration;
use sqlx_migrator::operation::Operation;
use sqlx_migrator::sqlx::Sqlite;

pub(crate) struct M0001Operation;

#[async_trait::async_trait]
impl Operation<Sqlite> for M0001Operation {
    async fn up(
        &self,
        connection: &mut <Sqlite as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        sqlx::query("CREATE TABLE sample (id INTEGER PRIMARY KEY, name TEXT)")
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn down(
        &self,
        connection: &mut <Sqlite as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        sqlx::query("DROP TABLE sample").execute(connection).await?;
        Ok(())
    }
}

pub(crate) struct M0001Migration;

#[async_trait::async_trait]
impl Migration<Sqlite> for M0001Migration {
    fn app(&self) -> &str {
        "main"
    }

    fn name(&self) -> &str {
        "m0001_simple"
    }

    fn operations(&self) -> Vec<Box<dyn Operation<Sqlite>>> {
        vec![Box::new(M0001Operation)]
    }
}