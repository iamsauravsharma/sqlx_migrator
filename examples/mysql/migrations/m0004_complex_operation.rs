use sqlx::{MySql, MySqlConnection};
use sqlx_migrator::error::Error;
use sqlx_migrator::migration::Migration;
use sqlx_migrator::operation::Operation;

pub(crate) struct M0004Operation {
    id: i32,
    message: String,
}

#[async_trait::async_trait]
impl Operation<MySql> for M0004Operation {
    async fn up(&self, connection: &mut MySqlConnection) -> Result<(), Error> {
        sqlx::query("INSERT INTO sample (id, name) VALUES (?, ?)")
            .bind(self.id)
            .bind(&self.message)
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn down(&self, connection: &mut MySqlConnection) -> Result<(), Error> {
        sqlx::query("DELETE FROM sample WHERE id = ?")
            .bind(self.id)
            .execute(connection)
            .await?;
        Ok(())
    }
}

pub(crate) struct M0004Migration {
    pub(crate) id: i32,
    pub(crate) message: String,
}

impl Migration<MySql> for M0004Migration {
    fn app(&self) -> &'static str {
        "main"
    }

    fn name(&self) -> &'static str {
        "m0004_complex_operation"
    }

    fn parents(&self) -> Vec<Box<dyn Migration<MySql>>> {
        vec![Box::new(
            crate::migrations::m0003_use_macros::M0003Migration,
        )]
    }

    fn operations(&self) -> Vec<Box<dyn Operation<MySql>>> {
        vec![Box::new(M0004Operation {
            id: self.id,
            message: self.message.clone(),
        })]
    }
}
