//! migrator module

use std::collections::{HashMap, HashSet};

use sqlx::Pool;

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
    async fn add_migration_to_db_table(
        &self,
        migration_full_name: &str,
        connection: &mut <Self::Database as sqlx::Database>::Connection,
    ) -> Result<(), Error>;

    /// Delete migration from table
    async fn delete_migration_from_db_table(
        &self,
        migration_full_name: &str,
        connection: &mut <Self::Database as sqlx::Database>::Connection,
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
        let applied_migration_list = self.fetch_applied_migration_from_db().await?;

        // convert applied migration string name to vector of migration implemented
        // objects
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

        // Hashmap which contains key as migration name and value as list of migration
        // which needs to applied earlier than key according to before method of value
        // migration
        let mut run_before_parents_hashmap = HashMap::new();

        for migration in self.migrations() {
            for run_before_migration in migration.run_before() {
                run_before_parents_hashmap
                    .entry(run_before_migration)
                    .or_insert(Vec::new())
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

                let mut all_run_before_parents_added = true;
                if let Some(before_migrations) = run_before_parents_hashmap.get(migration) {
                    all_run_before_parents_added = before_migrations
                        .iter()
                        .all(|migration| migration_plan.contains(migration));
                }

                if all_parents_applied
                    && all_run_before_parents_added
                    && !migration_plan.contains(&migration)
                {
                    migration_plan.push(migration);
                }
            }

            // If old migration plan length is equal to current length than no new migration
            // was added. Next loop also will not add migration so return error
            if old_migration_plan_length == migration_plan.len() {
                return Err(Error::FailedToCreateMigrationPlan);
            }
        }

        // Handle replaces condition
        let mut removed_migration_name = Vec::new();

        // List which migration needs to be removed from plan
        for migration in &migration_plan {
            if !migration.replaces().is_empty() {
                // Check if any replaces migration are applied for not
                let replaces_applied = migration
                    .replaces()
                    .iter()
                    .any(|migration| applied_migrations.contains(&migration));

                if replaces_applied {
                    removed_migration_name.push(migration.full_name());
                } else {
                    for replaced_migration in migration.replaces() {
                        removed_migration_name.push(replaced_migration.full_name());
                    }
                }
            }
        }

        // Retain only migration which are not in removed migration name
        migration_plan.retain(|migration| !removed_migration_name.contains(&migration.full_name()));

        // Return migration according to plan type
        match plan_type {
            PlanType::Full => (),
            PlanType::Apply => {
                migration_plan.retain(|migration| !applied_migrations.contains(migration));
            }
            PlanType::Revert => {
                migration_plan.retain(|migration| applied_migrations.contains(migration));
                migration_plan.reverse();
            }
        };
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
        self.ensure_migration_table_exists().await?;
        for migration in self.generate_migration_plan(PlanType::Apply).await? {
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
            tracing::info!("Applying migration {}", migration.full_name());
        }
        if migration.is_atomic() {
            let mut transaction = self.pool().begin().await?;
            for operation in migration.operations() {
                operation.up(&mut transaction).await?;
            }
            self.add_migration_to_db_table(&migration.full_name(), &mut transaction)
                .await?;
            transaction.commit().await?;
        } else {
            let mut connection = self.pool().acquire().await?;
            for operation in migration.operations() {
                operation.up(&mut connection).await?;
            }
            self.add_migration_to_db_table(&migration.full_name(), &mut connection)
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
        self.ensure_migration_table_exists().await?;
        for migration in self.generate_migration_plan(PlanType::Revert).await? {
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
            tracing::info!("Reverting migration {}", migration.full_name());
        }

        // Reverse operation since last applied operation need to be reverted first
        let mut operations = migration.operations();
        operations.reverse();

        if migration.is_atomic() {
            let mut transaction = self.pool().begin().await?;
            for operation in operations {
                operation.down(&mut transaction).await?;
            }
            self.delete_migration_from_db_table(&migration.full_name(), &mut transaction)
                .await?;
            transaction.commit().await?;
        } else {
            let mut connection = self.pool().acquire().await?;
            for operation in operations {
                operation.down(&mut connection).await?;
            }
            self.delete_migration_from_db_table(&migration.full_name(), &mut connection)
                .await?;
        }
        Ok(())
    }
}
