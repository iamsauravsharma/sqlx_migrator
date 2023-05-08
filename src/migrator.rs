//! migrator module
//!
//! It contains common enum and trait for implementing migrator for sqlx
//! supported database
//!
//! Currently project supports Postgres, Sqlite and Mysql Database only
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

#[cfg(feature = "mysql")]
use sqlx::MySql;
use sqlx::Pool;
#[cfg(feature = "postgres")]
use sqlx::Postgres;
#[cfg(feature = "sqlite")]
use sqlx::Sqlite;
#[cfg(all(
    any(feature = "postgres", feature = "mysql", feature = "sqlite"),
    feature = "any"
))]
use sqlx::{any::AnyKind, Any};

use crate::error::Error;
use crate::migration::{AppliedMigrationSqlRow, Migration};

type MigrationVecResult<'a, DB> = Result<Vec<&'a Box<dyn Migration<DB>>>, Error>;

/// Type of plan which needs to be generate
#[derive(Debug)]
pub enum PlanType {
    /// Plan type used when listing all migration in chronological order
    All,
    /// Plan type used when listing migrations which can be applied
    Apply,
    /// Plan type when listing migrations which can be reverted
    Revert,
}

/// Struct which determine type of plan to use
#[derive(Debug)]
pub struct Plan {
    plan_type: PlanType,
    app: Option<String>,
    migration: Option<String>,
}

impl Plan {
    /// Create new plan using plan type, app name and migration name
    ///
    /// # Errors
    /// When app value is none and migration value is some value
    pub fn new(
        plan_type: PlanType,
        app: Option<String>,
        migration: Option<String>,
    ) -> Result<Self, Error> {
        if migration.is_some() && app.is_none() {
            return Err(Error::AppNameRequired);
        }
        Ok(Self {
            plan_type,
            app,
            migration,
        })
    }
}

/// Info trait which implements some of database agnostic methods to
/// return data. Only required methods needs to be implemented if you want to
/// create your own migrator struct. This trait implements methods which doesn't
/// depends on Database Operation trait methods
pub trait Info<DB>
where
    DB: sqlx::Database,
{
    /// Return migrations
    fn migrations(&self) -> &HashSet<Box<dyn Migration<DB>>>;

    /// Return mutable reference of migrations
    fn migrations_mut(&mut self) -> &mut HashSet<Box<dyn Migration<DB>>>;

    /// Return pool of database
    fn pool(&self) -> &Pool<DB>;

    /// Add vector of migrations to Migrator object
    fn add_migrations(&mut self, migrations: Vec<Box<dyn Migration<DB>>>) {
        for migration in migrations {
            self.add_migration(migration);
        }
    }

    /// Add single migration to migrator object
    fn add_migration(&mut self, migration: Box<dyn Migration<DB>>) {
        let migration_parents = migration.parents();
        let migration_replaces = migration.replaces();
        let is_new_value = self.migrations_mut().insert(migration);
        // Only add parents and replaces if migrations was added first time. This can
        // increase performance of recursive addition by ignoring parent and replace
        // migration recursive addition
        if is_new_value {
            for parent in migration_parents {
                self.add_migration(parent);
            }
            for replace in migration_replaces {
                self.add_migration(replace);
            }
        }
    }
}

/// Trait which is implemented for database for performing database related
/// actions on database. Usually this trait is implemented for database to
/// support certain database along with info trait
#[async_trait::async_trait]
pub trait DatabaseOperation<DB>
where
    DB: sqlx::Database,
{
    /// Ensure migration table is created before running migrations. If not
    /// create one
    async fn ensure_migration_table_exists(&self) -> Result<(), Error>;

    /// Drop migration table if migration table exists
    async fn drop_migration_table_if_exists(&self) -> Result<(), Error>;

    /// Add migration to migration table
    #[allow(clippy::borrowed_box)]
    async fn add_migration_to_db_table(
        &self,
        migration: &Box<dyn Migration<DB>>,
        connection: &mut <DB as sqlx::Database>::Connection,
    ) -> Result<(), Error>;

    /// Delete migration from migration table
    #[allow(clippy::borrowed_box)]
    async fn delete_migration_from_db_table(
        &self,
        migration: &Box<dyn Migration<DB>>,
        connection: &mut <DB as sqlx::Database>::Connection,
    ) -> Result<(), Error>;

    /// List all applied migrations from database as struct
    async fn fetch_applied_migration_from_db(&self) -> Result<Vec<AppliedMigrationSqlRow>, Error>;

    /// Lock database while doing migrations so no two migrations run together
    async fn lock(&self) -> Result<(), Error>;

    /// Unlock locked database
    async fn unlock(&self) -> Result<(), Error>;
}

/// Migrate trait which migrate a database according to requirements. This trait
/// implements all methods which depends on DatabaseOperation trait. This trait
/// is not required to implement any method since all have provided
/// implementation and good to go for database agnostic case
#[async_trait::async_trait]
pub trait Migrate<DB>: Info<DB> + DatabaseOperation<DB> + Send + Sync
where
    DB: sqlx::Database,
{
    /// List all applied migrations. Returns a vector of migration
    async fn list_applied_migrations(&self) -> MigrationVecResult<DB> {
        if cfg!(feature = "tracing") {
            tracing::info!("Fetching applied migrations");
        }
        self.ensure_migration_table_exists().await?;
        let applied_migration_list = self.fetch_applied_migration_from_db().await?;

        // convert applied migration string name to vector of migration implemented
        // objects
        let mut applied_migrations = Vec::new();
        for migration in self.migrations() {
            if applied_migration_list
                .iter()
                .any(|sqlx_migration| sqlx_migration == migration)
            {
                applied_migrations.push(migration);
            }
        }

        Ok(applied_migrations)
    }

    /// Generate migration plan for according to plan type. Returns a vector of
    /// migration
    async fn generate_migration_plan(&self, plan: Plan) -> MigrationVecResult<DB> {
        let applied_migrations = self.list_applied_migrations().await?;

        if cfg!(feature = "tracing") {
            tracing::info!("Generating {:?} migration plan", plan);
        }

        let mut migration_plan = Vec::new();

        // Hashmap which contains key as migration name and value as list of migration
        // which needs to applied earlier than key according to run_before method of
        // migration
        let mut run_before_migration_hashmap = HashMap::<_, Vec<_>>::new();

        for migration in self.migrations() {
            for run_before_migration in migration.run_before() {
                run_before_migration_hashmap
                    .entry(run_before_migration)
                    .or_default()
                    .push(migration);
            }
        }

        // Create migration plan until migration plan length is equal to hashmap
        // length
        while migration_plan.len() != self.migrations().len() {
            let old_migration_plan_length = migration_plan.len();
            for migration in self.migrations() {
                // Check if all parents are applied or not
                let all_parents_applied = migration
                    .parents()
                    .iter()
                    .all(|migration| migration_plan.contains(&migration));

                // Check if all run before parents are added or not
                let all_run_before_parents_added = run_before_migration_hashmap
                    .get(migration)
                    .unwrap_or(&vec![])
                    .iter()
                    .all(|migration| migration_plan.contains(migration));

                if all_parents_applied
                    && all_run_before_parents_added
                    && !migration_plan.contains(&migration)
                {
                    migration_plan.push(migration);
                }
            }

            // If old migration plan length is equal to current length than no new migration
            // was added. Next loop also will not add migration so return error. This case
            // can only occur when Migration 1 needs to run before Migration 2 as
            // well as Migration 1 has Migration 2 as parents.
            if old_migration_plan_length == migration_plan.len() {
                return Err(Error::FailedToCreateMigrationPlan);
            }
        }

        // Remove migration from migration plan
        for migration in migration_plan.clone() {
            if !migration.replaces().is_empty() {
                // Check if any replaces migration are applied for not
                let replaces_applied = migration
                    .replaces()
                    .iter()
                    .any(|replace_migration| applied_migrations.contains(&replace_migration));

                // If any one of replaced migrations is applied than do not add current
                // migration to migration plan else add only current migration to migration plan
                if replaces_applied {
                    // Error if current migration as well as replace migration both are applied
                    if applied_migrations.contains(&migration) {
                        return Err(Error::BothMigrationTypeApplied);
                    }
                    migration_plan.retain(|&plan_migration| migration != plan_migration);
                } else {
                    for replaced_migration in migration.replaces() {
                        migration_plan
                            .retain(|&plan_migration| &replaced_migration != plan_migration);
                    }
                }
            }
        }

        // Modify migration plan according to plan type
        match plan.plan_type {
            PlanType::Apply => {
                migration_plan.retain(|migration| !applied_migrations.contains(migration));
            }
            PlanType::Revert => {
                migration_plan.retain(|migration| applied_migrations.contains(migration));
                migration_plan.reverse();
            }
            PlanType::All => {}
        };

        if let Some(app) = plan.app {
            // Find position of last migration which matches condition of provided app and
            // migration name
            let position = if let Some(name) = plan.migration {
                let Some(pos) =  migration_plan
                    .iter()
                    .rposition(
                        |migration| migration.app() == app && migration.name() == name
                    )
                    else {
                        if migration_plan.iter().any(|migration| migration.app() == app) {
                            return Err(Error::MigrationNameNotExists { app, migration: name });
                        }
                        return Err(Error::AppNameNotExists { app });
                    };
                pos
            } else {
                let Some(pos) = migration_plan
                    .iter()
                    .rposition(|migration| migration.app() == app) else {
                        return Err(Error::AppNameNotExists { app })
                    };
                pos
            };
            migration_plan.truncate(position + 1);
        }
        Ok(migration_plan)
    }

    /// Apply missing migration
    ///
    /// # Errors
    /// If failed to apply migration
    async fn apply_all(&self) -> Result<(), Error> {
        if cfg!(feature = "tracing") {
            tracing::info!("Applying all migration");
        }
        self.lock().await?;
        for migration in self
            .generate_migration_plan(Plan::new(PlanType::Apply, None, None)?)
            .await?
        {
            self.apply_migration(migration).await?;
        }
        self.unlock().await?;
        Ok(())
    }

    /// Apply given migration and add it to applied migration table
    #[allow(clippy::borrowed_box)]
    async fn apply_migration(&self, migration: &Box<dyn Migration<DB>>) -> Result<(), Error> {
        if cfg!(feature = "tracing") {
            tracing::info!(
                "Applying {} migration {}",
                migration.app(),
                migration.name()
            );
        }
        if migration.is_atomic() {
            let mut transaction = self.pool().begin().await?;
            for operation in migration.operations() {
                operation.up(&mut transaction).await?;
            }
            self.add_migration_to_db_table(migration, &mut transaction)
                .await?;
            transaction.commit().await?;
        } else {
            let mut connection = self.pool().acquire().await?;
            for operation in migration.operations() {
                operation.up(&mut connection).await?;
            }
            self.add_migration_to_db_table(migration, &mut connection)
                .await?;
        }
        Ok(())
    }

    /// Revert all applied migration from database
    ///
    /// # Errors
    /// If any migration or operation fails
    async fn revert_all(&self) -> Result<(), Error> {
        if cfg!(feature = "tracing") {
            tracing::info!("Reverting all migration");
        }
        self.lock().await?;
        for migration in self
            .generate_migration_plan(Plan::new(PlanType::Revert, None, None)?)
            .await?
        {
            self.revert_migration(migration).await?;
        }
        self.unlock().await?;
        Ok(())
    }

    /// Revert provided migration and remove migration from table
    #[allow(clippy::borrowed_box)]
    async fn revert_migration(&self, migration: &Box<dyn Migration<DB>>) -> Result<(), Error> {
        if cfg!(feature = "tracing") {
            tracing::info!(
                "Reverting {} migration {}",
                migration.app(),
                migration.name()
            );
        }

        // Reverse operation since last applied operation need to be reverted first
        let mut operations = migration.operations();
        operations.reverse();

        if migration.is_atomic() {
            let mut transaction = self.pool().begin().await?;
            for operation in operations {
                operation.down(&mut transaction).await?;
            }
            self.delete_migration_from_db_table(migration, &mut transaction)
                .await?;
            transaction.commit().await?;
        } else {
            let mut connection = self.pool().acquire().await?;
            for operation in operations {
                operation.down(&mut connection).await?;
            }
            self.delete_migration_from_db_table(migration, &mut connection)
                .await?;
        }
        Ok(())
    }
}

/// Migrator struct which store migrations graph and information related to
/// different library supported migrations
pub struct Migrator<DB>
where
    DB: sqlx::Database,
{
    migrations: HashSet<Box<dyn Migration<DB>>>,
    pool: Pool<DB>,
    table_name: Option<String>,
}

impl<DB> Migrator<DB>
where
    DB: sqlx::Database,
{
    /// Create new migrator from pool
    #[must_use]
    pub fn new(pool: &Pool<DB>) -> Self {
        Self {
            migrations: HashSet::new(),
            pool: pool.clone(),
            table_name: None,
        }
    }

    fn replace_table_name(&self, query: &'static str) -> Cow<'static, str> {
        if let Some(table_name) = &self.table_name {
            query
                .replace("_sqlx_migrator_migrations", table_name.as_str())
                .into()
        } else {
            query.into()
        }
    }

    /// Set custom migrations table name
    pub fn set_table_name<S: Into<String>>(&mut self, table_name: S) {
        self.table_name = Some(table_name.into());
    }
}

impl<DB> Info<DB> for Migrator<DB>
where
    DB: sqlx::Database,
{
    fn migrations(&self) -> &HashSet<Box<dyn Migration<DB>>> {
        &self.migrations
    }

    fn migrations_mut(&mut self) -> &mut HashSet<Box<dyn Migration<DB>>> {
        &mut self.migrations
    }

    fn pool(&self) -> &Pool<DB> {
        &self.pool
    }
}

#[cfg(all(any(feature = "postgres", feature = "mysql", feature = "sqlite")))]
fn common_drop_table() -> &'static str {
    "DROP TABLE IF EXISTS _sqlx_migrator_migrations"
}

#[cfg(all(any(feature = "postgres", feature = "mysql", feature = "sqlite")))]
fn common_fetch_row() -> &'static str {
    "SELECT id, app, name, applied_time FROM _sqlx_migrator_migrations"
}

#[cfg(all(any(feature = "postgres", feature = "sqlite")))]
fn postgres_sqlite_add_migration() -> &'static str {
    "INSERT INTO _sqlx_migrator_migrations(app, name) VALUES ($1, $2)"
}

#[cfg(all(any(feature = "postgres", feature = "sqlite")))]
fn postgres_sqlite_delete_migration() -> &'static str {
    "DELETE FROM _sqlx_migrator_migrations WHERE app = $1 AND name = $2"
}

#[cfg(feature = "postgres")]
fn postgres_create_migrator_table() -> &'static str {
    "CREATE TABLE IF NOT EXISTS _sqlx_migrator_migrations (
        id INT PRIMARY KEY NOT NULL GENERATED ALWAYS AS IDENTITY,
        app TEXT NOT NULL,
        name TEXT NOT NULL,
        applied_time TIMESTAMPTZ NOT NULL DEFAULT now(),
        UNIQUE (app, name)
    )"
}

#[cfg(feature = "postgres")]
async fn postgres_lock(pool: &Pool<Postgres>) -> Result<(), Error> {
    let database = pool.connect_options().get_database().unwrap_or_default();
    let lock_id = i64::from(crc32fast::hash(database.as_bytes()));
    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(lock_id)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(feature = "postgres")]
async fn postgres_unlock(pool: &Pool<Postgres>) -> Result<(), Error> {
    let database = pool.connect_options().get_database().unwrap_or_default();
    let lock_id = i64::from(crc32fast::hash(database.as_bytes()));
    sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(lock_id)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(feature = "postgres")]
#[async_trait::async_trait]
impl DatabaseOperation<Postgres> for Migrator<Postgres> {
    async fn ensure_migration_table_exists(&self) -> Result<(), Error> {
        sqlx::query(&self.replace_table_name(postgres_create_migrator_table()))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn drop_migration_table_if_exists(&self) -> Result<(), Error> {
        sqlx::query(&self.replace_table_name(common_drop_table()))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn add_migration_to_db_table(
        &self,
        migration: &Box<dyn Migration<Postgres>>,
        connection: &mut <Postgres as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        sqlx::query(&self.replace_table_name(postgres_sqlite_add_migration()))
            .bind(migration.app())
            .bind(migration.name())
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn delete_migration_from_db_table(
        &self,
        migration: &Box<dyn Migration<Postgres>>,
        connection: &mut <Postgres as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        sqlx::query(&self.replace_table_name(postgres_sqlite_delete_migration()))
            .bind(migration.app())
            .bind(migration.name())
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn fetch_applied_migration_from_db(&self) -> Result<Vec<AppliedMigrationSqlRow>, Error> {
        let rows = sqlx::query_as(&self.replace_table_name(common_fetch_row()))
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    async fn lock(&self) -> Result<(), Error> {
        postgres_lock(&self.pool).await
    }

    async fn unlock(&self) -> Result<(), Error> {
        postgres_unlock(&self.pool).await
    }
}

#[cfg(feature = "postgres")]
impl Migrate<Postgres> for Migrator<Postgres> {}

#[cfg(feature = "sqlite")]
fn sqlite_create_migrator_table() -> &'static str {
    "CREATE TABLE IF NOT EXISTS _sqlx_migrator_migrations (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        app TEXT NOT NULL,
        name TEXT NOT NULL,
        applied_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        UNIQUE (app, name)
    )"
}

#[cfg(feature = "sqlite")]
#[async_trait::async_trait]
impl DatabaseOperation<Sqlite> for Migrator<Sqlite> {
    async fn ensure_migration_table_exists(&self) -> Result<(), Error> {
        sqlx::query(&self.replace_table_name(sqlite_create_migrator_table()))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn drop_migration_table_if_exists(&self) -> Result<(), Error> {
        sqlx::query(&self.replace_table_name(common_drop_table()))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn add_migration_to_db_table(
        &self,
        migration: &Box<dyn Migration<Sqlite>>,
        connection: &mut <Sqlite as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        sqlx::query(&self.replace_table_name(postgres_sqlite_add_migration()))
            .bind(migration.app())
            .bind(migration.name())
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn delete_migration_from_db_table(
        &self,
        migration: &Box<dyn Migration<Sqlite>>,
        connection: &mut <Sqlite as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        sqlx::query(&self.replace_table_name(postgres_sqlite_delete_migration()))
            .bind(migration.app())
            .bind(migration.name())
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn fetch_applied_migration_from_db(&self) -> Result<Vec<AppliedMigrationSqlRow>, Error> {
        let rows = sqlx::query_as(&self.replace_table_name(common_fetch_row()))
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    async fn lock(&self) -> Result<(), Error> {
        Ok(())
    }

    async fn unlock(&self) -> Result<(), Error> {
        Ok(())
    }
}

#[cfg(feature = "sqlite")]
impl Migrate<Sqlite> for Migrator<Sqlite> {}

#[cfg(feature = "mysql")]
fn mysql_create_migrator_table() -> &'static str {
    "CREATE TABLE IF NOT EXISTS _sqlx_migrator_migrations (
        id INT PRIMARY KEY NOT NULL AUTO_INCREMENT,
        app VARCHAR(384) NOT NULL,
        name VARCHAR(384) NOT NULL,
        applied_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        UNIQUE (app, name)
    )"
}

#[cfg(feature = "mysql")]
fn mysql_add_migration() -> &'static str {
    "INSERT INTO _sqlx_migrator_migrations(app, name) VALUES (?, ?)"
}

#[cfg(feature = "mysql")]
fn mysql_delete_migration() -> &'static str {
    "DELETE FROM _sqlx_migrator_migrations WHERE app = ? AND name = ?"
}

#[cfg(feature = "mysql")]
async fn mysql_lock(pool: &Pool<MySql>) -> Result<(), Error> {
    let row: (String,) = sqlx::query_as("SELECT DATABASE()").fetch_one(pool).await?;
    let lock_id = crc32fast::hash(row.0.as_bytes()).to_string();
    sqlx::query("SELECT GET_LOCK(?, -1)")
        .bind(lock_id)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(feature = "mysql")]
async fn mysql_unlock(pool: &Pool<MySql>) -> Result<(), Error> {
    let row: (String,) = sqlx::query_as("SELECT DATABASE()").fetch_one(pool).await?;
    let lock_id = crc32fast::hash(row.0.as_bytes()).to_string();
    sqlx::query("SELECT RELEASE_LOCK(?)")
        .bind(lock_id)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(feature = "mysql")]
#[async_trait::async_trait]
impl DatabaseOperation<MySql> for Migrator<MySql> {
    async fn ensure_migration_table_exists(&self) -> Result<(), Error> {
        sqlx::query(&self.replace_table_name(mysql_create_migrator_table()))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn drop_migration_table_if_exists(&self) -> Result<(), Error> {
        sqlx::query(&self.replace_table_name(common_drop_table()))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn add_migration_to_db_table(
        &self,
        migration: &Box<dyn Migration<MySql>>,
        connection: &mut <MySql as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        sqlx::query(&self.replace_table_name(mysql_add_migration()))
            .bind(migration.app())
            .bind(migration.name())
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn delete_migration_from_db_table(
        &self,
        migration: &Box<dyn Migration<MySql>>,
        connection: &mut <MySql as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        sqlx::query(&self.replace_table_name(mysql_delete_migration()))
            .bind(migration.app())
            .bind(migration.name())
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn fetch_applied_migration_from_db(&self) -> Result<Vec<AppliedMigrationSqlRow>, Error> {
        let rows = sqlx::query_as(&self.replace_table_name(common_fetch_row()))
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    async fn lock(&self) -> Result<(), Error> {
        mysql_lock(&self.pool).await
    }

    async fn unlock(&self) -> Result<(), Error> {
        mysql_unlock(&self.pool).await
    }
}

#[cfg(feature = "mysql")]
impl Migrate<MySql> for Migrator<MySql> {}

#[cfg(all(
    any(feature = "postgres", feature = "mysql", feature = "sqlite"),
    feature = "any"
))]
#[async_trait::async_trait]
impl DatabaseOperation<Any> for Migrator<Any> {
    async fn ensure_migration_table_exists(&self) -> Result<(), Error> {
        let pool = &self.pool;
        let sql_query = match pool.any_kind() {
            #[cfg(feature = "postgres")]
            AnyKind::Postgres => self.replace_table_name(postgres_create_migrator_table()).as_ref(),
            #[cfg(feature = "sqlite")]
            AnyKind::Sqlite => self.replace_table_name(sqlite_create_migrator_table()).as_ref(),
            #[cfg(feature = "mysql")]
            AnyKind::MySql => self.replace_table_name(mysql_create_migrator_table()).as_ref(),
        };
        sqlx::query(sql_query).execute(pool).await?;
        Ok(())
    }

    async fn drop_migration_table_if_exists(&self) -> Result<(), Error> {
        sqlx::query(&self.replace_table_name(common_drop_table()))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn add_migration_to_db_table(
        &self,
        migration: &Box<dyn Migration<Any>>,
        connection: &mut <Any as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        let pool = &self.pool;
        let sql_query = match pool.any_kind() {
            #[cfg(feature = "postgres")]
            AnyKind::Postgres => self.replace_table_name(postgres_sqlite_add_migration()).as_ref(),
            #[cfg(feature = "sqlite")]
            AnyKind::Sqlite => self.replace_table_name(postgres_sqlite_add_migration()).as_ref(),
            #[cfg(feature = "mysql")]
            AnyKind::MySql => self.replace_table_name(mysql_add_migration()).as_ref(),
        };
        sqlx::query(sql_query)
            .bind(migration.app())
            .bind(migration.name())
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn delete_migration_from_db_table(
        &self,
        migration: &Box<dyn Migration<Any>>,
        connection: &mut <Any as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        let pool = &self.pool;
        let sql_query = match pool.any_kind() {
            #[cfg(feature = "postgres")]
            AnyKind::Postgres => self.replace_table_name(postgres_sqlite_delete_migration()).as_ref(),
            #[cfg(feature = "sqlite")]
            AnyKind::Sqlite => self.replace_table_name(postgres_sqlite_delete_migration()).as_ref(),
            #[cfg(feature = "mysql")]
            AnyKind::MySql => self.replace_table_name(mysql_delete_migration()).as_ref(),
        };
        sqlx::query(sql_query)
            .bind(migration.app())
            .bind(migration.name())
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn fetch_applied_migration_from_db(&self) -> Result<Vec<AppliedMigrationSqlRow>, Error> {
        let rows = sqlx::query_as(&self.replace_table_name(common_fetch_row()))
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    async fn lock(&self) -> Result<(), Error> {
        let connect_options = self.pool.connect_options();
        match connect_options.kind() {
            #[cfg(feature = "postgres")]
            AnyKind::Postgres => {
                let connect_options = connect_options
                    .as_postgres()
                    .ok_or(Error::FailedDatabaseConversion)?
                    .clone();
                let postgres_pool = Pool::connect_with(connect_options).await?;
                postgres_lock(&postgres_pool).await?;
            }
            #[cfg(feature = "sqlite")]
            AnyKind::Sqlite => {}
            #[cfg(feature = "mysql")]
            AnyKind::MySql => {
                let connect_options = connect_options
                    .as_mysql()
                    .ok_or(Error::FailedDatabaseConversion)?
                    .clone();
                let mysql_pool = Pool::connect_with(connect_options).await?;
                mysql_lock(&mysql_pool).await?;
            }
        };
        Ok(())
    }

    async fn unlock(&self) -> Result<(), Error> {
        let connect_options = self.pool.connect_options();
        match connect_options.kind() {
            #[cfg(feature = "postgres")]
            AnyKind::Postgres => {
                let connect_options = connect_options
                    .as_postgres()
                    .ok_or(Error::FailedDatabaseConversion)?
                    .clone();
                let postgres_pool = Pool::connect_with(connect_options).await?;
                postgres_unlock(&postgres_pool).await?;
            }
            #[cfg(feature = "sqlite")]
            AnyKind::Sqlite => {}
            #[cfg(feature = "mysql")]
            AnyKind::MySql => {
                let connect_options = connect_options
                    .as_mysql()
                    .ok_or(Error::FailedDatabaseConversion)?
                    .clone();
                let mysql_pool = Pool::connect_with(connect_options).await?;
                mysql_unlock(&mysql_pool).await?;
            }
        };
        Ok(())
    }
}

#[cfg(all(
    any(feature = "postgres", feature = "mysql", feature = "sqlite"),
    feature = "any"
))]
impl Migrate<Any> for Migrator<Any> {}
