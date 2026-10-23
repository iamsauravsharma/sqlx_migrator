# Moving from another migrator

Create an equivalent migration for each old one, implement [`OldMigrator`] to list the
old applied migrations, then sync once before running:

```rust,ignore
use sqlx::Postgres;
use sqlx_migrator::error::Error;
use sqlx_migrator::migration::Migration;
use sqlx_migrator::sync::OldMigrator;

struct SqlxMigrator;

#[async_trait::async_trait]
impl OldMigrator<Postgres> for SqlxMigrator {
    async fn applied_migrations(
        &self,
        connection: &mut sqlx::PgConnection,
    ) -> Result<Vec<Box<dyn Migration<Postgres>>>, Error> {
        let rows: Vec<(i64, String)> =
            sqlx::query_as("SELECT version, description FROM _sqlx_migrations")
                .fetch_all(connection)
                .await?;
        let mut migrations: Vec<Box<dyn Migration<Postgres>>> = vec![];
        for (version, description) in rows {
            migrations.push(Box::new(("main", format!("{version}_{description}"))));
        }
        Ok(migrations)
    }
}

migrator.sync(&mut *conn, &SqlxMigrator).await?;
```

A `Migrator` is itself an `OldMigrator`, so renaming the table or schema is a sync from
a migrator built with the old prefix or schema.
