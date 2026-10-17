//! `Any` driver harness: the same scenarios dispatched at runtime to each
//! supported database, one module per backend.

use sqlx::{Any, AnyConnection, Connection as _};
use sqlx_migrator::migrator::{Info as _, Migrator};

use crate::common::{Harness, backend_tests, chain, tables};

/// One database the `Any` driver dispatches to
enum AnyDb {
    #[cfg(feature = "sqlite")]
    Sqlite,
    #[cfg(feature = "postgres")]
    Postgres(String),
    #[cfg(feature = "mysql")]
    MySql(String),
}

impl AnyDb {
    /// Backend name sqlx reports for the connection
    fn backend(&self) -> &'static str {
        match self {
            #[cfg(feature = "sqlite")]
            AnyDb::Sqlite => "SQLite",
            #[cfg(feature = "postgres")]
            AnyDb::Postgres(_) => "PostgreSQL",
            #[cfg(feature = "mysql")]
            AnyDb::MySql(_) => "MySQL",
        }
    }

    /// Connection url; `SQLite` gets a file per test so parallel tests never
    /// share or delete each other's database
    fn url(&self, test: &str) -> String {
        match self {
            #[cfg(feature = "sqlite")]
            AnyDb::Sqlite => {
                let path = crate::common::sqlite_path(test)
                    .to_string_lossy()
                    .replace('\\', "/");
                format!("sqlite:{path}?mode=rwc")
            }
            #[cfg(feature = "postgres")]
            AnyDb::Postgres(url) => url.clone(),
            #[cfg(feature = "mysql")]
            AnyDb::MySql(url) => url.clone(),
        }
    }
}

#[async_trait::async_trait]
impl Harness for AnyDb {
    type Db = Any;

    async fn setup(&self, test: &str) -> AnyConnection {
        #[cfg(feature = "sqlite")]
        if matches!(self, AnyDb::Sqlite) {
            let _ = std::fs::remove_file(crate::common::sqlite_path(test));
        }
        let mut connection = AnyConnection::connect(&self.url(test)).await.unwrap();
        assert_eq!(connection.backend_name(), self.backend());
        for table in tables(test) {
            sqlx::query(sqlx::AssertSqlSafe(format!("DROP TABLE IF EXISTS {table}")))
                .execute(&mut connection)
                .await
                .unwrap();
        }
        connection
    }

    async fn connect(&self, test: &str) -> AnyConnection {
        AnyConnection::connect(&self.url(test)).await.unwrap()
    }

    fn teardown(&self, test: &str) {
        #[cfg(feature = "sqlite")]
        if matches!(self, AnyDb::Sqlite) {
            let _ = std::fs::remove_file(crate::common::sqlite_path(test));
        }
    }

    fn migrator(&self, test: &str) -> Migrator<Any> {
        let mut migrator = Migrator::<Any>::new().set_table_prefix(test).unwrap();
        migrator.add_migrations(chain(test)).unwrap();
        migrator
    }

    fn locks(&self) -> bool {
        self.backend() != "SQLite"
    }

    fn transactional_ddl(&self) -> bool {
        self.backend() != "MySQL"
    }
}

/// The `Any` driver needs its backends registered before connecting
fn install() {
    sqlx::any::install_default_drivers();
}

#[cfg(feature = "sqlite")]
mod sqlite {
    use super::{AnyDb, backend_tests, install};

    fn harnesses() -> [AnyDb; 1] {
        install();
        [AnyDb::Sqlite]
    }

    backend_tests!(harnesses(), "anylite");
}

#[cfg(feature = "postgres")]
mod postgres {
    use super::{AnyDb, backend_tests, install};

    fn harnesses() -> [AnyDb; 1] {
        install();
        [AnyDb::Postgres(crate::common::database_url(
            "POSTGRES_DATABASE_URL",
        ))]
    }

    backend_tests!(harnesses(), "anypg", ignore = "needs POSTGRES_DATABASE_URL");
}

#[cfg(feature = "mysql")]
mod mysql {
    use super::{AnyDb, backend_tests, install};

    fn harnesses() -> [AnyDb; 1] {
        install();
        [AnyDb::MySql(crate::common::database_url(
            "MYSQL_DATABASE_URL",
        ))]
    }

    backend_tests!(harnesses(), "anymy", ignore = "needs MYSQL_DATABASE_URL");
}
