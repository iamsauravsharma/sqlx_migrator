use sqlx::{MySql, MySqlConnection};
use sqlx_migrator::error::Error;
use sqlx_migrator::migration::Migration;
use sqlx_migrator::operation::Operation;

pub(crate) struct M0005Operation;

#[async_trait::async_trait]
impl Operation<MySql> for M0005Operation {
    async fn up(&self, connection: &mut MySqlConnection) -> Result<(), Error> {
        sqlx::query("INSERT INTO sample (id, name) VALUES (888, 'complex')")
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn down(&self, connection: &mut MySqlConnection) -> Result<(), Error> {
        sqlx::query("DELETE FROM sample WHERE id = 888")
            .execute(connection)
            .await?;
        Ok(())
    }
}

pub(crate) struct M0005Migration;

impl Migration<MySql> for M0005Migration {
    fn app(&self) -> &'static str {
        "main"
    }

    fn name(&self) -> &'static str {
        "m0005_reference_complex"
    }

    fn parents(&self) -> Vec<Box<dyn Migration<MySql>>> {
        vec![Box::new(("main", "m0004_complex_operation"))]
    }

    fn operations(&self) -> Vec<Box<dyn Operation<MySql>>> {
        vec![Box::new(M0005Operation)]
    }
}
