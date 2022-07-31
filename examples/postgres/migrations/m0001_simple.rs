use sqlx_migrator::error::Error;
use sqlx_migrator::migration::Migration;
use sqlx_migrator::operation::Operation;
use sqlx_migrator::sqlx::Postgres;

pub(crate) struct M0001Operation;

#[async_trait::async_trait]
impl Operation for M0001Operation {
    type Database = Postgres;

    async fn up(
        &self,
        connection: &mut <Self::Database as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        sqlx::query("CREATE TABLE sample (id INTEGER PRIMARY KEY, name TEXT)")
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn down(
        &self,
        connection: &mut <Self::Database as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        sqlx::query("DROP TABLE sample").execute(connection).await?;
        Ok(())
    }
}

pub(crate) struct M0001Migration;

#[async_trait::async_trait]
impl Migration for M0001Migration {
    type Database = Postgres;

    fn app(&self) -> &str {
        "main"
    }

    fn name(&self) -> &str {
        "m0001_simple"
    }

    fn operations(&self) -> Vec<Box<dyn Operation<Database = Self::Database>>> {
        vec![Box::new(M0001Operation)]
    }
}
