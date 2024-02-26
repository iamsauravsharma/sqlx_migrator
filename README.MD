# SQLX migrator

Migrator library for writing sqlx migration using Rust instead of SQL

|                    License                     |              Crates Version               |                 Docs                 |
| :--------------------------------------------: | :---------------------------------------: | :----------------------------------: |
| [![License: MIT][license_badge]][license_link] | [![Crate][cratesio_badge]][cratesio_link] | [![Docs][docsrs_badge]][docsrs_link] |

Supported Databases:

- [x] PostgreSQL
- [x] SQLite
- [x] MySql
- [x] Any

## Installation

To use sqlx migrator you can configure Cargo.toml as shown below according to your requirements

```toml
sqlx_migrator = { version = "0.12.0", features=["postgres"] }
```

OR

```toml
sqlx_migrator = { version = "0.12.0", features=["mysql"] }
```

OR

```toml
sqlx_migrator = { version = "0.12.0", features=["sqlite"] }
```

OR

```toml
sqlx_migrator = { version = "0.12.0", features=[
    "any",
    # Plus any one of above database driver
    ] }
```

# Usage

To use sqlx_migrator first you need to implement Operation trait to write your sqlx operation below are examples for using postgres based migration

```rust
use sqlx_migrator::error::Error;
use sqlx_migrator::operation::Operation;
// Its better to use sqlx imported from sqlx_migrator
use sqlx_migrator::sqlx;

pub(crate) struct FirstOperation;

#[async_trait::async_trait]
impl Operation<sqlx::Postgres> for FirstOperation {
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
If you have simple up operation and down operation than you can use macros for creating operation
```rust
use sqlx_migrator::sqlx;

pub(crate) struct FirstOperation;

sqlx_migrator::operation!(
    sqlx::Postgres,
    FirstOperation,
    "CREATE TABLE sample (id INTEGER PRIMARY KEY, name TEXT)",
    "DROP TABLE sample",
    );
```
You can even simplify it using
```rust
sqlx_migrator::postgres_operation!(
    FirstOperation,
    "CREATE TABLE sample (id INTEGER PRIMARY KEY, name TEXT)",
    "DROP TABLE sample",
    );
```

After creation of operation you can implement Migration struct to create single migration

```rust
use sqlx_migrator::error::Error;
use sqlx_migrator::migration::Migration;
use sqlx_migrator::operation::Operation;
use sqlx_migrator::sqlx;

pub(crate) struct FirstMigration;

impl Migration<sqlx::Postgres> for FirstMigration {
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
    fn parents(&self) -> Vec<Box<dyn Migration<sqlx::Postgres>>> {
        vec![]
    }

    // use operations function to add operation part of migration
    fn operations(&self) -> Vec<Box<dyn Operation<sqlx::Postgres>>> {
        vec![Box::new(FirstOperation)]
    }

    // Migration trait also have multiple other function see docs for usage
}
```

Which can be represented in simple form as
```rust
use sqlx_migrator::vec_box;
sqlx_migrator::migration!(sqlx::Postgres, "main", FirstMigration, vec_box![], vec_box![FirstOperation]);
// OR
sqlx_migrator::postgres_migration!("main", FirstMigration, vec_box![], vec_box![FirstOperation]);
```

Now at last you need to create migrator for your database to run migrations

```rust
use sqlx_migrator::migrator::{Info, Migrate, Migrator};
use sqlx_migrator::sqlx::Postgres;

#[tokio::main]
async fn main() {
    let uri = std::env::var("DATABASE_URL").unwrap();
    let pool = sqlx::Pool::<Postgres>::connect(&uri).await.unwrap();
    let mut migrator = Migrator::default();
    migrator.add_migration(Box::new(FirstMigration));
}
```

Now you can use two ways to run migrator either directly running migration or creating cli from migrator
For direct run

```rust
// use apply all to apply all pending migration
migrator.run(&pool, sqlx_migrator::migrator::Plan::apply_all()).await.unwrap();
// or use revert all to revert all applied migrations
migrator.run(&pool, sqlx_migrator::migrator::Plan::revert_all()).await.unwrap();
```

Or you can create cli using

```rust
use sqlx_migrator::cli::MigrationCommand;
MigrationCommand::parse_and_run(Box::new(migrator), &pool).await.unwrap();
```

If you want to extend your own clap based cli then you can add migrator to sub command enum and then run migrator
```rust
#[derive(clap::Parser)]
struct Cli {
    #[command(subcommand)]
    sub_command: CliSubcommand
}

#[derive(clap::Subcommand)]
enum CliSubcommand {
    #[command()]
    Migrator(sqlx_migrator::cli::MigrationCommand)
}

impl Cli {
    async fn run() {
        let cli = Self::parse();
        match cli.sub_command {
            Migrator(m) => {
                m.run(Box::new(migrator), &pool).await.unwrap()
            }
        }
    }
}
```

# Migrate from sqlx default sql based migration

To migrate from sqlx sql based migration you have two alternative:

1. Write all sql migration as rust operation
2. Write single rust based operation which apply and revert all sqlx sql based migration

#### Option: 1

Can be easily applied by following above usage docs where you only need to write your sql based migration as sqlx query
Then you can create cli for migrator

```rust
use sqlx_migrator::cli::MigrationCommand;
MigrationCommand::parse_and_run(Box::new(migrator), &pool).await.unwrap();
```

and run fake apply cli command
`<COMMAND_NAME> apply --fake`
which actually doesn't apply migration query but only update migration table

#### Option: 2

To run all sqlx sql based migration as single command create new operation

```rust
use sqlx_migrator::error::Error;
use sqlx_migrator::operation::Operation;
use sqlx_migrator::sqlx;

pub(crate) struct SqlxOperation;

#[async_trait::async_trait]
impl Operation<sqlx::Postgres> for SqlxOperation {
    async fn up(&self, connection: &mut sqlx::PgConnection) -> Result<(), Error> {
        sqlx::migrate!("migrations").run(connection).await?;
        Ok(())
    }

    async fn down(&self, connection: &mut sqlx::PgConnection) -> Result<(), Error> {
        sqlx::migrate!("migrations").undo(connection, 0).await?;
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
