//! Postgres migrator module
use std::collections::HashSet;

use sqlx::{Pool, Postgres, Row, Transaction};

use crate::error::Error;
use crate::migration::Migration;
use crate::migrator::Migrator as MigratorTrait;

/// Migrator struct which store migrations graph and information related to
/// postgres migrations
pub struct Migrator {
    migrations: HashSet<Box<dyn Migration<Database = Postgres>>>,
    pool: Pool<Postgres>,
}

impl Migrator {
    /// Create new migrator from pool
    #[must_use]
    pub fn new(pool: &Pool<Postgres>) -> Self {
        Self {
            migrations: HashSet::new(),
            pool: pool.clone(),
        }
    }
}

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    type Database = Postgres;

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
    id SERIAL PRIMARY KEY,
    migration_full_name TEXT UNIQUE NOT NULL,
    applied_time TIMESTAMPTZ NOT NULL DEFAULT now()
)
            "#,
        )
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn add_migration_to_db_table<'t>(
        &self,
        migration_full_name: &str,
        transaction: &mut Transaction<'t, Self::Database>,
    ) -> Result<(), Error> {
        sqlx::query(
            r#"
INSERT INTO _sqlx_migrator_migrations(migration_full_name) VALUES ($1)
            "#,
        )
        .bind(migration_full_name)
        .execute(transaction)
        .await?;

        Ok(())
    }

    async fn delete_migration_from_db_table<'t>(
        &self,
        migration_full_name: &str,
        transaction: &mut Transaction<'t, Self::Database>,
    ) -> Result<(), Error> {
        sqlx::query(
            r#"
DELETE FROM _sqlx_migrator_migrations WHERE migration_full_name = $1
            "#,
        )
        .bind(migration_full_name)
        .execute(transaction)
        .await?;
        Ok(())
    }

    async fn fetch_applied_migration_from_db(&self) -> Result<Vec<String>, Error> {
        let mut applied_migrations = Vec::new();
        let rows = sqlx::query("SELECT migration_full_name FROM _sqlx_migrator_migrations")
            .fetch_all(self.pool())
            .await?;

        for row in rows {
            let migration_full_name = row.try_get("migration_full_name")?;
            applied_migrations.push(migration_full_name);
        }
        Ok(applied_migrations)
    }
}
