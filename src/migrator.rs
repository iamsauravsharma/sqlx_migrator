//! migrator module

use std::collections::HashSet;

use sqlx::{Pool, Transaction};

use crate::error::Error;
use crate::migration::Migration;

type MigrationVecResult<'a, DB> = Result<Vec<&'a Box<dyn Migration<Database = DB>>>, Error>;

#[async_trait::async_trait]
/// Migrator trait
pub trait Migrator: Send + Sync {
    /// Database type
    type Database: sqlx::Database;

    /// Return migrations
    fn migrations(&self) -> &HashSet<Box<dyn Migration<Database = Self::Database>>>;

    /// Return mutable reference of migrations
    fn migrations_mut(&mut self) -> &mut HashSet<Box<dyn Migration<Database = Self::Database>>>;

    /// Return pool
    fn pool(&self) -> &Pool<Self::Database>;

    /// Ensure migration table is created before running migrations. If not
    /// create one
    async fn ensure_migration_table_exists(&self) -> Result<(), Error>;

    /// Add migration to migration table
    async fn add_migration_to_db_table<'t>(
        &self,
        migration_name: String,
        transaction: &mut Transaction<'t, Self::Database>,
    ) -> Result<(), Error>;

    /// Delete migration from table
    async fn delete_migration_from_db_table<'t>(
        &self,
        migration_name: String,
        transaction: &mut Transaction<'t, Self::Database>,
    ) -> Result<(), Error>;

    /// List all applied migrations
    async fn fetch_applied_migration_from_db(&self) -> Result<Vec<String>, Error>;

    /// Add vector of migrations to Migrator
    fn add_migrations(&mut self, migrations: Vec<Box<dyn Migration<Database = Self::Database>>>) {
        for migration in migrations {
            self.add_migration(migration);
        }
    }

    /// Add single migration to migrator
    fn add_migration(&mut self, migration: Box<dyn Migration<Database = Self::Database>>) {
        let parents = migration.parents();
        self.migrations_mut().insert(migration);
        for parent in parents {
            self.add_migration(parent);
        }
    }

    /// List all applied migrations
    async fn list_applied_migrations(&self) -> MigrationVecResult<Self::Database> {
        let applied_migration_list = self.fetch_applied_migration_from_db().await?;
        let mut applied_migrations = Vec::new();
        for migration in self.migrations() {
            if applied_migration_list.contains(&migration.name()) {
                applied_migrations.push(migration);
            }
        }
        Ok(applied_migrations)
    }

    /// Generate full migration plan
    #[allow(clippy::borrowed_box)]
    fn generate_full_migration_plan(&self) -> MigrationVecResult<Self::Database> {
        let mut migration_plan = Vec::new();
        while migration_plan.len() != self.migrations().len() {
            let old_migration_plan_length = migration_plan.len();
            for migration in self.migrations() {
                if migration
                    .parents()
                    .iter()
                    .all(|migration| migration_plan.contains(&migration))
                {
                    migration_plan.push(migration);
                }
            }
            if old_migration_plan_length == migration_plan.len() {
                return Err(Error::FailedToCreateMigrationPlan);
            }
        }
        Ok(migration_plan)
    }

    /// Generate apply all migration plan
    async fn apply_all_plan(&self) -> MigrationVecResult<Self::Database> {
        let applied_migrations = self.list_applied_migrations().await?;
        if cfg!(feature = "tracing") {
            tracing::info!("Creating apply migration plan");
        }
        let full_plan = self.generate_full_migration_plan()?;
        let mut apply_all_plan = Vec::new();
        for plan in full_plan {
            if !applied_migrations.contains(&plan) {
                apply_all_plan.push(plan);
            }
        }
        Ok(apply_all_plan)
    }

    /// Apply missing migration
    ///
    /// # Errors
    /// If any migration or operation fails
    async fn apply_all(&self) -> Result<(), Error> {
        if cfg!(feature = "tracing") {
            tracing::info!("Creating migration table if not exists");
        }
        self.ensure_migration_table_exists().await?;
        for migration in self.apply_all_plan().await? {
            self.apply_migration(migration).await?;
        }
        Ok(())
    }

    /// Apply certain migration to database
    #[allow(clippy::borrowed_box)]
    async fn apply_migration(
        &self,
        migration: &Box<dyn Migration<Database = Self::Database>>,
    ) -> Result<(), Error> {
        if cfg!(feature = "tracing") {
            tracing::info!("Applying migration {}", migration.name());
        }
        let mut transaction = self.pool().begin().await?;
        for operation in migration.operations() {
            operation.up(&mut transaction).await?;
        }

        self.add_migration_to_db_table(migration.name(), &mut transaction)
            .await?;
        transaction.commit().await?;
        Ok(())
    }

    /// Create revert all plan
    async fn revert_all_plan(&self) -> MigrationVecResult<Self::Database> {
        let applied_migrations = self.list_applied_migrations().await?;
        if cfg!(feature = "tracing") {
            tracing::info!("Creating revert migration plan");
        }
        let full_plan = self.generate_full_migration_plan()?;
        let mut revert_all_plan = Vec::new();
        for plan in full_plan {
            if applied_migrations.contains(&plan) {
                revert_all_plan.push(plan);
            }
        }
        revert_all_plan.reverse();
        Ok(revert_all_plan)
    }

    /// Revert all applied migration
    ///
    /// # Errors
    /// If any migration or operation fails
    async fn revert_all(&self) -> Result<(), Error> {
        if cfg!(feature = "tracing") {
            tracing::info!("Creating migration table if not exists");
        }
        self.ensure_migration_table_exists().await?;
        for migration in self.revert_all_plan().await? {
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
            tracing::info!("Reverting migration {}", migration.name());
        }
        let mut transaction = self.pool().begin().await?;
        let mut operations = migration.operations();
        operations.reverse();
        for operation in operations {
            operation.down(&mut transaction).await?;
        }
        self.delete_migration_from_db_table(migration.name(), &mut transaction)
            .await?;
        transaction.commit().await?;
        Ok(())
    }
}
