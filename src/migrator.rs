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

use sqlx::Pool;

use crate::error::Error;
use crate::migration::{AppliedMigrationSqlRow, Migration};

type MigrationVecResult<'a, DB> = Result<Vec<&'a Box<dyn Migration<Database = DB>>>, Error>;

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

#[async_trait::async_trait]
/// Migrator trait
pub trait Migrator: Send + Sync {
    /// Database type
    type Database: sqlx::Database;

    /// Return migrations
    fn migrations(&self) -> &HashSet<Box<dyn Migration<Database = Self::Database>>>;

    /// Return mutable reference of migrations
    fn migrations_mut(&mut self) -> &mut HashSet<Box<dyn Migration<Database = Self::Database>>>;

    /// Return pool of database
    fn pool(&self) -> &Pool<Self::Database>;

    /// Ensure migration table is created before running migrations. If not
    /// create one
    async fn ensure_migration_table_exists(&self) -> Result<(), Error>;

    /// Add migration to migration table
    #[allow(clippy::borrowed_box)]
    async fn add_migration_to_db_table(
        &self,
        migration: &Box<dyn Migration<Database = Self::Database>>,
        connection: &mut <Self::Database as sqlx::Database>::Connection,
    ) -> Result<(), Error>;

    /// Delete migration from table
    #[allow(clippy::borrowed_box)]
    async fn delete_migration_from_db_table(
        &self,
        migration: &Box<dyn Migration<Database = Self::Database>>,
        connection: &mut <Self::Database as sqlx::Database>::Connection,
    ) -> Result<(), Error>;

    /// List all applied migrations from database in string format (full name of
    /// migration)
    async fn fetch_applied_migration_from_db(&self) -> Result<Vec<AppliedMigrationSqlRow>, Error>;

    /// Add vector of migrations to Migrator object
    fn add_migrations(&mut self, migrations: Vec<Box<dyn Migration<Database = Self::Database>>>) {
        for migration in migrations {
            self.add_migration(migration);
        }
    }

    /// Add single migration to migrator object
    fn add_migration(&mut self, migration: Box<dyn Migration<Database = Self::Database>>) {
        for parent in migration.parents() {
            self.add_migration(parent);
        }
        self.migrations_mut().insert(migration);
    }

    /// List all applied migrations. Returns a vector of migration
    async fn list_applied_migrations(&self) -> MigrationVecResult<Self::Database> {
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
    async fn generate_migration_plan(&self, plan: Plan) -> MigrationVecResult<Self::Database> {
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
    async fn apply_migration(
        &self,
        migration: &Box<dyn Migration<Database = Self::Database>>,
    ) -> Result<(), Error> {
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
    async fn revert_migration(
        &self,
        migration: &Box<dyn Migration<Database = Self::Database>>,
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
