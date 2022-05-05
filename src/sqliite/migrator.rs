//! Sqlite migrator module

use std::collections::HashMap;

use petgraph::graph::NodeIndex;
use petgraph::Graph;
use sqlx::{Pool, Row, Sqlite, Transaction};

use crate::error::Error;
use crate::migration::Migration;
use crate::migrator::{MigratorTrait, NewMigrator};

/// Migrator struct which store migrations graph and information related to
/// Sqlite migration
pub struct Migrator {
    graph: Graph<Box<dyn Migration<Database = Sqlite>>, ()>,
    migrations_map: HashMap<String, NodeIndex>,
    pool: Pool<Sqlite>,
}

impl Migrator {
    /// Create new migrator from pool
    pub fn new_from_pool(pool: &Pool<Sqlite>) -> Self {
        Self {
            graph: Graph::new(),
            migrations_map: HashMap::new(),
            pool: pool.clone(),
        }
    }

    /// Create new migrator from uri
    pub async fn new_from_uri(uri: &str) -> Result<Self, Error> {
        let pool = Pool::connect(uri).await?;
        Ok(Self::new_from_pool(&pool))
    }

    /// Create new migrator from env
    pub async fn new_from_env() -> Result<Self, Error> {
        let database_uri = std::env::var("DATABASE_URL")
            .map_err(|_| Error::FailedToGetEnv(String::from("DATABASE_URL")))?;
        Self::new_from_uri(database_uri.as_str()).await
    }
}

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
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

    fn pool(&self) -> &Pool<Self::Database> {
        &self.pool
    }

    async fn ensure_migration_table(&self) -> Result<(), Error> {
        sqlx::query(
            r#"
CREATE TABLE IF NOT EXISTS _sqlx_migrator_migrations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    migration_name TEXT UNIQUE NOT NULL,
    applied_time TIMESTAMP DEFAULT CURRENT_TIMESTAMP
)
            "#,
        )
        .execute(self.pool())
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

    async fn list_applied_migration(&self) -> Result<Vec<String>, Error> {
        let mut applied_migrations = Vec::new();
        let rows = sqlx::query("SELECT migration_name FROM _sqlx_migrator_migrations")
            .fetch_all(self.pool())
            .await?;

        for row in rows {
            let migration_name: String = row.try_get("migration_name")?;
            applied_migrations.push(migration_name);
        }
        Ok(applied_migrations)
    }
}
