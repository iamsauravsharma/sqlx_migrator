//! Postgres harness: schema and prefix per test, advisory lock,
//! transactional DDL.

use sqlx::{Connection as _, PgConnection, Postgres};
use sqlx_migrator::error::Error;
use sqlx_migrator::migrator::{LockStrategy, Plan};
use sqlx_migrator::{Dialect, Migrator};

use crate::common::{
    Harness, SqlMigration, applied, backend_tests, chain, database_url, table_exists, tables,
};

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

    fn row_lock_migrator(&self, test: &str) -> Migrator<Postgres> {
        let dialect = Dialect::postgres_with(Some(test), Some(test))
            .unwrap()
            .set_lock_strategy(LockStrategy::Row)
            .unwrap();
        let mut migrator = Migrator::<Postgres>::new(dialect);
        migrator.add_migrations(chain(test)).unwrap();
        migrator
    }

    fn legacy_table_sql(&self, table: &str) -> String {
        format!(
            "CREATE TABLE {table} (id INT PRIMARY KEY NOT NULL GENERATED ALWAYS AS IDENTITY, app \
             TEXT NOT NULL, name TEXT NOT NULL, applied_time TIMESTAMPTZ NOT NULL DEFAULT now(), \
             UNIQUE (app, name))"
        )
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

/// Locks which hold the plan in one transaction, named for the test prefixes
const TRANSACTION_LOCKS: [(&str, LockStrategy); 3] = [
    ("advisory", LockStrategy::TransactionAdvisory),
    ("share", LockStrategy::TableShareUpdateExclusive),
    ("access", LockStrategy::TableAccessExclusive),
];

/// Migrator with a transaction lock, running `chain` and then the given
/// migration
fn transaction_lock_migrator(
    test: &str,
    strategy: LockStrategy,
    last: SqlMigration,
) -> Migrator<Postgres> {
    let dialect = Dialect::postgres_with(Some(test), Some(test))
        .unwrap()
        .set_lock_strategy(strategy)
        .unwrap();
    let mut migrator = Migrator::<Postgres>::new(dialect);
    migrator.add_migrations(chain(test)).unwrap();
    migrator.add_migration(Box::new(last)).unwrap();
    migrator
}

/// Migration creating a table after the second one of `chain`
fn creating(test: &str) -> SqlMigration {
    SqlMigration::new("main", "third")
        .parent("main", "second")
        .operation(
            &format!("CREATE TABLE {test}_failed (id INTEGER PRIMARY KEY)"),
            &format!("DROP TABLE {test}_failed"),
        )
}

/// With a transaction lock a plan is applied entirely or not at all.
#[tokio::test]
#[ignore = "needs POSTGRES_DATABASE_URL"]
async fn transaction_lock_applies_the_plan_in_one_transaction() {
    for (name, strategy) in TRANSACTION_LOCKS {
        let test = format!("tpg_txplan_{name}");
        let mut connection = Pg.setup(&test).await;
        let failing = SqlMigration::new("main", "third")
            .parent("main", "second")
            .operation("THIS IS NOT SQL", "SELECT 1");
        let migrator = transaction_lock_migrator(&test, strategy, failing);

        assert!(matches!(
            migrator.run(&mut connection, &Plan::apply_all()).await,
            Err(Error::Sqlx(_))
        ));
        assert_eq!(
            applied(&mut connection, &migrator).await,
            Vec::<String>::new()
        );
        assert!(!table_exists::<Postgres>(&mut connection, &format!("{test}_first")).await);

        let migrator = transaction_lock_migrator(&test, strategy, creating(&test));
        let report = migrator
            .run(&mut connection, &Plan::apply_all())
            .await
            .unwrap();
        assert_eq!(report.len(), 3);
        let report = migrator
            .run(&mut connection, &Plan::revert_all())
            .await
            .unwrap();
        assert_eq!(report.len(), 3);
    }
}

/// The plan is one transaction, so a migration which cannot run in a
/// transaction is refused before anything runs.
#[tokio::test]
#[ignore = "needs POSTGRES_DATABASE_URL"]
async fn transaction_lock_refuses_non_atomic_migrations() {
    for (name, strategy) in TRANSACTION_LOCKS {
        let test = format!("tpg_txatomic_{name}");
        let mut connection = Pg.setup(&test).await;
        let migrator = transaction_lock_migrator(&test, strategy, creating(&test).non_atomic());
        assert!(matches!(
            migrator.run(&mut connection, &Plan::apply_all()).await,
            Err(Error::PlanError { .. })
        ));
        assert_eq!(
            applied(&mut connection, &migrator).await,
            Vec::<String>::new()
        );
        // a fake run only records migrations, so it is allowed
        migrator
            .run(&mut connection, &Plan::apply_all().fake(true))
            .await
            .unwrap();
    }
}

/// Two runs at once on a new database do not conflict: the second waits for
/// the lock and then finds everything applied.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs POSTGRES_DATABASE_URL"]
async fn transaction_lock_serialises_concurrent_runs() {
    for (name, strategy) in TRANSACTION_LOCKS {
        let test = format!("tpg_txrace_{name}");
        let mut first = Pg.setup(&test).await;
        let mut second = Pg.connect(&test).await;
        let migrator =
            std::sync::Arc::new(transaction_lock_migrator(&test, strategy, creating(&test)));
        let other = std::sync::Arc::clone(&migrator);
        let handle =
            tokio::spawn(async move { other.run(&mut second, &Plan::apply_all()).await.unwrap() });
        let report = migrator.run(&mut first, &Plan::apply_all()).await.unwrap();
        let other_report = handle.await.unwrap();
        assert_eq!(report.len() + other_report.len(), 3);
        assert_eq!(
            applied(&mut first, &migrator).await,
            vec!["first", "second", "third"]
        );
    }
}

/// A run which waits for the lock sees what the run holding it created, even
/// though it looked for the schema before, which `Pg::setup` does.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs POSTGRES_DATABASE_URL"]
async fn transaction_lock_sees_what_the_holder_created() {
    for (name, strategy) in TRANSACTION_LOCKS {
        let test = format!("tpg_txseen_{name}");
        let mut waiting = Pg.setup(&test).await;
        let mut holder = Pg.connect(&test).await;
        let migrator =
            std::sync::Arc::new(transaction_lock_migrator(&test, strategy, creating(&test)));
        // hold the lock the run asks for and create the schema while it waits
        for statement in [
            "BEGIN".to_string(),
            format!(
                "SELECT pg_advisory_xact_lock(hashtext('{}'))",
                migrator.table_name()
            ),
            format!("CREATE SCHEMA IF NOT EXISTS {test}"),
        ] {
            sqlx::raw_sql(sqlx::AssertSqlSafe(statement))
                .execute(&mut holder)
                .await
                .unwrap();
        }
        let run = {
            let migrator = std::sync::Arc::clone(&migrator);
            tokio::spawn(async move { migrator.run(&mut waiting, &Plan::apply_all()).await })
        };
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        assert!(!run.is_finished(), "run should wait for the lock");
        sqlx::raw_sql("COMMIT").execute(&mut holder).await.unwrap();
        let report = run.await.unwrap().unwrap();
        assert_eq!(report.len(), 3);
    }
}
