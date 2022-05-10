//! Sqlite migrator module
use std::collections::HashSet;

use sqlx::{Pool, Row, Sqlite, Transaction};

use crate::error::Error;
use crate::migration::Migration;
use crate::migrator::Migrator as MigratorTrait;

/// Migrator struct which store migrations graph and information related to
/// Sqlite migration
pub struct Migrator {
    migrations: HashSet<Box<dyn Migration<Database = Sqlite>>>,
    pool: Pool<Sqlite>,
}

impl Migrator {
    /// Create new migrator from pool
    #[must_use]
    pub fn new_from_pool(pool: &Pool<Sqlite>) -> Self {
        Self {
            migrations: HashSet::new(),
            pool: pool.clone(),
        }
    }

    /// Create new migrator from uri
    ///
    /// # Errors
    /// - If pool creation fails
    pub async fn new_from_uri(uri: &str) -> Result<Self, Error> {
        let pool = Pool::connect(uri).await?;
        Ok(Self::new_from_pool(&pool))
    }

    /// Create new migrator from env
    ///
    /// # Errors
    /// - If env var is not set or is not valid
    /// - If pool creation fails
    pub async fn new_from_env() -> Result<Self, Error> {
        let database_uri = std::env::var("DATABASE_URL")
            .map_err(|_| Error::FailedToGetEnv(String::from("DATABASE_URL")))?;
        Self::new_from_uri(database_uri.as_str()).await
    }
}

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    type Database = Sqlite;

    fn migrations(&self) -> &HashSet<Box<dyn Migration<Database = Self::Database>>> {
        &self.migrations
    }

    fn migrations_mut(&mut self) -> &mut HashSet<Box<dyn Migration<Database = Self::Database>>> {
        &mut self.migrations
    }

    fn pool(&self) -> &Pool<Self::Database> {
        &self.pool
    }

    async fn ensure_migration_table_exists(&self) -> Result<(), Error> {
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

    async fn add_migration_to_db_table<'t>(
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

    async fn delete_migration_from_db_table<'t>(
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

    async fn fetch_applied_migration_from_db(&self) -> Result<Vec<String>, Error> {
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
