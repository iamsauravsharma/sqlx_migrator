//! migrator module

use std::collections::HashSet;

use sqlx::{Pool, Transaction};

use crate::error::Error;
use crate::migration::Migration;

type MigrationVecResult<'a, DB> = Result<Vec<&'a Box<dyn Migration<Database = DB>>>, Error>;

/// Type of plan used to generate migrations
#[derive(Debug)]
pub enum PlanType {
    /// Full plan. Plan containing all migrations according to order
    Full,
    /// Apply plan. Plan containing migrations which can be applied
    Apply,
    /// Revert plan. Plan containing migrations which can be reverted
    Revert,
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
    async fn add_migration_to_db_table<'t>(
        &self,
        migration_full_name: &str,
        transaction: &mut Transaction<'t, Self::Database>,
    ) -> Result<(), Error>;

    /// Delete migration from table
    async fn delete_migration_from_db_table<'t>(
        &self,
        migration_full_name: &str,
        transaction: &mut Transaction<'t, Self::Database>,
    ) -> Result<(), Error>;

    /// List all applied migrations from database in string format (full name of
    /// migration)
    async fn fetch_applied_migration_from_db(&self) -> Result<Vec<String>, Error>;

    /// Add vector of migrations to Migrator object
    fn add_migrations(&mut self, migrations: Vec<Box<dyn Migration<Database = Self::Database>>>) {
        for migration in migrations {
            self.add_migration(migration);
        }
    }

    /// Add single migration to migrator object
    fn add_migration(&mut self, migration: Box<dyn Migration<Database = Self::Database>>) {
        let parents = migration.parents();
        self.migrations_mut().insert(migration);
        for parent in parents {
            self.add_migration(parent);
        }
    }

    /// List all applied migrations. Returns a vector of migration
    async fn list_applied_migrations(&self) -> MigrationVecResult<Self::Database> {
        if cfg!(feature = "tracing") {
            tracing::info!("Fetching applied migrations");
        }
        let applied_migration_list = self.fetch_applied_migration_from_db().await?;

        let mut applied_migrations = Vec::new();
        for migration in self.migrations() {
            if applied_migration_list.contains(&migration.full_name()) {
                applied_migrations.push(migration);
            }
        }
        Ok(applied_migrations)
    }

    /// Generate migration plan for according to plan type. Returns a vector of
    /// migration
    async fn generate_migration_plan(
        &self,
        plan_type: PlanType,
    ) -> MigrationVecResult<Self::Database> {
        let applied_migrations = self.list_applied_migrations().await?;
        if cfg!(feature = "tracing") {
            tracing::info!("Generating {:?} migration plan", plan_type);
        }
        let mut migration_plan = Vec::new();
        while migration_plan.len() != self.migrations().len() {
            let old_migration_plan_length = migration_plan.len();
            for migration in self.migrations() {
                if migration
                    .parents()
                    .iter()
                    .all(|migration| migration_plan.contains(&migration))
                    && !migration_plan.contains(&migration)
                {
                    migration_plan.push(migration);
                }
            }
            if old_migration_plan_length == migration_plan.len() {
                return Err(Error::FailedToCreateMigrationPlan);
            }
        }
        match plan_type {
            PlanType::Full => (),
            PlanType::Apply => migration_plan.retain(|plan| !applied_migrations.contains(plan)),
            PlanType::Revert => {
                migration_plan.retain(|plan| applied_migrations.contains(plan));
                migration_plan.reverse();
            }
        };
        Ok(migration_plan)
    }

    /// Apply missing migration plan
    ///
    /// # Errors
    /// If failed to apply migration
    async fn apply_all(&self) -> Result<(), Error> {
        if cfg!(feature = "tracing") {
            tracing::info!("Applying all migration");
        }
        self.ensure_migration_table_exists().await?;
        for migration in self.generate_migration_plan(PlanType::Apply).await? {
            self.apply_migration(migration).await?;
        }
        Ok(())
    }

    /// Apply certain migration to database and add it to applied migration
    #[allow(clippy::borrowed_box)]
    async fn apply_migration(
        &self,
        migration: &Box<dyn Migration<Database = Self::Database>>,
    ) -> Result<(), Error> {
        if cfg!(feature = "tracing") {
            tracing::info!("Applying migration {}", migration.full_name());
        }
        let mut transaction = self.pool().begin().await?;
        for operation in migration.operations() {
            operation.up(&mut transaction).await?;
        }

        self.add_migration_to_db_table(&migration.full_name(), &mut transaction)
            .await?;
        transaction.commit().await?;
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
        self.ensure_migration_table_exists().await?;
        for migration in self.generate_migration_plan(PlanType::Revert).await? {
            self.revert_migration(migration).await?;
        }
        Ok(())
    }

    /// Revert migration
    #[allow(clippy::borrowed_box)]
    async fn revert_migration(
        &self,
        migration: &Box<dyn Migration<Database = Self::Database>>,
    ) -> Result<(), Error> {
        if cfg!(feature = "tracing") {
            tracing::info!("Reverting migration {}", migration.full_name());
        }
        let mut transaction = self.pool().begin().await?;
        let mut operations = migration.operations();
        operations.reverse();
        for operation in operations {
            operation.down(&mut transaction).await?;
        }
        self.delete_migration_from_db_table(&migration.full_name(), &mut transaction)
            .await?;
        transaction.commit().await?;
        Ok(())
    }
}
