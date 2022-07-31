use sqlx_migrator::error::Error;
use sqlx_migrator::migration::Migration;
use sqlx_migrator::operation::Operation;

pub(crate) struct M0002Operation;

#[async_trait::async_trait]
impl Operation for M0002Operation {
    type Database = sqlx::Postgres;

    async fn up(
        &self,
        connection: &mut <Self::Database as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        sqlx::query("INSERT INTO sample (id, name) VALUES (99, 'Some text') ")
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn down(
        &self,
        connection: &mut <Self::Database as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        sqlx::query("DELETE FROM sample WHERE id = 99")
            .execute(connection)
            .await?;
        Ok(())
    }
}

pub(crate) struct M0002Migration;

#[async_trait::async_trait]
impl Migration for M0002Migration {
    type Database = sqlx::Postgres;

    fn app(&self) -> &str {
        "main"
    }

    fn name(&self) -> &str {
        "m0002_with_parents"
    }

    fn parents(&self) -> Vec<Box<dyn Migration<Database = Self::Database>>> {
        vec![Box::new(crate::migrations::m0001_simple::M0001Migration)]
    }

    fn operations(&self) -> Vec<Box<dyn Operation<Database = Self::Database>>> {
        vec![Box::new(M0002Operation)]
    }
}
