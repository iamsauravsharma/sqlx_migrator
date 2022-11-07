//! migrator module
//!
//! It contains common enum and trait for implementing migrator for sqlx
//! supported database
//!
//! For support only some required trait methods need to be implemented. It is
//! best if you do not override provided methods as such method are database
//! agnostic and perform some complex tasks its rarely needs to be custom
//! implemented

use std::collections::{HashMap, HashSet};

use sqlx::{Pool, PgConnection, SqliteConnection};
#[cfg(feature = "postgres")]
use sqlx::Postgres;
#[cfg(feature = "sqlite")]
use sqlx::Sqlite;

use crate::error::Error;
use crate::migration::{AppliedMigrationSqlRow, MigrationTrait};

type MigrationTraitVecResult<'a, DB> = Result<Vec<&'a Box<dyn MigrationTrait<DB>>>, Error>;

/// Type of plan used to generate migrations
#[derive(Debug)]
pub enum Plan {
    /// Full generation plan. Plan containing all migrations according to order
    Full,
    /// Apply generation plan. Plan containing migrations which can be applied
    Apply {
        /// Migration app name which migration needs to be applied
        app: Option<String>,
        /// Migration name till which migration needs to be applied. app should
        /// be Some if it is Some value
        name: Option<String>,
    },
    /// Revert generation plan. Plan containing migrations which can be reverted
    Revert {
        /// Migration app name which migration needs to be reverted
        app: Option<String>,
        /// Migration name till which migration needs to be reverted. app should
        /// be Some if it is Some value
        name: Option<String>,
    },
}

#[allow(clippy::module_name_repetitions)]
#[async_trait::async_trait]
/// Migrator trait
pub trait MigratorTrait<DB>: Send + Sync
where
    DB: sqlx::Database,
{
    /// Return migrations
    fn migrations(&self) -> &HashSet<Box<dyn MigrationTrait<DB>>>;

    /// Return mutable reference of migrations
    fn migrations_mut(&mut self) -> &mut HashSet<Box<dyn MigrationTrait<DB>>>;

    /// Return pool of database
    fn pool(&self) -> &Pool<DB>;

    /// Ensure migration table is created before running migrations. If not
    /// create one
    async fn ensure_migration_table_exists(&self) -> Result<(), Error>;

    /// Add migration to migration table
    #[allow(clippy::borrowed_box)]
    async fn add_migration_to_db_table(
        &self,
        migration: &Box<dyn MigrationTrait<DB>>,
        connection: &mut <DB as sqlx::Database>::Connection,
    ) -> Result<(), Error>;

    /// Delete migration from table
    #[allow(clippy::borrowed_box)]
    async fn delete_migration_from_db_table(
        &self,
        migration: &Box<dyn MigrationTrait<DB>>,
        connection: &mut <DB as sqlx::Database>::Connection,
    ) -> Result<(), Error>;

    /// List all applied migrations from database in string format (full name of
    /// migration)
    async fn fetch_applied_migration_from_db(&self) -> Result<Vec<AppliedMigrationSqlRow>, Error>;

    /// Add vector of migrations to Migrator object
    fn add_migrations(&mut self, migrations: Vec<Box<dyn MigrationTrait<DB>>>) {
        for migration in migrations {
            self.add_migration(migration);
        }
    }

    /// Add single migration to migrator object
    fn add_migration(&mut self, migration: Box<dyn MigrationTrait<DB>>) {
        for parent in migration.parents() {
            self.add_migration(parent);
        }
        self.migrations_mut().insert(migration);
    }

    /// List all applied migrations. Returns a vector of migration
    async fn list_applied_migrations(&self) -> MigrationTraitVecResult<DB> {
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
    async fn generate_migration_plan(&self, plan: Plan) -> MigrationTraitVecResult<DB> {
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
        let temp_migration = migration_plan.clone();
        for migration in &temp_migration {
            if !migration.replaces().is_empty() {
                // Check if any replaces migration are applied for not
                let replaces_applied = migration
                    .replaces()
                    .iter()
                    .any(|migration| applied_migrations.contains(&migration));

                if replaces_applied {
                    migration_plan.retain(|plan_migration| migration != plan_migration);
                } else {
                    for replaced_migration in migration.replaces() {
                        migration_plan
                            .retain(|plan_migration| &&replaced_migration != plan_migration);
                    }
                }
            }
        }

        // Modify migration plan according to plan type
        let (migration_app, migration_name) = match plan {
            Plan::Full => (None, None),
            Plan::Apply { app, name } => {
                migration_plan.retain(|migration| !applied_migrations.contains(migration));
                (app, name)
            }
            Plan::Revert { app, name } => {
                migration_plan.retain(|migration| applied_migrations.contains(migration));
                migration_plan.reverse();
                (app, name)
            }
        };

        // Error if only migration name present and app name not present
        if migration_name.is_some() && migration_app.is_none() {
            return Err(Error::AppNameRequired);
        }

        // Find position of last element and truncate till that position
        if let Some(app) = migration_app {
            let position = if let Some(name) = migration_name {
                migration_plan
                    .iter()
                    .rposition(|migration| migration.app() == app && migration.name() == name)
            } else {
                migration_plan
                    .iter()
                    .rposition(|migration| migration.app() == app)
            };
            if let Some(pos) = position {
                migration_plan.truncate(pos + 1);
            } else {
                migration_plan.clear();
            }
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
        for migration in self
            .generate_migration_plan(Plan::Apply {
                app: None,
                name: None,
            })
            .await?
        {
            self.apply_migration(migration).await?;
        }
        Ok(())
    }

    /// Apply given migration and add it to applied migration table
    #[allow(clippy::borrowed_box)]
    async fn apply_migration(&self, migration: &Box<dyn MigrationTrait<DB>>) -> Result<(), Error> {
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
        for migration in self
            .generate_migration_plan(Plan::Revert {
                app: None,
                name: None,
            })
            .await?
        {
            self.revert_migration(migration).await?;
        }
        Ok(())
    }

    /// Revert provided migration and remove migration from table
    #[allow(clippy::borrowed_box)]
    async fn revert_migration(&self, migration: &Box<dyn MigrationTrait<DB>>) -> Result<(), Error> {
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
/// postgres migrations
pub struct Migrator<DB>
where
    DB: sqlx::Database,
{
    migrations: HashSet<Box<dyn MigrationTrait<DB>>>,
    pool: Pool<DB>,
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
        }
    }
}

#[cfg(feature = "postgres")]
#[async_trait::async_trait]
impl MigratorTrait<Postgres> for Migrator<Postgres> {
    fn migrations(&self) -> &HashSet<Box<dyn MigrationTrait<Postgres>>> {
        &self.migrations
    }

    fn migrations_mut(&mut self) -> &mut HashSet<Box<dyn MigrationTrait<Postgres>>> {
        &mut self.migrations
    }

    fn pool(&self) -> &Pool<Postgres> {
        &self.pool
    }

    async fn ensure_migration_table_exists(&self) -> Result<(), Error> {
        sqlx::query(
            r#"
CREATE TABLE IF NOT EXISTS _sqlx_migrator_migrations (
    id SERIAL PRIMARY KEY,
    app TEXT NOT NULL,
    name TExT NOT NULL,
    applied_time TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (app, name)
)
            "#,
        )
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn add_migration_to_db_table(
        &self,
        migration: &Box<dyn MigrationTrait<Postgres>>,
        connection: &mut PgConnection,
    ) -> Result<(), Error> {
        sqlx::query(
            r#"
INSERT INTO _sqlx_migrator_migrations(app, name) VALUES ($1, $2)
            "#,
        )
        .bind(migration.app())
        .bind(migration.name())
        .execute(connection)
        .await?;
        Ok(())
    }

    async fn delete_migration_from_db_table(
        &self,
        migration: &Box<dyn MigrationTrait<Postgres>>,
        connection: &mut PgConnection,
    ) -> Result<(), Error> {
        sqlx::query(
            r#"
DELETE FROM _sqlx_migrator_migrations WHERE app = $1 AND name = $2
            "#,
        )
        .bind(migration.app())
        .bind(migration.name())
        .execute(connection)
        .await?;
        Ok(())
    }

    async fn fetch_applied_migration_from_db(&self) -> Result<Vec<AppliedMigrationSqlRow>, Error> {
        let rows =
            sqlx::query_as("SELECT id, app, name, applied_time FROM _sqlx_migrator_migrations")
                .fetch_all(self.pool())
                .await?;
        Ok(rows)
    }
}

#[cfg(feature = "sqlite")]
#[async_trait::async_trait]
impl MigratorTrait<Sqlite> for Migrator<Sqlite> {
    fn migrations(&self) -> &HashSet<Box<dyn MigrationTrait<Sqlite>>> {
        &self.migrations
    }

    fn migrations_mut(&mut self) -> &mut HashSet<Box<dyn MigrationTrait<Sqlite>>> {
        &mut self.migrations
    }

    fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }

    async fn ensure_migration_table_exists(&self) -> Result<(), Error> {
        sqlx::query(
            r#"
CREATE TABLE IF NOT EXISTS _sqlx_migrator_migrations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    app TEXT NOT NULL,
    name TEXT NOT NULL,
    applied_time TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (app, name)
)
            "#,
        )
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn add_migration_to_db_table(
        &self,
        migration: &Box<dyn MigrationTrait<Sqlite>>,
        connection: &mut SqliteConnection,
    ) -> Result<(), Error> {
        sqlx::query(
            r#"
INSERT INTO _sqlx_migrator_migrations(app, name) VALUES ($1, $2)
            "#,
        )
        .bind(migration.app())
        .bind(migration.name())
        .execute(connection)
        .await?;
        Ok(())
    }

    async fn delete_migration_from_db_table(
        &self,
        migration: &Box<dyn MigrationTrait<Sqlite>>,
        connection: &mut SqliteConnection,
    ) -> Result<(), Error> {
        sqlx::query(
            r#"
DELETE FROM _sqlx_migrator_migrations WHERE app = $1 AND name = $2
            "#,
        )
        .bind(migration.app())
        .bind(migration.name())
        .execute(connection)
        .await?;
        Ok(())
    }

    async fn fetch_applied_migration_from_db(&self) -> Result<Vec<AppliedMigrationSqlRow>, Error> {
        let rows =
            sqlx::query_as("SELECT id, app, name, applied_time FROM _sqlx_migrator_migrations")
                .fetch_all(self.pool())
                .await?;
        Ok(rows)
    }
}
