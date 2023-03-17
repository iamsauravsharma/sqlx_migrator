use sqlx::MySqlConnection;
use sqlx_migrator::error::Error;
use sqlx_migrator::migration::MigrationTrait;
use sqlx_migrator::operation::OperationTrait;
use sqlx_migrator::sqlx::MySql;

pub(crate) struct M0001Operation;

#[async_trait::async_trait]
impl OperationTrait<MySql> for M0001Operation {
    async fn up(&self, connection: &mut MySqlConnection) -> Result<(), Error> {
        sqlx::query("CREATE TABLE sample (id INTEGER PRIMARY KEY, name TEXT)")
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn down(&self, connection: &mut MySqlConnection) -> Result<(), Error> {
        sqlx::query("DROP TABLE sample").execute(connection).await?;
        Ok(())
    }
}

pub(crate) struct M0001Migration;

#[async_trait::async_trait]
impl MigrationTrait<MySql> for M0001Migration {
    fn app(&self) -> &str {
        "main"
    }

    fn name(&self) -> &str {
        "m0001_simple"
    }

    fn parents(&self) -> Vec<Box<dyn MigrationTrait<MySql>>> {
        vec![]
    }

    fn operations(&self) -> Vec<Box<dyn OperationTrait<MySql>>> {
        vec![Box::new(M0001Operation)]
    }
}
