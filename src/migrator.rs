use std::collections::HashMap;

use anyhow::{Context, Error};
use petgraph::graph::NodeIndex;
use petgraph::Graph;
use sqlx::{PgPool, Postgres, Row, Transaction};

use crate::Migration;

/// Migrator struct which store migrations graph and information related to
/// migration
pub struct Migrator<'a> {
    graph: Graph<Box<dyn Migration>, ()>,
    migrations_map: HashMap<String, NodeIndex>,
    pool: &'a PgPool,
}

impl<'a> Migrator<'a> {
    /// Create new migrator using postgres pool
    #[must_use]
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            graph: Graph::new(),
            migrations_map: HashMap::new(),
            pool,
        }
    }

    /// Add vector of migrations to Migrator
    pub fn add_migrations(&mut self, migrations: Vec<Box<dyn Migration>>) -> Vec<NodeIndex> {
        let mut node_index_vec = Vec::new();
        for migration in migrations {
            node_index_vec.push(self.add_migration(migration));
        }
        node_index_vec
    }

    /// Add single migration to migrator
    pub fn add_migration(&mut self, migration: Box<dyn Migration>) -> NodeIndex {
        let (node_index, parents) = match self.migrations_map.get(&migration.full_name()) {
            None => {
                let full_name = migration.full_name();
                let parents = migration.parents();
                let node_index = self.graph.add_node(migration);
                self.migrations_map.insert(full_name, node_index);

                (node_index, parents)
            }
            Some(&val) => (val, migration.parents()),
        };
        for parent in parents {
            let parent_index = match self.migrations_map.get(&parent.full_name()) {
                None => self.add_migration(parent),
                Some(&val) => val,
            };
            self.graph.add_edge(parent_index, node_index, ());
        }
        node_index
    }

    async fn apply_all_plan(&self) -> Result<Vec<&Box<dyn Migration>>, Error> {
        let applied_migrations = self.list_applied_migration().await?;
        tracing::info!("Creating apply migration plan");
        let mut added_node = Vec::new();
        let mut plan_vec = Vec::<&Box<dyn Migration>>::new();
        while added_node.len() < self.graph.node_indices().len() {
            for node_index in self.graph.node_indices() {
                let mut dfs = petgraph::visit::Dfs::new(&self.graph, node_index);
                while let Some(nx) = dfs.next(&self.graph) {
                    if !added_node.contains(&nx) {
                        let migration = &self.graph[nx];
                        let parent_added = self
                            .graph
                            .neighbors_directed(nx, petgraph::Direction::Incoming)
                            .all(|x| added_node.contains(&x));
                        if parent_added {
                            added_node.push(nx);
                            if !applied_migrations.contains(&migration.full_name()) {
                                plan_vec.push(migration);
                            }
                        }
                    }
                }
            }
        }
        Ok(plan_vec)
    }

    /// Apply missing migration
    /// # Errors
    /// If any migration or operation fails
    pub async fn apply(&self) -> Result<(), Error> {
        self.ensure_migration_table().await?;
        for migration in self.apply_all_plan().await? {
            self.apply_migration(migration).await?;
        }
        Ok(())
    }

    #[allow(clippy::borrowed_box)]
    async fn apply_migration(&self, migration: &Box<dyn Migration>) -> Result<(), Error> {
        tracing::info!("Applying migration {}", migration.full_name());
        let mut transaction = self.pool.begin().await.context(format!(
            "Failed to begin transaction for {} up migration",
            migration.full_name()
        ))?;
        for operation in migration.operations() {
            operation.up(&mut transaction).await?;
        }

        self.add_migration_to_table(migration.full_name(), &mut transaction)
            .await?;
        transaction.commit().await.context(format!(
            "Failed to commit transaction for {} up migration",
            migration.full_name()
        ))?;
        Ok(())
    }

    async fn revert_all_plan(&self) -> Result<Vec<&Box<dyn Migration>>, Error> {
        let applied_migrations = self.list_applied_migration().await?;
        tracing::info!("Creating revert migration plan");
        let mut added_node = Vec::new();
        let mut plan_vec = Vec::<&Box<dyn Migration>>::new();
        while added_node.len() < self.graph.node_indices().len() {
            for node_index in self.graph.node_indices() {
                let mut dfs = petgraph::visit::Dfs::new(&self.graph, node_index);
                while let Some(nx) = dfs.next(&self.graph) {
                    if !added_node.contains(&nx) {
                        let migration = &self.graph[nx];
                        let parent_added = self
                            .graph
                            .neighbors_directed(nx, petgraph::Direction::Incoming)
                            .all(|x| added_node.contains(&x));
                        if parent_added {
                            added_node.push(nx);
                            if applied_migrations.contains(&migration.full_name()) {
                                plan_vec.push(migration);
                            }
                        }
                    }
                }
            }
        }
        plan_vec.reverse();
        Ok(plan_vec)
    }

    /// Revert all applied migration
    /// # Errors
    /// If any migration or operation fails
    pub async fn revert(&self) -> Result<(), Error> {
        self.ensure_migration_table().await?;
        for migration in self.revert_all_plan().await? {
            self.revert_migration(migration).await?;
        }
        Ok(())
    }

    #[allow(clippy::borrowed_box)]
    async fn revert_migration(&self, migration: &Box<dyn Migration>) -> Result<(), Error> {
        tracing::info!("Reverting migration {}", migration.full_name());
        let mut transaction = self.pool.begin().await.context(format!(
            "Failed to begin transaction for {} down migration",
            migration.full_name()
        ))?;
        let mut operations = migration.operations();
        operations.reverse();
        for operation in operations {
            operation.down(&mut transaction).await?;
        }
        self.delete_migration_from_table(migration.full_name(), &mut transaction)
            .await?;
        transaction.commit().await.context(format!(
            "Failed to commit transaction for {} down migration",
            migration.full_name()
        ))?;
        Ok(())
    }

    async fn ensure_migration_table(&self) -> Result<(), Error> {
        tracing::info!("Creating migration table if not exists");
        sqlx::query(
            r#"
CREATE TABLE IF NOT EXISTS _migrator_migrations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    migration_name TEXT UNIQUE NOT NULL,
    applied_time TIMESTAMPTZ NOT NULL DEFAULT now()
)
            "#,
        )
        .execute(self.pool)
        .await
        .context("Failed to create migrations table")?;
        Ok(())
    }

    async fn add_migration_to_table<'t>(
        &self,
        migration_full_name: String,
        transaction: &mut Transaction<'t, Postgres>,
    ) -> Result<(), Error> {
        sqlx::query(
            r#"
INSERT INTO _migrator_migrations(migration_name) VALUES ($1)
            "#,
        )
        .bind(migration_full_name)
        .execute(transaction)
        .await
        .context("Failed to add migration to table")?;
        Ok(())
    }

    async fn delete_migration_from_table<'t>(
        &self,
        migration_full_name: String,
        transaction: &mut Transaction<'t, Postgres>,
    ) -> Result<(), Error> {
        sqlx::query(
            r#"
DELETE FROM  _migrator_migrations where migration_name = $1
            "#,
        )
        .bind(migration_full_name)
        .execute(transaction)
        .await
        .context("Failed to delete migration from table")?;
        Ok(())
    }

    async fn list_applied_migration(&self) -> Result<Vec<String>, Error> {
        let mut applied_migrations = Vec::new();
        let rows = sqlx::query("SELECT migration_name FROM _migrator_migrations")
            .fetch_all(self.pool)
            .await
            .context("Failed to get migrations name")?;

        for row in rows {
            let migration_name: String = row
                .try_get("migration_name")
                .context("Failed to get migration_name value")?;
            applied_migrations.push(migration_name);
        }
        Ok(applied_migrations)
    }
}