# SQLX migrator

A Rust library for writing SQLX migrations using Rust instead of SQL.

|                    License                     |              Crates Version               |                 Docs                 |
| :--------------------------------------------: | :---------------------------------------: | :----------------------------------: |
| [![License: MIT][license_badge]][license_link] | [![Crate][cratesio_badge]][cratesio_link] | [![Docs][docsrs_badge]][docsrs_link] |

Supported databases: PostgreSQL, SQLite and MySQL.

## Features

- Migrations as Rust values with parents, `replaces` and `run_before`.
- Operations as code, SQL strings or embedded SQL files.
- CLI (`apply`, `revert`, `redo`, `list`, `repair`, `prune`) and `RunReport`.
- Locking, failed migration tracking and hooks.
- Sync from other migration tools.

## Installation

Enable `postgres`, `mysql` or `sqlite` (and `json` for CLI JSON output):

```toml
sqlx_migrator = { version = "0.20.0", features = ["postgres"] }
```

## Quick start

```rust
use sqlx_migrator::migrator::{Migrator, Plan};
use sqlx_migrator::vec_box;

struct CreateSample;

// an operation is a `(up, down)` pair of SQL strings, or implement `Operation`
// for code
sqlx_migrator::postgres_migration!(
    CreateSample,
    "main",
    "create_sample",
    vec_box![],
    vec_box![(
        "CREATE TABLE sample (id INTEGER PRIMARY KEY, name TEXT)",
        "DROP TABLE sample"
    )]
);

#[tokio::main]
async fn main() {
    let pool = sqlx::PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();
    let mut conn = pool.acquire().await.unwrap();

    let mut migrator = Migrator::default();
    migrator.add_migration(Box::new(CreateSample)).unwrap();
    migrator.run(&mut *conn, &Plan::apply_all()).await.unwrap();
}
```

## Documentation

See the [crate docs][docsrs_link] for the guide, locking, CLI, hooks and sync.

[license_badge]: https://img.shields.io/github/license/iamsauravsharma/sqlx_migrator.svg?style=for-the-badge
[license_link]: LICENSE
[cratesio_badge]: https://img.shields.io/crates/v/sqlx_migrator.svg?style=for-the-badge
[cratesio_link]: https://crates.io/crates/sqlx_migrator
[docsrs_badge]: https://img.shields.io/docsrs/sqlx_migrator/latest?style=for-the-badge
[docsrs_link]: https://docs.rs/sqlx_migrator
