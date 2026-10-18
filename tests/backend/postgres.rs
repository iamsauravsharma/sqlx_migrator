//! Postgres harness: schema and prefix per test, advisory lock,
//! transactional DDL.

use sqlx::{Connection as _, PgConnection, Postgres};
use sqlx_migrator::{Dialect, Migrator};

use crate::common::{Harness, backend_tests, chain, database_url, tables};

const URL: &str = "POSTGRES_DATABASE_URL";

struct Pg;

#[async_trait::async_trait]
impl Harness for Pg {
    type Db = Postgres;

    async fn setup(&self, test: &str) -> PgConnection {
        let mut connection = PgConnection::connect(&database_url(URL)).await.unwrap();
        for table in tables(test) {
            sqlx::query(sqlx::AssertSqlSafe(format!("DROP TABLE IF EXISTS {table}")))
                .execute(&mut connection)
                .await
                .unwrap();
        }
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA IF EXISTS {test} CASCADE"
        )))
        .execute(&mut connection)
        .await
        .unwrap();
        connection
    }

    async fn connect(&self, _test: &str) -> PgConnection {
        PgConnection::connect(&database_url(URL)).await.unwrap()
    }

    fn migrator(&self, test: &str) -> Migrator<Postgres> {
        let mut migrator =
            Migrator::<Postgres>::new(Dialect::postgres_with(Some(test), Some(test)).unwrap());
        migrator.add_migrations(chain(test)).unwrap();
        migrator
    }

    fn table_name(&self, test: &str) -> String {
        format!("{test}._{test}_sqlx_migrator_migrations")
    }

    fn lock_statements(&self) -> Option<(&'static str, &'static str)> {
        Some((
            "SELECT pg_advisory_lock(hashtext($1))",
            "SELECT pg_advisory_unlock(hashtext($1))",
        ))
    }

    fn transactional_ddl(&self) -> bool {
        true
    }
}

// Postgres reserves the `pg_` prefix for schema names
backend_tests!(Pg, "tpg", ignore = "needs POSTGRES_DATABASE_URL");
