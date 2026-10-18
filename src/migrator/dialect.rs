//! SQL as data: a [`Dialect`] holds the SQL the migrator runs against one
//! database, and [`SupportedDatabase`] executes it for any sqlx database.

use std::fmt;
use std::marker::PhantomData;

use sqlx::{AssertSqlSafe, Database, Executor, IntoArguments};

use super::internal_migration::InternalMigrations;
use crate::error::Error;
use crate::migration::AppliedMigrationSqlRow;

/// SQL of one backend with table names filled in
#[derive(Debug, Clone)]
pub(crate) struct Sql {
    /// Creates the schema if missing, `None` without a schema or on backends
    /// which cannot create one
    pub(crate) create_schema: Option<String>,
    /// Internal migrations which create and change the migrator tables
    pub(crate) internal: InternalMigrations,
    /// Drops the migration and meta tables if they exist
    pub(crate) drop_tables: String,
    /// Selects the applied migration rows as `id, app, name, applied_time`
    pub(crate) fetch_rows: String,
    /// Records an applied migration. Binds app, name, execution time in
    /// milliseconds and batch
    pub(crate) add_migration: String,
    /// Records a migration which started running as failed. Binds app, name
    /// and batch
    pub(crate) start_migration: String,
    /// Marks a running migration as successful. Binds execution time in
    /// milliseconds, app then name
    pub(crate) finish_migration: String,
    /// Removes an applied migration. Binds app then name
    pub(crate) delete_migration: String,
    /// Removes the migrations which started running and never finished
    pub(crate) delete_failed_migrations: String,
    /// Statements of the advisory lock, `None` on backends without one
    pub(crate) lock: Option<Lock>,
}

const DEFAULT_TABLE_NAME: &str = "_sqlx_migrator_migrations";
const DEFAULT_META_TABLE_NAME: &str = "_sqlx_migrator_meta";

/// Validated table prefix and schema the SQL of a backend is built for
#[derive(Debug, Clone, Default)]
pub(crate) struct Names {
    /// Table prefix
    table_prefix: Option<String>,
    /// Schema the tables live in
    schema: Option<String>,
}

impl Names {
    /// Names for an optional table prefix and schema
    ///
    /// # Errors
    /// When the table prefix or the schema contains invalid characters
    pub(crate) fn new(table_prefix: Option<&str>, schema: Option<&str>) -> Result<Self, Error> {
        let is_name_char = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_';
        if table_prefix.is_some_and(|prefix| prefix.is_empty() || !prefix.chars().all(is_name_char))
        {
            return Err(Error::InvalidTablePrefix);
        }
        if schema.is_some_and(|schema| {
            !schema.starts_with(|c: char| c.is_ascii_lowercase() || c == '_')
                || !schema.chars().all(is_name_char)
        }) {
            return Err(Error::InvalidSchema);
        }
        Ok(Self {
            table_prefix: table_prefix.map(str::to_string),
            schema: schema.map(str::to_string),
        })
    }

    /// Table prefix, if any
    pub(crate) fn table_prefix(&self) -> Option<&str> {
        self.table_prefix.as_deref()
    }

    /// Schema the tables live in, if any
    pub(crate) fn schema(&self) -> Option<&str> {
        self.schema.as_deref()
    }

    /// Applies the prefix and schema to a base table name
    fn qualified(&self, base: &str) -> String {
        let name = self
            .table_prefix
            .as_ref()
            .map_or(base.to_string(), |prefix| format!("_{prefix}{base}"));
        self.schema
            .as_ref()
            .map_or(name.clone(), |schema| format!("{schema}.{name}"))
    }

    /// Name of the migration table
    pub(crate) fn table(&self) -> String {
        self.qualified(DEFAULT_TABLE_NAME)
    }

    /// Name of the meta table
    pub(crate) fn meta_table(&self) -> String {
        self.qualified(DEFAULT_META_TABLE_NAME)
    }
}

/// Advisory lock held while a plan runs.
#[derive(Debug, Clone)]
pub(crate) struct Lock {
    acquire: String,
    release: String,
}

impl Lock {
    /// Lock from a statement which takes it, waiting until it is free, and
    /// one which releases it
    #[cfg(any(feature = "postgres", feature = "mysql"))]
    pub(crate) fn new(acquire: impl Into<String>, release: impl Into<String>) -> Self {
        Self {
            acquire: acquire.into(),
            release: release.into(),
        }
    }

    /// Statement which takes the lock
    pub(crate) fn acquire(&self) -> &str {
        &self.acquire
    }

    /// Statement which releases the lock
    pub(crate) fn release(&self) -> &str {
        &self.release
    }
}

/// Configuration of the migrator for database `DB`: table names, advisory
/// lock and DDL behaviour, together with the SQL built for them.
///
/// Use [`Dialect::postgres`], [`Dialect::mysql`] or [`Dialect::sqlite`] for the
/// built in configuration, or the matching `_with` constructor to also set a
/// table prefix and schema. Those are fixed once the dialect is built. Options
/// which only some databases have, such as the advisory lock, can only be
/// set on the dialect of a database which has them.
pub struct Dialect<DB> {
    names: Names,
    sql: Sql,
    lock: bool,
    create_schema: bool,
    database: PhantomData<DB>,
}

impl<DB> Clone for Dialect<DB> {
    fn clone(&self) -> Self {
        Self {
            names: self.names.clone(),
            sql: self.sql.clone(),
            lock: self.lock,
            create_schema: self.create_schema,
            database: PhantomData,
        }
    }
}

impl<DB> fmt::Debug for Dialect<DB> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Dialect")
            .field("names", &self.names)
            .field("sql", &self.sql)
            .field("lock", &self.lock)
            .field("create_schema", &self.create_schema)
            .finish()
    }
}

impl<DB> Dialect<DB> {
    /// Dialect for `names` whose SQL is built once by `build_sql`
    pub(crate) fn new(names: Names, build_sql: fn(&Names) -> Sql) -> Self {
        let sql = build_sql(&names);
        Self {
            names,
            sql,
            lock: true,
            create_schema: true,
            database: PhantomData,
        }
    }

    /// The SQL run by the migrator
    pub(crate) fn sql(&self) -> &Sql {
        &self.sql
    }

    /// Lock statements when the lock is enabled and the backend has one
    pub(crate) fn lock_statements(&self) -> Option<&Lock> {
        self.sql.lock.as_ref().filter(|_| self.lock)
    }

    /// Statement creating the schema when enabled and the backend has one
    pub(crate) fn create_schema_statement(&self) -> Option<&str> {
        self.sql
            .create_schema
            .as_deref()
            .filter(|_| self.create_schema)
    }

    /// Name of the table which stores applied migrations
    ///
    /// Format depends on the prefix and schema the dialect was built with:
    /// - With schema: `{schema}._sqlx_migrator_migrations`
    /// - With prefix: `_{prefix}_sqlx_migrator_migrations`
    /// - With both: `{schema}._{prefix}_sqlx_migrator_migrations`
    /// - Default: `_sqlx_migrator_migrations`
    #[must_use]
    pub fn table_name(&self) -> String {
        self.names.table()
    }

    /// Name of the meta table which records the internal migrations. It
    /// follows the same prefix and schema rules as [`Dialect::table_name`].
    #[must_use]
    pub fn meta_table_name(&self) -> String {
        self.names.meta_table()
    }

    /// Name of the schema the tables live in, if any
    #[must_use]
    pub fn schema(&self) -> Option<&str> {
        self.names.schema()
    }

    /// Table prefix the table names were built with, if any
    #[must_use]
    pub fn table_prefix(&self) -> Option<&str> {
        self.names.table_prefix()
    }
}

/// Advisory lock option of databases which have advisory locks
#[cfg(any(feature = "postgres", feature = "mysql"))]
macro_rules! lock_option {
    ($database:ty) => {
        impl Dialect<$database> {
            /// Whether the advisory lock is held while a plan runs
            #[must_use]
            pub fn lock(&self) -> bool {
                self.lock
            }

            /// Sets whether the advisory lock is held while a plan runs. Use
            /// `false` behind a transaction pooling proxy such as `PgBouncer`
            #[must_use]
            pub fn set_lock(mut self, lock: bool) -> Self {
                self.lock = lock;
                self
            }
        }
    };
}

#[cfg(feature = "postgres")]
lock_option!(sqlx::Postgres);

#[cfg(feature = "mysql")]
lock_option!(sqlx::MySql);

#[cfg(feature = "postgres")]
impl Dialect<sqlx::Postgres> {
    /// Built in Postgres dialect with default table names
    #[must_use]
    pub fn postgres() -> Self {
        super::postgres::dialect(Names::default())
    }

    /// Built in Postgres dialect with a table prefix and schema, which is
    /// created if missing
    ///
    /// # Errors
    /// When the table prefix or the schema contains invalid characters
    pub fn postgres_with(table_prefix: Option<&str>, schema: Option<&str>) -> Result<Self, Error> {
        Names::new(table_prefix, schema).map(super::postgres::dialect)
    }

    /// Whether the schema is created if missing before the tables are
    /// ensured
    #[must_use]
    pub fn create_schema(&self) -> bool {
        self.create_schema
    }

    /// Sets whether the schema is created if missing. Use `false` when the
    /// schema is managed elsewhere, for example when the database user is not
    /// allowed to create schemas
    #[must_use]
    pub fn set_create_schema(mut self, create_schema: bool) -> Self {
        self.create_schema = create_schema;
        self
    }
}

#[cfg(feature = "mysql")]
impl Dialect<sqlx::MySql> {
    /// Built in MySQL dialect with default table names
    #[must_use]
    pub fn mysql() -> Self {
        super::mysql::dialect(Names::default())
    }

    /// Built in MySQL dialect with a table prefix and schema. The schema
    /// refers to a database which must exist
    ///
    /// # Errors
    /// When the table prefix or the schema contains invalid characters
    pub fn mysql_with(table_prefix: Option<&str>, schema: Option<&str>) -> Result<Self, Error> {
        Names::new(table_prefix, schema).map(super::mysql::dialect)
    }
}

#[cfg(feature = "sqlite")]
impl Dialect<sqlx::Sqlite> {
    /// Built in `SQLite` dialect with default table names. `SQLite`
    /// serialises writers itself, so there is no lock: concurrent migrators
    /// against the same file are not protected
    #[must_use]
    pub fn sqlite() -> Self {
        super::sqlite::dialect(Names::default())
    }

    /// Built in `SQLite` dialect with a table prefix and schema
    ///
    /// # Errors
    /// When the table prefix or the schema contains invalid characters
    pub fn sqlite_with(table_prefix: Option<&str>, schema: Option<&str>) -> Result<Self, Error> {
        Names::new(table_prefix, schema).map(super::sqlite::dialect)
    }
}

/// A sqlx database the migrator can execute a [`Dialect`] against.
///
/// Implemented for every database whose connection runs plain and bound
/// queries, which covers Postgres, MySQL and `SQLite`.
#[async_trait::async_trait]
pub trait SupportedDatabase: Database + Send + Sync {
    /// Runs one or more statements without parameters
    async fn execute(connection: &mut Self::Connection, sql: &str) -> Result<(), Error>;

    /// Runs one statement binding the given text parameters in order
    async fn execute_bound(
        connection: &mut Self::Connection,
        sql: &str,
        binds: &[&str],
    ) -> Result<(), Error>;

    /// Fetches the first column of every row as text
    async fn fetch_text(connection: &mut Self::Connection, sql: &str)
    -> Result<Vec<String>, Error>;

    /// Fetches applied migration rows
    async fn fetch_rows(
        connection: &mut Self::Connection,
        sql: &str,
    ) -> Result<Vec<AppliedMigrationSqlRow>, Error>;
}

#[async_trait::async_trait]
impl<DB> SupportedDatabase for DB
where
    DB: Database + Send + Sync,
    DB::Arguments: IntoArguments<DB>,
    for<'c> &'c mut DB::Connection: Executor<'c, Database = DB>,
    for<'q> String: sqlx::Encode<'q, DB> + sqlx::Type<DB>,
    for<'r> (String,): sqlx::FromRow<'r, DB::Row>,
    for<'r> AppliedMigrationSqlRow: sqlx::FromRow<'r, DB::Row>,
{
    async fn execute(connection: &mut Self::Connection, sql: &str) -> Result<(), Error> {
        sqlx::raw_sql(AssertSqlSafe(sql.to_string()))
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn execute_bound(
        connection: &mut Self::Connection,
        sql: &str,
        binds: &[&str],
    ) -> Result<(), Error> {
        let mut query = sqlx::query(AssertSqlSafe(sql.to_string()));
        for bind in binds {
            query = query.bind((*bind).to_string());
        }
        query.execute(connection).await?;
        Ok(())
    }

    async fn fetch_text(
        connection: &mut Self::Connection,
        sql: &str,
    ) -> Result<Vec<String>, Error> {
        let rows: Vec<(String,)> = sqlx::query_as(AssertSqlSafe(sql.to_string()))
            .fetch_all(connection)
            .await?;
        Ok(rows.into_iter().map(|(value,)| value).collect())
    }

    async fn fetch_rows(
        connection: &mut Self::Connection,
        sql: &str,
    ) -> Result<Vec<AppliedMigrationSqlRow>, Error> {
        Ok(sqlx::query_as(AssertSqlSafe(sql.to_string()))
            .fetch_all(connection)
            .await?)
    }
}
