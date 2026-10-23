# Subcommands

```rust,ignore
use sqlx_migrator::cli::MigrationCommand;

MigrationCommand::parse_and_run(&mut *conn, &migrator).await?;
```

| Subcommand | Purpose |
| --- | --- |
| `apply` | Apply pending migrations. `--plan`, `--check`, `--fake`, `--app`, `--migration`, `--count`. |
| `revert` | Revert the last migration, or `--all`, `--count`, `--last-batch`, `--app`, `--migration`. |
| `redo` | Revert the last migration and apply it again. |
| `list` | Show migrations with status, batch and time. |
| `repair` | Remove failed rows after fixing the database. |
| `prune` | Remove rows of migrations no longer in the code. |
| `drop` | Drop the migration table once everything is reverted. |

`list`, `apply --plan` and `revert --plan` accept `--format json` with the `json` feature.
Prompts need a terminal, so pass `--force` in scripts.
