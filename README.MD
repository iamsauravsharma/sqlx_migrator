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
and schema (see `Dialect::postgres_with`, `Dialect::mysql_with` and `Dialect::sqlite_with`).
The migrator creates and changes this table itself with numbered internal migrations. A
small meta table next to it, `_sqlx_migrator_meta`, records which of them have run, so each
runs once. A database changed by an internal migration this release does not know is
refused.

Each row records the migration's `app`, `name` and `applied_time`, plus:

- `success`: `false` while a non atomic migration is running or after it failed.
- `execution_time_ns`: how long the migration took in nanoseconds, `0` for fake runs and synced migrations.
- `batch`: migrations applied by one run share a batch, `revert --last-batch` reverts it.
  Rows already in the table when this column is added get their `id` as batch, so they
  count as one batch each in apply order. Synced migrations are recorded as one batch.

A non atomic migration is recorded with `success = false` before its operations run and
marked successful after the last one. If it fails part way the database is partly
migrated, so every run is refused with `Error::FailedMigration` until you fix the database
by hand and run `repair`, which removes the failed entry so the migration runs again.
Atomic migrations are recorded in the same transaction as their operations, so they never
leave a failed entry.

## Hooks

Register a function with `Migrator::add_hook` and a `HookType` (`BeforeRun`, `AfterRun`,
`BeforeMigration`, `AfterMigration`) saying when it runs. It gets the connection doing the
work, which is the transaction of an atomic migration, so `SET LOCAL lock_timeout` works, and
a `HookContext` with the direction, the migration or the run report. Migration hooks are
skipped by a fake plan. A plain function or a closure returning a boxed future works:

```rust,ignore
use sqlx::Executor as _;
use sqlx_migrator::hook::HookType;

migrator.add_hook(HookType::BeforeMigration, |connection, _context| {
    Box::pin(async move {
        connection.execute("SET LOCAL lock_timeout = '5s'").await?;
        Ok(())
    })
});
```

## Locking and transactions

While a plan runs the migrator locks, so two processes cannot migrate at once. Choose the
lock with `Dialect::set_lock_strategy`, which fails for a lock the database does not have:

| `LockStrategy` | Use |
| --- | --- |
| `Advisory` | A lock held by the connection, the default on Postgres and MySQL. Not reliable behind a transaction pooling proxy such as PgBouncer. |
| `None` | No lock, for a single process. The default on SQLite, which serialises writers. |
| `Row` | A row in a `_sqlx_migrator_lock` table, on every database. A run which finds the row fails with `Error::LockHeld`. A crashed run leaves its row, delete it by hand. |
| `TransactionAdvisory` | Postgres only. An advisory lock held until the end of one transaction which holds the whole plan, so it works behind PgBouncer, the plan is applied entirely or not at all and every migration must be atomic. |
| `TableShareUpdateExclusive` | Postgres only. Like `TransactionAdvisory`, and also `LOCK TABLE` on the migration table in `SHARE UPDATE EXCLUSIVE` mode, which other transactions can still read. |
| `TableAccessExclusive` | Postgres only. Like `TransactionAdvisory`, and also `LOCK TABLE` in `ACCESS EXCLUSIVE` mode, which blocks reads of the migration table too. |

```rust
use sqlx_migrator::migrator::LockStrategy;
use sqlx_migrator::{Dialect, Migrator};
let dialect = Dialect::postgres().set_lock_strategy(LockStrategy::TransactionAdvisory).unwrap();
let migrator = Migrator::<Postgres>::new(dialect);
```

Atomic migrations run inside a transaction. MySQL and MariaDB commit implicitly on
most DDL statements, so on those backends a failed atomic migration cannot be rolled
back as a unit.

## Custom SQL

A migrator is configured by a `Dialect` value holding the table prefix, schema and advisory lock
options, plus the SQL built from them. `Migrator::default` uses the built in
`Dialect::postgres`, `Dialect::mysql` or `Dialect::sqlite`. The table prefix and schema are arguments of the `_with`
constructors, for example `Dialect::postgres_with(Some("prefix"), Some("schema"))`. Adjust a built in dialect with `set_lock_strategy` and `set_create_schema` where the database has it, and pass it to `Migrator::new`:

```rust
use sqlx::Postgres;
use sqlx_migrator::migrator::LockStrategy;
use sqlx_migrator::{Dialect, Migrator};

let dialect = Dialect::postgres().set_lock_strategy(LockStrategy::None).unwrap();
let migrator = Migrator::<Postgres>::new(dialect);
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
| `revert` | Revert the last applied migration by default; `--all`, `--app` / `--migration`, `--count` or `--last-batch` (everything the last run applied) choose more. Asks for confirmation unless `--force` is passed. |
| `redo` | Revert the last applied migration and apply it again, handy while developing a migration. |
| `list` | Show every migration with its status (applied, failed, pending, replaced or unknown), batch and execution time. |
| `repair` | Remove failed entries from the migration table after fixing the database by hand, so those migrations run again. Asks for confirmation unless `--force` is passed. |
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
