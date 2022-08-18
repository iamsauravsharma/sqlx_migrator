# SQLX migrator
Migrator for writing sqlx migration using Rust instead of SQL

| License | Crates Version | Docs |
| :---: | :---: | :---: |
| [![License: MIT][license_badge]][license_link] | [![Crate][cratesio_badge]][cratesio_link] | [![Docs][docsrs_badge]][docsrs_link] |

Supported Databases:
- [X] PostgreSQL
- [X] SQLite

## Installation

```toml
sqlx_migrator = "0.2.0"
```

# Usage

To use sqlx_migrator first you need to implement Operation trait to write your sqlx operation
```rust
use sqlx_migrator::error::Error;
use sqlx_migrator::operation::Operation;
//Its better to use sqlx imported from sqlx_migrator
use sqlx_migrator::sqlx;

pub(crate) struct FirstOperation;

#[async_trait::async_trait]
impl Operation for FirstOperation {
    type Database = sqlx::Postgres;

    // Up function runs apply migration
    async fn up(
        &self,
        connection: &mut <Self::Database as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        sqlx::query("CREATE TABLE sample (id INTEGER PRIMARY KEY, name TEXT)")
            .execute(connection)
            .await?;
        Ok(())
    }

    // down migration runs down migration
    async fn down(
        &self,
        connection: &mut <Self::Database as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        sqlx::query("DROP TABLE sample").execute(connection).await?;
        Ok(())
    }
}
```

After creation of operation you can implement Migration struct to create single migration

```rust
use sqlx_migrator::error::Error;
use sqlx_migrator::migration::Migration;
use sqlx_migrator::operation::Operation;
use sqlx_migrator::sqlx;

pub(crate) struct FirstMigration;

#[async_trait::async_trait]
impl Migration for FirstMigration {
    type Database = Postgres;

    // app where migration lies can be any value
    fn app(&self) -> &str {
        "main"
    }

    // name of migration
    // Combination of migration app and name must be unique to work properly
    fn name(&self) -> &str {
        "first_migration"
    }

    // use operations function to add operation part of migration
    fn operations(&self) -> Vec<Box<dyn Operation<Database = Self::Database>>> {
        vec![Box::new(FirstOperation)]
    }

    // Migration trait also have multiple other function see docs for usage
}
```

Now at last you need to create migrator for your database to run migrations
```rust
use sqlx_migrator::migrator::Migrator as MigratorTrait;
use sqlx_migrator::sqlite::migrator::Migrator;

let uri = std::env::var("DATABASE_URL").unwrap();
let pool = sqlx::Pool::connect(&uri).await.unwrap();
let mut migrator = Migrator::new(&pool);
migrator.add_migration(FirstMigration);
```

Now you can use two ways to run migrator either directly running migration or creating cli from migrator
For directly run
```rust
// use apply all to apply all pending migration
migrator.apply_all().await.unwrap();
// or use revert all to revert all applied migrations
migrator.revert_all().await.unwrap();
```
Or you can create cli
```rust
sqlx_migrator::cli::run(Box::new(migrator)).await.unwrap();
```

[license_badge]: https://img.shields.io/github/license/iamsauravsharma/sqlx_migrator.svg?style=for-the-badge
[license_link]: LICENSE

[cratesio_badge]: https://img.shields.io/crates/v/sqlx_migrator.svg?style=for-the-badge
[cratesio_link]: https://crates.io/crates/sqlx_migrator

[docsrs_badge]: https://img.shields.io/docsrs/sqlx_migrator/latest?style=for-the-badge
[docsrs_link]: https://docs.rs/sqlx_migrator
