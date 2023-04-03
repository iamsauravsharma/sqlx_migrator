# SQLX migrator

Migrator for writing sqlx migration using Rust instead of SQL

|                    License                     |              Crates Version               |                 Docs                 |
| :--------------------------------------------: | :---------------------------------------: | :----------------------------------: |
| [![License: MIT][license_badge]][license_link] | [![Crate][cratesio_badge]][cratesio_link] | [![Docs][docsrs_badge]][docsrs_link] |

Supported Databases:

- [x] PostgreSQL
- [x] SQLite
- [x] MySql
- [x] Any

## Installation

To use postgres migrator along with tokio-native-tls runtime you can configure Cargo.toml

```toml
sqlx_migrator = { version = "0.5.1", features=["postgres", "runtime-tokio-rustls"] }
```

# Usage

To use sqlx_migrator first you need to implement OperationTrait trait to write your sqlx operation

```rust
use sqlx_migrator::error::Error;
use sqlx_migrator::operation::OperationTrait;
// Its better to use sqlx imported from sqlx_migrator
use sqlx_migrator::sqlx;

pub(crate) struct FirstOperation;

#[async_trait::async_trait]
impl OperationTrait<sqlx::Postgres> for FirstOperation {
    // Up function runs apply migration
    async fn up(&self, connection: &mut sqlx::PgConnection) -> Result<(), Error> {
        sqlx::query("CREATE TABLE sample (id INTEGER PRIMARY KEY, name TEXT)")
            .execute(connection)
            .await?;
        Ok(())
    }

    // down migration runs down migration
    async fn down(&self, connection: &mut sqlx::PgConnection) -> Result<(), Error> {
        sqlx::query("DROP TABLE sample").execute(connection).await?;
        Ok(())
    }
}
```

After creation of operation you can implement Migration struct to create single migration

```rust
use sqlx_migrator::error::Error;
use sqlx_migrator::migration::MigrationTrait;
use sqlx_migrator::operation::OperationTrait;
use sqlx_migrator::sqlx;

pub(crate) struct FirstMigration;

impl MigrationTrait<sqlx::Postgres> for FirstMigration {
    // app where migration lies can be any value
    fn app(&self) -> &str {
        "main"
    }

    // name of migration
    // Combination of migration app and name must be unique to work properly
    fn name(&self) -> &str {
        "first_migration"
    }

    // use parents function to add parents of a migrations.
    fn parents(&self) -> Vec<Box<dyn MigrationTrait<DB>>> {
        vec![]
    }

    // use operations function to add operation part of migration
    fn operations(&self) -> Vec<Box<dyn OperationTrait<sqlx::Postgres>>> {
        vec![Box::new(FirstOperation)]
    }

    // MigrationTrait trait also have multiple other function see docs for usage
}
```

Now at last you need to create migrator for your database to run migrations

```rust
use sqlx_migrator::migrator::{Migrator, MigratorTrait};
use sqlx_migrator::sqlx;

fn main() {
    let uri = std::env::var("DATABASE_URL").unwrap();
    let pool = sqlx::Pool::<sqlx::Postgres>::connect(&uri).await.unwrap();
    let mut migrator = Migrator::new(&pool);
    migrator.add_migration(Box::new(FirstMigration));
}
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

# Migrate from sqlx default sql based migration

To migrate from sqlx sql based migration you have two alternative:

1. Write all sql migration as rust operation
2. Write single rust based operation which apply and revert all sqlx sql based migration

#### Option: 1

Can be easily applied by following above usage docs where you only need to write your sql based migration as sqlx query
Then you can create cli for migrator

```rust
sqlx_migrator::cli::run(Box::new(migrator)).await.unwrap();
```

and run fake apply cli command
`<COMMAND_NAME> apply --fake`
which actually doesn't apply migration query but only update migration table

#### Option: 2

To run all sqlx sql based migration as single command create new operation

```rust
use sqlx_migrator::error::Error;
use sqlx_migrator::operation::OperationTrait;
use sqlx_migrator::sqlx;

pub(crate) struct SqlxOperation;

#[async_trait::async_trait]
impl OperationTrait<sqlx::Postgres> for SqlxOperation {
    async fn up(&self, connection: &mut sqlx::PgConnection) -> Result<(), Error> {
        sqlx::migrate!("db/migrations").run(connection).await?;
        Ok(())
    }

    async fn down(&self, connection: &mut sqlx::PgConnection) -> Result<(), Error> {
        sqlx::migrate!("db/migrations").undo(connection, 0).await?;
        Ok(())
    }
}
```

[license_badge]: https://img.shields.io/github/license/iamsauravsharma/sqlx_migrator.svg?style=for-the-badge
[license_link]: LICENSE
[cratesio_badge]: https://img.shields.io/crates/v/sqlx_migrator.svg?style=for-the-badge
[cratesio_link]: https://crates.io/crates/sqlx_migrator
[docsrs_badge]: https://img.shields.io/docsrs/sqlx_migrator/latest?style=for-the-badge
[docsrs_link]: https://docs.rs/sqlx_migrator
