# SQLX migrator

A Rust library for writing SQLX migrations using Rust instead of SQL.

|                    License                     |              Crates Version               |                 Docs                 |
| :--------------------------------------------: | :---------------------------------------: | :----------------------------------: |
| [![License: MIT][license_badge]][license_link] | [![Crate][cratesio_badge]][cratesio_link] | [![Docs][docsrs_badge]][docsrs_link] |

Supported Databases:

- [x] PostgreSQL
- [x] SQLite
- [x] MySql

## Installation

Add `sqlx_migrator` to your `Cargo.toml` with the appropriate database feature:

```toml
sqlx_migrator = { version = "0.20.0", features=["postgres"] }
```

OR

```toml
sqlx_migrator = { version = "0.20.0", features=["mysql"] }
```

OR

```toml
sqlx_migrator = { version = "0.20.0", features=["sqlite"] }
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
    // Combination of migration app and name must be unique to work properly except for virtual migration
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

If your up and down queries are simple strings, you can simplify the implementation.
A `(up, down)` tuple is a reversible operation and a single string is an irreversible one:
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
        ),
        "INSERT INTO sample (id, name) VALUES (1, 'seed')"
    ]
);
```

SQL can also be kept in files next to the migration and embedded at compile time.
Paths are resolved relative to the invoking file, like `include_str!`. A base path
expands to `<base>.up.sql` and `<base>.down.sql`; explicit `up:` and `down:` files
can be given instead, and `up:` alone creates an irreversible operation:
```rust
sqlx_migrator::postgres_migration!(
    FirstMigration,
    "main",
    "first_migration",
    sqlx_migrator::vec_box![],
    sqlx_migrator::vec_box![
        sqlx_migrator::include_sql!("sql/0001"),
        sqlx_migrator::include_sql!(up: "sql/0001_seed.sql"),
    ]
);
```

The macros cover the simple case. For a migration which needs `replaces`, `run_before`
or `is_atomic`, implement the `Migration` trait by hand as shown above.

Finally, create a migrator to run your migrations:

```rust
use sqlx_migrator::migrator::Migrator;
use sqlx::Postgres;

#[tokio::main]
async fn main() {
    let uri = std::env::var("DATABASE_URL").unwrap();
    let pool = sqlx::Pool::<Postgres>::connect(&uri).await.unwrap();
    let mut migrator = Migrator::default();
    // Adding migration can fail if another migration with same app and name and different values gets added
    // Adding migrations add its parents, replaces and run before as well
    migrator.add_migration(Box::new(FirstMigration)).unwrap();
}
```

## Migration table

Applied migrations are recorded in `_sqlx_migrator_migrations`, with an optional prefix
and schema (see `Dialect::postgres_with`, `Dialect::mysql_with` and `Dialect::sqlite_with`). The table is
created and changed by the migrator's own internal migrations, a numbered list of SQL
statements per database. A small meta table next to it, `_sqlx_migrator_meta`,
records the name of every internal migration which has run. When a release adds an internal
migration, older databases run it the next time migrations run; a database changed by
an internal migration this release does not know is refused rather than guessed at.
Each internal migration is safe to rerun, so databases created before the meta table
existed simply catch up.

## Locking and transactions

While a plan runs, the migrator holds a database lock so two processes cannot
migrate at the same time. Postgres uses a session level advisory lock and MySQL uses
`GET_LOCK`; SQLite serialises writers itself. Session level locks are not reliable
behind a transaction pooling proxy such as PgBouncer, and are unnecessary when a
single process runs migrations, so the lock can be removed from the dialect
(see "Custom SQL" below):

```rust
use sqlx_migrator::{Dialect, Migrator};
let migrator = Migrator::<Postgres>::new(Dialect::postgres().set_lock(false));
```

Atomic migrations run inside a transaction. MySQL and MariaDB commit implicitly on
most DDL statements, so on those backends a failed atomic migration cannot be rolled
back as a unit.

## Custom SQL

A migrator is configured by a `Dialect` value holding the table prefix, schema and advisory lock
options, plus the SQL built from them. `Migrator::default` uses the built in
`Dialect::postgres`, `Dialect::mysql` or `Dialect::sqlite`. The table prefix and schema are arguments of the `_with`
constructors, for example `Dialect::postgres_with(Some("prefix"), Some("schema"))`. Adjust a built in dialect with `set_lock` and `set_create_schema` where the database has them, and pass it to `Migrator::new`:

```rust
use sqlx::Postgres;
use sqlx_migrator::{Dialect, Migrator};

let migrator = Migrator::<Postgres>::new(Dialect::postgres().set_lock(false));
```

Generic code over the database uses `SupportedDatabase`, which every sqlx database the
crate can execute SQL against implements automatically.

# Running Migrations

You can run migrations directly or integrate them into a CLI:
## Programmatic Execution
```rust
use sqlx_migrator::migrator::Plan;
let mut conn = pool.acquire().await?;
// use apply all to apply all pending migration
let report = migrator.run(&mut *conn, &Plan::apply_all()).await.unwrap();
// the report lists what ran and how long each migration took
for record in report.records() {
    println!("{}:{} took {:?}", record.app(), record.name(), record.elapsed());
}
// or use revert all to revert all applied migrations
migrator.run(&mut *conn, &Plan::revert_all()).await.unwrap();
// If you need to apply or revert to certain stage then see `Plan` docs
```

Progress is emitted through the `tracing` crate at `info` level, with a span per run
and per migration carrying the app and name, so any `tracing` subscriber can show it.

## CLI Integration
To integrate sqlx_migrator into your CLI, you can either use the built-in
`MigrationCommand` or extend your own CLI with migrator support. Below are
examples of both approaches:

#### Built-in Migration Command

```rust
use sqlx_migrator::cli::MigrationCommand;

MigrationCommand::parse_and_run(&mut *conn, &migrator).await.unwrap();
```

The command provides these subcommands:

| Subcommand | Purpose |
| --- | --- |
| `apply` | Apply pending migrations. `--plan` shows what would run, `--check` fails when something is pending, `--fake` only records them, `--app` / `--migration` / `--count` limit the plan. |
| `revert` | Revert the last applied migration by default; `--all`, `--app` / `--migration` or `--count` choose more. Asks for confirmation unless `--force` is passed. |
| `redo` | Revert the last applied migration and apply it again, handy while developing a migration. |
| `list` | Show every migration with its status: applied, pending, replaced, or unknown (in the table but no longer in the code). |
| `prune` | Delete unknown entries from the migration table after migrations were removed from the code. Asks for confirmation unless `--force` is passed. |
| `drop` | Drop the migration table once every migration is reverted. |

The default output is a table sized to its content. With the optional `json` feature
enabled, `list`, `apply --plan` and `revert --plan` also accept `--format json` for
scripting:

```toml
sqlx_migrator = { version = "0.20.0", features=["postgres", "json"] }
```

Confirmation prompts need a terminal, so pass `--force` when running non-interactively.

#### Extending Your Own CLI with Migrator Support

```rust
#[derive(clap::Parser)]
struct Cli {
    #[command(subcommand)]
    sub_command: CliSubcommand
}

#[derive(clap::Subcommand)]
enum CliSubcommand {
    Migrator(sqlx_migrator::cli::MigrationCommand)
}

impl Cli {
    async fn run() {
        let cli = Self::parse();
        // create connection
        match cli.sub_command {
            Migrator(m) => {
                m.run(&mut conn, &migrator).await.unwrap()
            }
        }
    }
}
```

# Migrate from old migrator migration to `sqlx_migrator` migration

To transition from your old migration system to `sqlx_migrator`, follow these steps:

### 1. Create Corresponding Migrations
For each migration in your old system, create an equivalent migration in `sqlx_migrator` format. This ensures all historical migrations are properly represented in the new system.

### 2. Implement the OldMigrator Trait
The `OldMigrator` trait allows you to bridge between your old migration system and `sqlx_migrator`. Here's a implementation example for sqlx sql:

```rust
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
        let old_migrations: Vec<(i64, String)> = sqlx::query_as(
            "SELECT version, description FROM _sqlx_migrations ORDER BY version",
        )
        .fetch_all(connection)
        .await?;
        let mut migrations: Vec<Box<dyn Migration<Postgres>>> = vec![];
        // your custom logic here to map from old migrator to new migration
        // for example for sqlx you may use a migration name as combination of both version
        // and description or only one between them it is your choice
        // and sqlx migrator have no preference between them
        for (version, description) in old_migrations {
            migrations.push(Box::new(("main", format!("{version}_{description}"))));
        }
        Ok(migrations)
    }
}
```

### 3. Run sync
Before running your main migration code, execute the sync operation

```rust
let sqlx_sql_migrator = SqlxMigrator;
migrator.sync(&mut *conn, &sqlx_sql_migrator).await?;
```

You can also implement above functionality for other migrators such as `diesel` or `seaorm`.
The implementation may be generic over the database. A `Migrator` is itself an
`OldMigrator`, which covers renaming the migration table or moving it to another schema:
build a migrator with the old table prefix or schema and sync from it.

[license_badge]: https://img.shields.io/github/license/iamsauravsharma/sqlx_migrator.svg?style=for-the-badge
[license_link]: LICENSE
[cratesio_badge]: https://img.shields.io/crates/v/sqlx_migrator.svg?style=for-the-badge
[cratesio_link]: https://crates.io/crates/sqlx_migrator
[docsrs_badge]: https://img.shields.io/docsrs/sqlx_migrator/latest?style=for-the-badge
[docsrs_link]: https://docs.rs/sqlx_migrator
