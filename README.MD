# SQLX migrator

A Rust library for writing SQLX migrations using Rust instead of SQL.

|                    License                     |              Crates Version               |                 Docs                 |
| :--------------------------------------------: | :---------------------------------------: | :----------------------------------: |
| [![License: MIT][license_badge]][license_link] | [![Crate][cratesio_badge]][cratesio_link] | [![Docs][docsrs_badge]][docsrs_link] |

Supported Databases:

- [x] PostgreSQL
- [x] SQLite
- [x] MySql
- [x] Any

## Installation

Add `sqlx_migrator` to your `Cargo.toml` with the appropriate database feature:

```toml
sqlx_migrator = { version = "0.17.0", features=["postgres"] }
```

OR

```toml
sqlx_migrator = { version = "0.17.0", features=["mysql"] }
```

OR

```toml
sqlx_migrator = { version = "0.17.0", features=["sqlite"] }
```

OR

```toml
sqlx_migrator = { version = "0.17.0", features=[
    "any",
    # Plus any one of above database driver
    ] }
```

# Usage

To use `sqlx_migrator`, implement the `Operation` trait to define your migration logic. Here's an example using PostgreSQL:

```rust
use sqlx_migrator::error::Error;
use sqlx_migrator::operation::Operation;

pub(crate) struct FirstOperation;

#[async_trait::async_trait]
impl Operation<sqlx::Postgres> for FirstOperation {
    // Up function runs apply migration
    async fn up(&self, connection: &mut sqlx::PgConnection) -> Result<(), Error> {
        sqlx::query("CREATE TABLE sample (id INTEGER PRIMARY KEY, name TEXT)")
            // NOTE: if you want to use connection multiple times pass `&mut *connection`
            // as a parameter instead of `connection`
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
After defining your operations, you can create a migration:

```rust
use sqlx_migrator::error::Error;
use sqlx_migrator::migration::Migration;
use sqlx_migrator::operation::Operation;

pub(crate) struct FirstMigration;

impl Migration<sqlx::Postgres> for FirstMigration {
    // app where migration lies can be any value
    fn app(&self) -> &str {
        "main"
    }

    // name of migration
    // Combination of migration app and name must be unique to work properly expects for virtual migration
    fn name(&self) -> &str {
        "first_migration"
    }

    // Use the parent function to add parents of a migration.
    // If you cannot access or create the parent migration easily, you can also use
    // `(A,N) where A: AsRef<str>, N: AsRef<str>` where A is the app name
    // and N is the name of the migration.
    fn parents(&self) -> Vec<Box<dyn Migration<sqlx::Postgres>>> {
        vec![]
        // vec![("main", "initial_migration"), AnotherInitialMigration]
    }

    // use operations function to add operation part of migration
    fn operations(&self) -> Vec<Box<dyn Operation<sqlx::Postgres>>> {
        vec![Box::new(FirstOperation)]
    }

    // Migration trait also have multiple other function see docs for usage
}
```

This migration can be represented in a simpler form using macros:
```rust
use sqlx_migrator::vec_box;
sqlx_migrator::migration!(
    sqlx::Postgres,
    FirstMigration,
    "main",
    "first_migration",
    vec_box![], 
    vec_box![FirstOperation]
);
// OR
sqlx_migrator::postgres_migration!(
    FirstMigration,
    "main",
    "first_migration",
    vec_box![], 
    vec_box![FirstOperation]
);
```

If your up and down queries are simple strings, you can simplify the implementation:
```rust
sqlx_migrator::postgres_migration!(
    FirstMigration,
    "main",
    "first_migration",
    sqlx_migrator::vec_box![],
    sqlx_migrator::vec_box![
        (
            "CREATE TABLE sample (id INTEGER PRIMARY KEY, name TEXT)",
            "DROP TABLE sample"
        )
    ]
);
```

Finally, create a migrator to run your migrations:

```rust
use sqlx_migrator::migrator::{Info, Migrate, Migrator};
use sqlx::Postgres;

#[tokio::main]
async fn main() {
    let uri = std::env::var("DATABASE_URL").unwrap();
    let pool = sqlx::Pool::<Postgres>::connect(&uri).await.unwrap();
    let mut migrator = Migrator::default();
    // Adding migration can fail if another migration with same app and name and different values gets added
    // Adding migrations add its parents, replaces and not before as well
    migrator.add_migration(Box::new(FirstMigration)).unwrap();
}
```

# Running Migrations

You can run migrations directly or integrate them into a CLI:
## Programmatic Execution
```rust
use sqlx_migrator::migrator::Plan;
let mut conn = pool.acquire().await?;
// use apply all to apply all pending migration
migrator.run(&mut *conn, Plan::apply_all()).await.unwrap();
// or use revert all to revert all applied migrations
migrator.run(&mut *conn, Plan::revert_all()).await.unwrap();
// If you need to apply or revert to certain stage than see `Plan` docs
```

## CLI Integration
To integrate sqlx_migrator into your CLI, you can either use the built-in
`MigrationCommand` or extend your own CLI with migrator support. Below are
examples of both approaches:

#### Built-in Migration Command

```rust
use sqlx_migrator::cli::MigrationCommand;

MigrationCommand::parse_and_run(&mut *conn, Box::new(migrator)).await.unwrap();
```

#### Extending Your Own CLI with Migrator Support

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
        // create connection
        match cli.sub_command {
            Migrator(m) => {
                m.run(&mut conn, Box::new(migrator)).await.unwrap()
            }
        }
    }
}
```

# Migrate from sqlx default sql based migration

To migrate from sqlx sql based migration to rust migration the recommended approach
is to rewrite your SQL migrations as Rust operations and migrations as explained above.
After rewriting your SQL migrations, need to ensure your existing migrations are properly recognized without being re-executed. This ensures that the migration state aligns with the existing database.
This guide explains three methods to achieve this "fake apply" process.

### 1. Programmatic Fake Apply

Use the fake option with the `Plan::apply_all()` function:
```rust
use sqlx_migrator::migrator::Plan;

migrator.run(&mut *conn, Plan::apply_all().fake(true)).await.unwrap();
```

#### Important notes
- Run this exactly once during your migration process
- Remove the `fake(true)` option after the initial migration
- Ensure no other migrations are pending when you execute this

### 2. CLI-Based Fake Apply
If you're using a CLI, use the` --fake` flag with the apply command: `<migrator_cli_command> apply --fake`

#### Important notes
- Run with `--fake` flag once to mark existing migrations
- Best suited for environments where you have direct CLI access

### 3. Custom Fake Apply

This method involves manually mapping your SQLx migrations to sqlx-migrator entries before running migrator normally. This can be performed as explained below and can be run multiple times where as above solution cannot be run multiple times

#### Implementation guide
 - Create a mapping between old and new migration names
 - Query existing migrations from `_sqlx_migrations`
 - Insert corresponding records into `_sqlx_migrator_migrations`
 - Now perform migrations normally
 - If you prefer remove this custom code after all instances have migrated

[license_badge]: https://img.shields.io/github/license/iamsauravsharma/sqlx_migrator.svg?style=for-the-badge
[license_link]: LICENSE
[cratesio_badge]: https://img.shields.io/crates/v/sqlx_migrator.svg?style=for-the-badge
[cratesio_link]: https://crates.io/crates/sqlx_migrator
[docsrs_badge]: https://img.shields.io/docsrs/sqlx_migrator/latest?style=for-the-badge
[docsrs_link]: https://docs.rs/sqlx_migrator
