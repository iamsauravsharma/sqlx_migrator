use std::collections::HashMap;

use petgraph::graph::NodeIndex;
use petgraph::Graph;
use sqlx::{Pool, Postgres, Row, Transaction};

use crate::error::Error;
use crate::migration::Migration;
use crate::migrator::MigratorTrait;

/// Migrator struct which store migrations graph and information related to
/// postgres migration
pub struct PostgresMigrator<'a> {
    graph: Graph<Box<dyn Migration<Database = Postgres>>, ()>,
    migrations_map: HashMap<String, NodeIndex>,
    pool: &'a Pool<Postgres>,
}

#[async_trait::async_trait]
impl<'a> MigratorTrait for PostgresMigrator<'a> {
    type Database = Postgres;

    fn graph(&self) -> &Graph<Box<dyn Migration<Database = Self::Database>>, ()> {
        &self.graph
    }

    fn graph_mut(&mut self) -> &mut Graph<Box<dyn Migration<Database = Self::Database>>, ()> {
        &mut self.graph
    }

    fn migrations_map(&self) -> &HashMap<String, NodeIndex> {
        &self.migrations_map
    }

    fn pool(&self) -> &Pool<Self::Database> {
        self.pool
    }

    async fn ensure_migration_table(&self) -> Result<(), Error> {
        if cfg!(feature = "tracing") {
            tracing::info!("Creating migration table if not exists");
        }
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
INSERT INTO _migrator_migrations(migration_name) VALUES ($1)
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
DELETE FROM  _migrator_migrations where migration_name = $1
            "#,
        )
        .bind(migration_name)
        .execute(transaction)
        .await?;
        Ok(())
    }

    async fn list_applied_migration(&self) -> Result<Vec<String>, Error> {
        let mut applied_migrations = Vec::new();
        let rows = sqlx::query("SELECT migration_name FROM _migrator_migrations")
            .fetch_all(self.pool)
            .await?;

        for row in rows {
            let migration_name: String = row.try_get("migration_name")?;
            applied_migrations.push(migration_name);
        }
        Ok(applied_migrations)
    }
}
