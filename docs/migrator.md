## Migration table

Applied migrations go in `_sqlx_migrator_migrations` (optional prefix and schema via
`Dialect::postgres_with` and friends). Each row has `app`, `name`, `applied_time`,
`success`, `execution_time_ns` and `batch` (migrations applied by one run share it).
The migrator creates and upgrades the table itself.

A non atomic migration is recorded as failed before it runs. If it fails, runs are
refused with `Error::FailedMigration` until you fix the database and run `repair`.
Atomic migrations are recorded in their own transaction and never leave a failed row.

## Locking

Choose how two processes are kept from migrating at once with
`Dialect::set_lock_strategy`:

| `LockStrategy` | |
| --- | --- |
| `Advisory` | Connection lock. Default on Postgres and MySQL. |
| `None` | No lock. Default on SQLite. |
| `Row` | A row in `_sqlx_migrator_lock`, works everywhere without a session. |
| `TransactionAdvisory` | Postgres. One transaction runs the whole plan, so every migration must be atomic. |
| `TableShareUpdateExclusive` | Postgres. Like the above, plus a table lock that allows reads. |
| `TableAccessExclusive` | Postgres. Like the above, plus a table lock that blocks reads. |

```rust,ignore
use sqlx::Postgres;
use sqlx_migrator::migrator::LockStrategy;
use sqlx_migrator::{Dialect, Migrator};

let dialect = Dialect::postgres()
    .set_lock_strategy(LockStrategy::Row)
    .unwrap();
let migrator = Migrator::<Postgres>::new(dialect);
```

MySQL commits most DDL implicitly, so a failed atomic migration cannot be rolled back there,
and neither can a grouped plan (`Plan::grouped`).
