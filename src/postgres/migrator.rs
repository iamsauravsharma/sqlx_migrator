//! Postgres migrator module
use std::collections::HashSet;

use sqlx::{Pool, Postgres};

use crate::error::Error;
use crate::migration::Migration;
use crate::migrator::{Migrator as MigratorTrait, SqlMigratorMigration};

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
    app TEXT NOT NULL,
    name TExT NOT NULL,
    applied_time TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (app, name)
)
            "#,
        )
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn add_migration_to_db_table(
        &self,
        migration: &Box<dyn Migration<Database = Self::Database>>,
        connection: &mut <Self::Database as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        sqlx::query(
            r#"
INSERT INTO _sqlx_migrator_migrations(app, name) VALUES ($1, $2)
            "#,
        )
        .bind(migration.app())
        .bind(migration.name())
        .execute(connection)
        .await?;
        Ok(())
    }

    async fn delete_migration_from_db_table(
        &self,
        migration: &Box<dyn Migration<Database = Self::Database>>,
        connection: &mut <Self::Database as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        sqlx::query(
            r#"
DELETE FROM _sqlx_migrator_migrations WHERE app = $1 AND name = $2
            "#,
        )
        .bind(migration.app())
        .bind(migration.name())
        .execute(connection)
        .await?;
        Ok(())
    }

    async fn fetch_applied_migration_from_db(&self) -> Result<Vec<SqlMigratorMigration>, Error> {
        let rows =
            sqlx::query_as("SELECT id, app, name, applied_time FROM _sqlx_migrator_migrations")
                .fetch_all(self.pool())
                .await?;
        Ok(rows)
    }
}
