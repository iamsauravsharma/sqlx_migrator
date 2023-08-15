//! Migrator module
//!
//! It contains common enum and trait for implementing migrator for sqlx
//! supported database
//!
//! It also provides its own struct Migrator which supports Postgres, Sqlite and
//! Mysql Database only
//!
//! # Example
//! Create own custom Migrator which only supports postgres and uses own unique
//! table name instead of default table name
//!
//! ```rust,no_run
//! use std::collections::HashSet;
//!
//! use sqlx::{Pool, Postgres};
//! use sqlx_migrator::error::Error;
//! use sqlx_migrator::migration::{AppliedMigrationSqlRow, Migration};
//! use sqlx_migrator::migrator::{DatabaseOperation, Info, Migrate};
//!
//! #[derive(Default)]
//! pub struct CustomMigrator {
//!     migrations: HashSet<Box<dyn Migration<Postgres>>>,
//! }
//!
//! impl Info<Postgres> for CustomMigrator {
//!     fn migrations(&self) -> &HashSet<Box<dyn Migration<Postgres>>> {
//!         &self.migrations
//!     }
//!
//!     fn migrations_mut(&mut self) -> &mut HashSet<Box<dyn Migration<Postgres>>> {
//!         &mut self.migrations
//!     }
//! }
//! #[async_trait::async_trait]
//! impl DatabaseOperation<Postgres> for CustomMigrator {
//!     async fn ensure_migration_table_exists(
//!         &self,
//!         connection: &mut <Postgres as sqlx::Database>::Connection,
//!     ) -> Result<(), Error> {
//!         sqlx::query(
//!             "CREATE TABLE IF NOT EXISTS _custom_table_name (
//!         id INT PRIMARY KEY NOT NULL GENERATED ALWAYS AS IDENTITY,
//!         app TEXT NOT NULL,
//!         name TEXT NOT NULL,
//!         applied_time TIMESTAMPTZ NOT NULL DEFAULT now(),
//!         UNIQUE (app, name)
//!     )",
//!         )
//!         .execute(connection)
//!         .await?;
//!         Ok(())
//!     }
//!
//!     async fn drop_migration_table_if_exists(
//!         &self,
//!         connection: &mut <Postgres as sqlx::Database>::Connection,
//!     ) -> Result<(), Error> {
//!         sqlx::query("DROP TABLE IF EXISTS _custom_table_name")
//!             .execute(connection)
//!             .await?;
//!         Ok(())
//!     }
//!
//!     async fn add_migration_to_db_table(
//!         &self,
//!         migration: &Box<dyn Migration<Postgres>>,
//!         connection: &mut <Postgres as sqlx::Database>::Connection,
//!     ) -> Result<(), Error> {
//!         sqlx::query("INSERT INTO _custom_table_name(app, name) VALUES ($1, $2)")
//!             .bind(migration.app())
//!             .bind(migration.name())
//!             .execute(connection)
//!             .await?;
//!         Ok(())
//!     }
//!
//!     async fn delete_migration_from_db_table(
//!         &self,
//!         migration: &Box<dyn Migration<Postgres>>,
//!         connection: &mut <Postgres as sqlx::Database>::Connection,
//!     ) -> Result<(), Error> {
//!         sqlx::query("DELETE FROM _custom_table_name WHERE app = $1 AND name = $2")
//!             .bind(migration.app())
//!             .bind(migration.name())
//!             .execute(connection)
//!             .await?;
//!         Ok(())
//!     }
//!
//!     async fn fetch_applied_migration_from_db(
//!         &self,
//!         connection: &mut <Postgres as sqlx::Database>::Connection,
//!     ) -> Result<Vec<AppliedMigrationSqlRow>, Error> {
//!         Ok(sqlx::query_as::<_, AppliedMigrationSqlRow>(
//!             "SELECT id, app, name, applied_time FROM _custom_table_name",
//!         )
//!         .fetch_all(connection)
//!         .await?)
//!     }
//!
//!     async fn lock(
//!         &self,
//!         connection: &mut <Postgres as sqlx::Database>::Connection,
//!     ) -> Result<(), Error> {
//!         let (database_name,): (String,) = sqlx::query_as("SELECT CURRENT_DATABASE()")
//!             .fetch_one(&mut *connection)
//!             .await?;
//!         let lock_id = i64::from(crc32fast::hash(database_name.as_bytes()));
//!         sqlx::query("SELECT pg_advisory_lock($1)")
//!             .bind(lock_id)
//!             .execute(connection)
//!             .await?;
//!         Ok(())
//!     }
//!
//!     async fn unlock(
//!         &self,
//!         connection: &mut <Postgres as sqlx::Database>::Connection,
//!     ) -> Result<(), Error> {
//!         let (database_name,): (String,) = sqlx::query_as("SELECT CURRENT_DATABASE()")
//!             .fetch_one(&mut *connection)
//!             .await?;
//!         let lock_id = i64::from(crc32fast::hash(database_name.as_bytes()));
//!         sqlx::query("SELECT pg_advisory_unlock($1)")
//!             .bind(lock_id)
//!             .execute(connection)
//!             .await?;
//!         Ok(())
//!     }
//! }
//!
//! impl Migrate<Postgres> for CustomMigrator {}
//! ```

use std::collections::{HashMap, HashSet};

use sqlx::{Connection, Pool};

use crate::error::Error;
use crate::migration::{AppliedMigrationSqlRow, Migration};

/// Any database module which support mysql, sqlite and postgres by default
#[cfg(all(
    any(feature = "postgres", feature = "mysql", feature = "sqlite"),
    feature = "any"
))]
mod any;

/// Module for mysql
#[cfg(feature = "mysql")]
mod mysql;

/// Module for sqlite
#[cfg(feature = "sqlite")]
mod sqlite;

/// Module for postgres
#[cfg(feature = "postgres")]
mod postgres;

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
/// return data as well as add migrations
pub trait Info<DB>
where
    DB: sqlx::Database,
{
    /// Return migrations
    fn migrations(&self) -> &HashSet<Box<dyn Migration<DB>>>;

    /// Return mutable reference of migrations
    fn migrations_mut(&mut self) -> &mut HashSet<Box<dyn Migration<DB>>>;

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
    async fn ensure_migration_table_exists(
        &self,
        connection: &mut <DB as sqlx::Database>::Connection,
    ) -> Result<(), Error>;

    /// Drop migration table if migration table exists
    async fn drop_migration_table_if_exists(
        &self,
        connection: &mut <DB as sqlx::Database>::Connection,
    ) -> Result<(), Error>;

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
    async fn fetch_applied_migration_from_db(
        &self,
        connection: &mut <DB as sqlx::Database>::Connection,
    ) -> Result<Vec<AppliedMigrationSqlRow>, Error>;

    /// Lock database while doing migrations so no two migrations run together
    async fn lock(&self, connection: &mut <DB as sqlx::Database>::Connection) -> Result<(), Error>;

    /// Unlock locked database
    async fn unlock(
        &self,
        connection: &mut <DB as sqlx::Database>::Connection,
    ) -> Result<(), Error>;
}

/// Migrate trait which migrate a database according to requirements. This trait
/// implements all methods which depends on DatabaseOperation trait and Info
/// trait. This trait doesn't requires to implement any method since all
/// function have default implementation
#[async_trait::async_trait]
pub trait Migrate<DB>: Info<DB> + DatabaseOperation<DB> + Send + Sync
where
    DB: sqlx::Database,
{
    /// List all applied migrations. Returns a vector of migration
    async fn list_applied_migrations(
        &self,
        connection: &mut <DB as sqlx::Database>::Connection,
    ) -> MigrationVecResult<DB> {
        if cfg!(feature = "tracing") {
            tracing::info!("Fetching applied migrations");
        }
        self.ensure_migration_table_exists(connection).await?;
        let applied_migration_list = self.fetch_applied_migration_from_db(connection).await?;

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
    async fn generate_migration_plan(
        &self,
        plan: Plan,
        connection: &mut <DB as sqlx::Database>::Connection,
    ) -> MigrationVecResult<DB> {
        let applied_migrations = self.list_applied_migrations(connection).await?;

        if cfg!(feature = "tracing") {
            tracing::info!("Generating {:?} migration plan", plan);
        }

        let mut migration_plan = Vec::new();

        // Hashmap which contains key as migration name and value as list of migration
        // which becomes parent for key due to value having key as run before value
        let mut parents_due_to_run_before = HashMap::<_, Vec<_>>::new();

        for migration in self.migrations() {
            for run_before_migration in migration.run_before() {
                parents_due_to_run_before
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
                let all_run_before_parents_added = parents_due_to_run_before
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

        // Remove migration from migration plan according to replaces vector
        for migration in migration_plan.clone() {
            // Only need to check case when replaces contain value otherwise logic can be
            // ignored
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
                let Some(pos) = migration_plan
                    .iter()
                    .rposition(|migration| migration.app() == app && migration.name() == name)
                else {
                    if migration_plan
                        .iter()
                        .any(|migration| migration.app() == app)
                    {
                        return Err(Error::MigrationNameNotExists {
                            app,
                            migration: name,
                        });
                    }
                    return Err(Error::AppNameNotExists { app });
                };
                pos
            } else {
                let Some(pos) = migration_plan
                    .iter()
                    .rposition(|migration| migration.app() == app)
                else {
                    return Err(Error::AppNameNotExists { app });
                };
                pos
            };
            migration_plan.truncate(position + 1);
        }
        Ok(migration_plan)
    }

    /// Apply all migrations which are not applied till now
    ///
    /// # Errors
    /// If failed to apply migration
    async fn apply_all(&self, pool: &Pool<DB>) -> Result<(), Error> {
        let mut connection = pool.acquire().await?;
        if cfg!(feature = "tracing") {
            tracing::info!("Applying all migration");
        }
        self.lock(&mut connection).await?;
        let plan = Plan::new(PlanType::Apply, None, None)?;
        for migration in self.generate_migration_plan(plan, &mut connection).await? {
            self.apply_migration(migration, &mut connection).await?;
        }
        self.unlock(&mut connection).await?;
        connection.close().await?;
        Ok(())
    }

    /// Apply given migration and add it to applied migration table
    #[allow(clippy::borrowed_box)]
    async fn apply_migration(
        &self,
        migration: &Box<dyn Migration<DB>>,
        connection: &mut <DB as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        if cfg!(feature = "tracing") {
            tracing::info!(
                "Applying {} migration {}",
                migration.app(),
                migration.name()
            );
        }
        if migration.is_atomic() {
            let mut transaction = connection.begin().await?;
            for operation in migration.operations() {
                operation.up(&mut transaction).await?;
            }
            self.add_migration_to_db_table(migration, &mut transaction)
                .await?;
            transaction.commit().await?;
        } else {
            for operation in migration.operations() {
                operation.up(connection).await?;
            }
            self.add_migration_to_db_table(migration, connection)
                .await?;
        }
        Ok(())
    }

    /// Revert all applied migration from database
    ///
    /// # Errors
    /// If any migration or operation fails
    async fn revert_all(&self, pool: &Pool<DB>) -> Result<(), Error> {
        let mut connection = pool.acquire().await?;
        if cfg!(feature = "tracing") {
            tracing::info!("Reverting all migration");
        }
        self.lock(&mut connection).await?;
        let plan = Plan::new(PlanType::Revert, None, None)?;
        for migration in self.generate_migration_plan(plan, &mut connection).await? {
            self.revert_migration(migration, &mut connection).await?;
        }
        self.unlock(&mut connection).await?;
        connection.close().await?;
        Ok(())
    }

    /// Revert provided migration and remove migration from table
    #[allow(clippy::borrowed_box)]
    async fn revert_migration(
        &self,
        migration: &Box<dyn Migration<DB>>,
        connection: &mut <DB as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
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
            let mut transaction = connection.begin().await?;
            for operation in operations {
                operation.down(&mut transaction).await?;
            }
            self.delete_migration_from_db_table(migration, &mut transaction)
                .await?;
            transaction.commit().await?;
        } else {
            for operation in operations {
                operation.down(connection).await?;
            }
            self.delete_migration_from_db_table(migration, connection)
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
    prefix: Option<String>,
}

impl<DB> Migrator<DB>
where
    DB: sqlx::Database,
{
    /// Use prefix for migrator table name only ascii alpha numeric and
    /// underscore characters are supported for table name. prefix will set
    /// table name as `_{prefix}{default_table_name}` where default table
    /// name is `_sqlx_migrator_migrations`
    ///
    /// # Errors
    /// When passed prefix is not ascii alpha numeric or underscore character
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Result<Self, Error> {
        let prefix = prefix.into();
        if !prefix
            .chars()
            .all(|c| char::is_ascii_alphanumeric(&c) || c == '_')
        {
            return Err(Error::NonAsciiAlphaNumeric);
        }
        self.prefix = Some(prefix);
        Ok(self)
    }

    #[cfg(any(feature = "postgres", feature = "mysql", feature = "sqlite"))]
    pub(crate) fn table_name(&self) -> String {
        let default_table_name = "_sqlx_migrator_migrations".to_string();
        if let Some(prefix) = &self.prefix {
            format!("_{prefix}{default_table_name}")
        } else {
            default_table_name
        }
    }
}

impl<DB> Default for Migrator<DB>
where
    DB: sqlx::Database,
{
    fn default() -> Self {
        Self {
            migrations: HashSet::default(),
            prefix: None,
        }
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
}

impl<DB> Migrate<DB> for Migrator<DB>
where
    DB: sqlx::Database,
    Self: DatabaseOperation<DB>,
{
}
