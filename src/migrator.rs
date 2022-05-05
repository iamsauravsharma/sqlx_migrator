//! migrator module

use std::collections::HashMap;

use petgraph::graph::NodeIndex;
use petgraph::Graph;
use sqlx::{Pool, Transaction};

use crate::error::Error;
use crate::migration::Migration;

#[async_trait::async_trait]
/// Migrator trait
pub trait Migrator: Send {
    /// Database type
    type Database: sqlx::Database;

    /// Return graph
    fn graph(&self) -> &Graph<Box<dyn Migration<Database = Self::Database>>, ()>;

    /// Return mutable reference of graph
    fn graph_mut(&mut self) -> &mut Graph<Box<dyn Migration<Database = Self::Database>>, ()>;

    /// Return migrations map
    fn migrations_map(&self) -> &HashMap<String, NodeIndex>;

    /// Return pool
    fn pool(&self) -> &Pool<Self::Database>;

    /// Ensure migration table is created before running migrations. If not
    /// created create one
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
    fn add_migrations(
        &mut self,
        migrations: Vec<Box<dyn Migration<Database = Self::Database>>>,
    ) -> Vec<NodeIndex> {
        let mut node_index_vec = Vec::new();
        for migration in migrations {
            node_index_vec.push(self.add_migration(migration));
        }
        node_index_vec
    }

    /// Add single migration to migrator
    fn add_migration(
        &mut self,
        migration: Box<dyn Migration<Database = Self::Database>>,
    ) -> NodeIndex {
        let parents = migration.parents();
        let mut migrations_map = self.migrations_map().clone();
        let &mut node_index = migrations_map
            .entry(migration.name())
            .or_insert_with(|| self.graph_mut().add_node(migration));
        for parent in parents {
            let parent_index = self.add_migration(parent);
            self.graph_mut().add_edge(parent_index, node_index, ());
        }
        node_index
    }

    /// Generate full migration plan
    #[allow(clippy::borrowed_box)]
    fn generate_full_migration_plan(&self) -> Vec<&Box<dyn Migration<Database = Self::Database>>> {
        let mut added_node = Vec::new();
        let mut plan_vec = Vec::<&Box<dyn Migration<Database = Self::Database>>>::new();
        while added_node.len() < self.graph().node_indices().len() {
            for node_index in self.graph().node_indices() {
                let mut dfs = petgraph::visit::Dfs::new(&self.graph(), node_index);
                while let Some(nx) = dfs.next(&self.graph()) {
                    if !added_node.contains(&nx) {
                        let migration = &self.graph()[nx];
                        let parent_added = self
                            .graph()
                            .neighbors_directed(nx, petgraph::Direction::Incoming)
                            .all(|x| added_node.contains(&x));
                        if parent_added {
                            added_node.push(nx);
                            plan_vec.push(migration);
                        }
                    }
                }
            }
        }
        plan_vec
    }

    /// Generate apply all migration plan
    async fn apply_all_plan(
        &self,
    ) -> Result<Vec<&Box<dyn Migration<Database = Self::Database>>>, Error> {
        let applied_migrations = self.fetch_applied_migration_from_db().await?;
        if cfg!(feature = "tracing") {
            tracing::info!("Creating apply migration plan");
        }
        let full_plan = self.generate_full_migration_plan();
        let mut apply_all_plan = Vec::new();
        for plan in full_plan {
            if !applied_migrations.contains(&plan.name()) {
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
    async fn revert_all_plan(
        &self,
    ) -> Result<Vec<&Box<dyn Migration<Database = Self::Database>>>, Error> {
        let applied_migrations = self.fetch_applied_migration_from_db().await?;
        if cfg!(feature = "tracing") {
            tracing::info!("Creating revert migration plan");
        }
        let full_plan = self.generate_full_migration_plan();
        let mut revert_all_plan = Vec::new();
        for plan in full_plan {
            if applied_migrations.contains(&plan.name()) {
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
