//! Sqlite migrator module

use std::collections::HashMap;

use petgraph::graph::NodeIndex;
use petgraph::Graph;
use sqlx::{Pool, Row, Sqlite, Transaction};

use crate::error::Error;
use crate::migration::Migration;
use crate::migrator::MigratorTrait;

/// Migrator struct which store migrations graph and information related to
/// Sqlite migration
#[derive(Default)]
pub struct SqliteMigrator {
    graph: Graph<Box<dyn Migration<Database = Sqlite>>, ()>,
    migrations_map: HashMap<String, NodeIndex>,
}

#[async_trait::async_trait]
impl MigratorTrait for SqliteMigrator {
    type Database = Sqlite;

    fn graph(&self) -> &Graph<Box<dyn Migration<Database = Self::Database>>, ()> {
        &self.graph
    }

    fn graph_mut(&mut self) -> &mut Graph<Box<dyn Migration<Database = Self::Database>>, ()> {
        &mut self.graph
    }

    fn migrations_map(&self) -> &HashMap<String, NodeIndex> {
        &self.migrations_map
    }

    async fn ensure_migration_table(&self, pool: &Pool<Self::Database>) -> Result<(), Error> {
        sqlx::query(
            r#"
CREATE TABLE IF NOT EXISTS _sqlx_migrator_migrations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    migration_name TEXT UNIQUE NOT NULL,
    applied_time TIMESTAMP DEFAULT CURRENT_TIMESTAMP
)
            "#,
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    async fn add_migration_to_table<'t>(
        &self,
        migration_name: String,
        transaction: &mut Transaction<'t, Self::Database>,
    ) -> Result<(), Error> {
        sqlx::query(
            r#"
INSERT INTO _sqlx_migrator_migrations(migration_name) VALUES ($1)
            "#,
        )
        .bind(migration_name)
        .execute(transaction)
        .await?;
        Ok(())
    }

    async fn delete_migration_from_table<'t>(
        &self,
        migration_name: String,
        transaction: &mut Transaction<'t, Self::Database>,
    ) -> Result<(), Error> {
        sqlx::query(
            r#"
DELETE FROM _sqlx_migrator_migrations WHERE migration_name = $1
            "#,
        )
        .bind(migration_name)
        .execute(transaction)
        .await?;
        Ok(())
    }

    async fn list_applied_migration(
        &self,
        pool: &Pool<Self::Database>,
    ) -> Result<Vec<String>, Error> {
        let mut applied_migrations = Vec::new();
        let rows = sqlx::query("SELECT migration_name FROM _sqlx_migrator_migrations")
            .fetch_all(pool)
            .await?;

        for row in rows {
            let migration_name: String = row.try_get("migration_name")?;
            applied_migrations.push(migration_name);
        }
        Ok(applied_migrations)
    }
}
