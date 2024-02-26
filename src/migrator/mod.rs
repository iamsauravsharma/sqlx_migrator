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

use sqlx::Connection;

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

type MigrationVec<'a, DB> = Vec<&'a Box<dyn Migration<DB>>>;
type MigrationVecResult<'a, DB> = Result<MigrationVec<'a, DB>, Error>;

/// Type of plan which needs to be generate
#[derive(Debug)]
pub enum PlanType {
    /// Plan type used when listing migrations which can be applied
    Apply,
    /// Plan type when listing migrations which can be reverted
    Revert,
}

/// Struct which determine type of plan to use
#[derive(Debug)]
pub struct Plan {
    plan_type: PlanType,
    app_migration: Option<(String, Option<String>)>,
    count: Option<usize>,
}

impl Plan {
    fn new(
        plan_type: PlanType,
        app_migration: Option<(String, Option<String>)>,
        count: Option<usize>,
    ) -> Self {
        Self {
            plan_type,
            app_migration,
            count,
        }
    }

    /// Create new plan for apply all
    #[must_use]
    pub fn apply_all() -> Self {
        Self::new(PlanType::Apply, None, None)
    }

    /// Create new plan for apply for provided app and migration name
    #[must_use]
    pub fn apply_name(app: &str, name: &Option<String>) -> Self {
        Self::new(PlanType::Apply, Some((app.to_string(), name.clone())), None)
    }

    /// Create new plan for apply count
    #[must_use]
    pub fn apply_count(count: usize) -> Self {
        Self::new(PlanType::Apply, None, Some(count))
    }

    /// Create new plan for revert all
    #[must_use]
    pub fn revert_all() -> Self {
        Self::new(PlanType::Revert, None, None)
    }

    /// Create new plan for revert for provided app and migration name
    #[must_use]
    pub fn revert_name(app: &str, name: &Option<String>) -> Self {
        Self::new(
            PlanType::Revert,
            Some((app.to_string(), name.clone())),
            None,
        )
    }

    /// Create new plan for revert count
    #[must_use]
    pub fn revert_count(count: usize) -> Self {
        Self::new(PlanType::Revert, None, Some(count))
    }
}

/// Info trait which implements some of database agnostic methods to add
/// migration or returns immutable or mutable migrations list
pub trait Info<DB> {
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
        // Only add parents and replaces migrations if current migration was added first
        // time. This can increase performance of recursive addition by ignoring
        // parent and replace migration recursive addition
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

/// Trait which is implemented for database for performing different
/// operations/action on database. Usually this trait is implemented for
/// database to support database along with info trait
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

    /// Add migration to migration db table
    #[allow(clippy::borrowed_box)]
    async fn add_migration_to_db_table(
        &self,
        migration: &Box<dyn Migration<DB>>,
        connection: &mut <DB as sqlx::Database>::Connection,
    ) -> Result<(), Error>;

    /// Delete migration from migration db table
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

/// Process plan to provided migrations list
fn process_plan<DB>(
    migration_list: &mut MigrationVec<DB>,
    applied_migrations: &MigrationVec<DB>,
    plan: &Plan,
) -> Result<(), Error>
where
    DB: sqlx::Database,
{
    // Modify migration list according to plan type
    match plan.plan_type {
        PlanType::Apply => {
            migration_list.retain(|migration| !applied_migrations.contains(migration));
        }
        PlanType::Revert => {
            migration_list.retain(|migration| applied_migrations.contains(migration));
            migration_list.reverse();
        }
    };

    if let Some((app, migration_name)) = &plan.app_migration {
        // Find position of last migration which matches condition of provided app and
        // migration name
        let position = if let Some(name) = migration_name {
            let Some(pos) = migration_list
                .iter()
                .rposition(|migration| migration.app() == app && migration.name() == name)
            else {
                if migration_list
                    .iter()
                    .any(|migration| migration.app() == app)
                {
                    return Err(Error::MigrationNameNotExists {
                        app: app.to_string(),
                        migration: name.to_string(),
                    });
                }
                return Err(Error::AppNameNotExists {
                    app: app.to_string(),
                });
            };
            pos
        } else {
            let Some(pos) = migration_list
                .iter()
                .rposition(|migration| migration.app() == app)
            else {
                return Err(Error::AppNameNotExists {
                    app: app.to_string(),
                });
            };
            pos
        };
        migration_list.truncate(position + 1);
    } else if let Some(count) = plan.count {
        let actual_len = migration_list.len();
        if count > actual_len {
            return Err(Error::CountGreater { actual_len, count });
        }
        migration_list.truncate(count);
    }
    Ok(())
}

/// Migrate trait which migrate a database according to requirements. This trait
/// implements all methods which depends on `DatabaseOperation` trait and `Info`
/// trait. This trait doesn't requires to implement any method since all
/// function have default implementation and all methods are database agnostics
#[async_trait::async_trait]
pub trait Migrate<DB>: Info<DB> + DatabaseOperation<DB> + Send + Sync
where
    DB: sqlx::Database,
{
    /// Generate migration plan according to plan. Returns a vector of
    /// migration. If plan is none than it will generate plan with all
    /// migrations in chronological order of apply
    async fn generate_migration_plan(
        &self,
        plan: Option<&Plan>,
        connection: &mut <DB as sqlx::Database>::Connection,
    ) -> MigrationVecResult<DB> {
        tracing::debug!("fetching applied migrations");

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
        tracing::debug!("generating {:?} migration plan", plan);

        let mut migration_list = Vec::new();

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

        // Create migration list until migration list length is equal to hash set
        // length
        let migrations_hash_set_len = self.migrations().len();
        while migration_list.len() != migrations_hash_set_len {
            let old_migration_list_length = migration_list.len();
            for migration in self.migrations() {
                if !migration_list.contains(&migration) {
                    // Check if all parents are applied or not
                    let all_parents_applied = migration
                        .parents()
                        .iter()
                        .all(|parent_migration| migration_list.contains(&parent_migration));

                    if all_parents_applied {
                        // Check if all run before parents are added or not
                        let all_run_before_parents_added = parents_due_to_run_before
                            .get(migration)
                            .unwrap_or(&vec![])
                            .iter()
                            .all(|run_before_migration| {
                                migration_list.contains(run_before_migration)
                            });

                        if all_run_before_parents_added {
                            migration_list.push(migration);
                        }
                    }
                }
            }

            // If old migration plan length is equal to current length than no new migration
            // was added. Next loop also will not add migration so return error. This case
            // can only occur when Migration A needs to run before Migration B as
            // well as Migration A has Migration B as parents.
            if old_migration_list_length == migration_list.len() {
                return Err(Error::FailedToCreateMigrationPlan);
            }
        }

        // Remove migration from migration list according to replaces vector
        for migration in migration_list.clone() {
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
                    migration_list.retain(|&plan_migration| migration != plan_migration);
                } else {
                    for replaced_migration in migration.replaces() {
                        migration_list
                            .retain(|&plan_migration| &replaced_migration != plan_migration);
                    }
                }
            }
        }

        if let Some(planned) = plan {
            process_plan(&mut migration_list, &applied_migrations, planned)?;
        }

        Ok(migration_list)
    }

    /// Run provided plan migrations
    ///
    /// # Errors
    /// If failed to run provided plan migrations
    async fn run(
        &self,
        connection: &mut <DB as sqlx::Database>::Connection,
        plan: &Plan,
    ) -> Result<(), Error> {
        tracing::debug!("running plan {:?}", plan);
        self.lock(connection).await?;
        for migration in self.generate_migration_plan(Some(plan), connection).await? {
            match plan.plan_type {
                PlanType::Apply => {
                    tracing::debug!("applying {} : {}", migration.app(), migration.name());
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
                }
                PlanType::Revert => {
                    tracing::debug!("reverting {} : {}", migration.app(), migration.name());

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
                }
            };
        }
        self.unlock(connection).await?;
        Ok(())
    }
}

const DEFAULT_TABLE_NAME: &str = "_sqlx_migrator_migrations";

/// Migrator struct which store migrations graph and information related to
/// different library supported migrations
pub struct Migrator<DB> {
    migrations: HashSet<Box<dyn Migration<DB>>>,
    table_name: String,
}

impl<DB> Migrator<DB> {
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
        self.table_name = format!("_{prefix}{DEFAULT_TABLE_NAME}");
        Ok(self)
    }

    /// Get name of table which is used for storing migrations related
    /// information in database
    #[must_use]
    pub fn table_name(&self) -> &str {
        &self.table_name
    }
}

impl<DB> Default for Migrator<DB> {
    fn default() -> Self {
        Self {
            migrations: HashSet::default(),
            table_name: DEFAULT_TABLE_NAME.to_string(),
        }
    }
}

impl<DB> Info<DB> for Migrator<DB> {
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
