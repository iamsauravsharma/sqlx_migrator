//! MySQL harness: prefix per test (schemas are databases which must exist),
//! named lock, no transactional DDL.

use sqlx::{Connection as _, MySql, MySqlConnection};
use sqlx_migrator::migrator::{Info as _, Migrator};

use crate::common::{Harness, backend_tests, chain, database_url, tables};

const URL: &str = "MYSQL_DATABASE_URL";

struct My;

#[async_trait::async_trait]
impl Harness for My {
    type Db = MySql;

    async fn setup(&self, test: &str) -> MySqlConnection {
        let mut connection = MySqlConnection::connect(&database_url(URL)).await.unwrap();
        for table in tables(test) {
            sqlx::query(sqlx::AssertSqlSafe(format!("DROP TABLE IF EXISTS {table}")))
                .execute(&mut connection)
                .await
                .unwrap();
        }
        connection
    }

    async fn connect(&self, _test: &str) -> MySqlConnection {
        MySqlConnection::connect(&database_url(URL)).await.unwrap()
    }

    fn migrator(&self, test: &str) -> Migrator<MySql> {
        let mut migrator = Migrator::<MySql>::new().set_table_prefix(test).unwrap();
        migrator.add_migrations(chain(test)).unwrap();
        migrator
    }

    fn locks(&self) -> bool {
        true
    }

    fn transactional_ddl(&self) -> bool {
        false
    }
}

backend_tests!([My], "my", ignore = "needs MYSQL_DATABASE_URL");
