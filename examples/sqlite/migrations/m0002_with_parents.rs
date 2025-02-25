use sqlx::{Sqlite, SqliteConnection};
use sqlx_migrator::error::Error;
use sqlx_migrator::migration::Migration;
use sqlx_migrator::operation::Operation;

pub(crate) struct M0002Operation;

#[async_trait::async_trait]
impl Operation<Sqlite> for M0002Operation {
    async fn up(&self, connection: &mut SqliteConnection) -> Result<(), Error> {
        sqlx::query("INSERT INTO sample (id, name) VALUES (99, 'Some text')")
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn down(&self, connection: &mut SqliteConnection) -> Result<(), Error> {
        sqlx::query("DELETE FROM sample WHERE id = 99")
            .execute(connection)
            .await?;
        Ok(())
    }
}

pub(crate) struct M0002Migration;

impl Migration<Sqlite> for M0002Migration {
    fn app(&self) -> &'static str {
        "main"
    }

    fn name(&self) -> &'static str {
        "m0002_with_parents"
    }

    fn parents(&self) -> Vec<Box<dyn Migration<Sqlite>>> {
        vec![Box::new(crate::migrations::m0001_simple::M0001Migration)]
    }

    fn operations(&self) -> Vec<Box<dyn Operation<Sqlite>>> {
        vec![Box::new(M0002Operation)]
    }
}
