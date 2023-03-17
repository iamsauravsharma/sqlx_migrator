use sqlx::SqliteConnection;
use sqlx_migrator::error::Error;
use sqlx_migrator::migration::MigrationTrait;
use sqlx_migrator::operation::OperationTrait;
use sqlx_migrator::sqlx::Sqlite;

pub(crate) struct M0001Operation;

#[async_trait::async_trait]
impl OperationTrait<Sqlite> for M0001Operation {
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
impl MigrationTrait<Sqlite> for M0001Migration {
    fn app(&self) -> &str {
        "main"
    }

    fn name(&self) -> &str {
        "m0001_simple"
    }

    fn parents(&self) -> Vec<Box<dyn MigrationTrait<Sqlite>>> {
        vec![]
    }

    fn operations(&self) -> Vec<Box<dyn OperationTrait<Sqlite>>> {
        vec![Box::new(M0001Operation)]
    }
}
