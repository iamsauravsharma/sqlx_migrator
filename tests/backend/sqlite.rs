//! `SQLite` harness: a file per test so two connections share a database, no
//! lock, transactional DDL. Needs no external service, so it always runs.

use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{ConnectOptions as _, Sqlite, SqliteConnection};
use sqlx_migrator::migrator::LockStrategy;
use sqlx_migrator::{Dialect, Migrator};

use crate::common::{Harness, backend_tests, chain, sqlite_path};

struct Lite;

async fn connect(test: &str) -> SqliteConnection {
    SqliteConnectOptions::new()
        .filename(sqlite_path(test))
        .create_if_missing(true)
        .connect()
        .await
        .unwrap()
}

#[async_trait::async_trait]
impl Harness for Lite {
    type Db = Sqlite;

    async fn setup(&self, test: &str) -> SqliteConnection {
        let _ = std::fs::remove_file(sqlite_path(test));
        connect(test).await
    }

    async fn connect(&self, test: &str) -> SqliteConnection {
        connect(test).await
    }

    fn teardown(&self, test: &str) {
        let _ = std::fs::remove_file(sqlite_path(test));
    }

    fn migrator(&self, test: &str) -> Migrator<Sqlite> {
        let mut migrator = Migrator::<Sqlite>::new(Dialect::sqlite_with(Some(test), None).unwrap());
        migrator.add_migrations(chain(test)).unwrap();
        migrator
    }

    fn row_lock_migrator(&self, test: &str) -> Migrator<Sqlite> {
        let dialect = Dialect::sqlite_with(Some(test), None)
            .unwrap()
            .set_lock_strategy(LockStrategy::Row)
            .unwrap();
        let mut migrator = Migrator::<Sqlite>::new(dialect);
        migrator.add_migrations(chain(test)).unwrap();
        migrator
    }

    fn legacy_table_sql(&self, table: &str) -> String {
        format!(
            "CREATE TABLE {table} (id INTEGER PRIMARY KEY AUTOINCREMENT, app TEXT NOT NULL, name \
             TEXT NOT NULL, applied_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, UNIQUE \
             (app, name))"
        )
    }

    fn lock_statements(&self) -> Option<(&'static str, &'static str)> {
        None
    }

    fn transactional_ddl(&self) -> bool {
        true
    }
}

backend_tests!(Lite, "lite");
