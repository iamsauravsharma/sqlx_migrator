//! `list` subcommand
#![expect(clippy::print_stdout, reason = "allow printing to stdout in cli")]
use std::collections::HashSet;

use clap::Parser;
#[cfg(feature = "json")]
use serde::Serialize;
use sqlx::Database;

use super::output::{OutputFormat, render_table};
use crate::error::Error;
use crate::migration::AppliedMigrationSqlRow;
use crate::migrator::{Migrator, Plan, SupportedDatabase};

/// Status of a migration as shown by `list`
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "json", derive(Serialize), serde(rename_all = "lowercase"))]
pub(super) enum ListStatus {
    /// Applied and recorded in the migration table
    Applied,
    /// Started running and never finished, so the database may be partly
    /// migrated. Fix it by hand and run the `repair` command
    Failed,
    /// Known migration which is not applied yet
    Pending,
    /// Known migration which will not be applied because a migration replacing
    /// it is applied or will be applied instead
    Replaced,
    /// Recorded in the migration table but no longer present in the migration
    /// list. Such entries can be removed with the `prune` command
    Unknown,
}

impl ListStatus {
    fn symbol(self) -> &'static str {
        match self {
            ListStatus::Applied => "\u{2713}",
            ListStatus::Failed => "!",
            ListStatus::Pending => "\u{2717}",
            ListStatus::Replaced => "\u{2194}",
            ListStatus::Unknown => "?",
        }
    }
}

/// A row of the `list` command
#[cfg_attr(feature = "json", derive(Serialize))]
pub(super) struct ListEntry<'a> {
    pub(super) id: Option<i32>,
    pub(super) app: &'a str,
    pub(super) name: &'a str,
    pub(super) status: ListStatus,
    pub(super) applied_time: Option<&'a str>,
    pub(super) execution_time_ns: Option<i64>,
    pub(super) batch: Option<i32>,
}

/// Build the list entries: every known migration in apply order followed by
/// applied rows which are not known any more.
pub(super) fn list_entries<'a, DB>(
    migrator: &'a Migrator<DB>,
    applied_migrations: &'a [AppliedMigrationSqlRow],
) -> Result<Vec<ListEntry<'a>>, Error>
where
    DB: SupportedDatabase,
{
    let status_list = migrator.status(applied_migrations)?;
    let apply_plan_keys = if status_list.is_empty() {
        HashSet::new()
    } else {
        migrator
            .generate_migration_plan(Some(&Plan::apply_all()), applied_migrations)?
            .iter()
            .map(|migration| (migration.app(), migration.name()))
            .collect::<HashSet<_>>()
    };
    let known_keys = status_list
        .iter()
        .map(|(migration, _)| (migration.app(), migration.name()))
        .collect::<HashSet<_>>();
    let mut entries = status_list
        .iter()
        .map(|(migration, applied)| {
            let key = (migration.app(), migration.name());
            match applied {
                Some(row) => {
                    ListEntry {
                        id: Some(row.id()),
                        app: migration.app(),
                        name: migration.name(),
                        status: if row.success() {
                            ListStatus::Applied
                        } else {
                            ListStatus::Failed
                        },
                        applied_time: Some(row.applied_time()),
                        execution_time_ns: Some(row.execution_time_ns()),
                        batch: Some(row.batch()),
                    }
                }
                None => {
                    ListEntry {
                        id: None,
                        app: migration.app(),
                        name: migration.name(),
                        status: if apply_plan_keys.contains(&key) {
                            ListStatus::Pending
                        } else {
                            ListStatus::Replaced
                        },
                        applied_time: None,
                        execution_time_ns: None,
                        batch: None,
                    }
                }
            }
        })
        .collect::<Vec<_>>();
    // applied migrations which no longer exist in the migration list (e.g.
    // migration was removed from the codebase) so drift is visible
    entries.extend(
        applied_migrations
            .iter()
            .filter(|row| !known_keys.contains(&(row.app(), row.name())))
            .map(|row| {
                ListEntry {
                    id: Some(row.id()),
                    app: row.app(),
                    name: row.name(),
                    status: ListStatus::Unknown,
                    applied_time: Some(row.applied_time()),
                    execution_time_ns: Some(row.execution_time_ns()),
                    batch: Some(row.batch()),
                }
            }),
    );
    Ok(entries)
}

/// Time kept in nanoseconds as milliseconds with a fraction, for example
/// `1.234` for 1234567 nanoseconds
pub(super) fn milliseconds(nanoseconds: i64) -> String {
    format!(
        "{}.{:03}",
        nanoseconds / 1_000_000,
        nanoseconds % 1_000_000 / 1_000
    )
}

/// Render list entries in the requested format
#[cfg_attr(
    not(feature = "json"),
    expect(
        clippy::unnecessary_wraps,
        reason = "only JSON serialization can fail; keep one signature"
    )
)]
pub(super) fn render_list(
    entries: &[ListEntry<'_>],
    format: OutputFormat,
) -> Result<String, Error> {
    match format {
        #[cfg(feature = "json")]
        OutputFormat::Json => Ok(serde_json::to_string_pretty(entries)?),
        OutputFormat::Table => {
            let rows = entries
                .iter()
                .map(|entry| {
                    vec![
                        entry
                            .id
                            .map_or_else(|| "N/A".to_string(), |id| id.to_string()),
                        entry.app.to_string(),
                        entry.name.to_string(),
                        entry.status.symbol().to_string(),
                        entry.applied_time.unwrap_or("N/A").to_string(),
                        entry
                            .batch
                            .map_or_else(|| "N/A".to_string(), |batch| batch.to_string()),
                        entry
                            .execution_time_ns
                            .map_or_else(|| "N/A".to_string(), milliseconds),
                    ]
                })
                .collect::<Vec<_>>();
            Ok(render_table(
                &[
                    "ID",
                    "App",
                    "Name",
                    "Status",
                    "Applied time",
                    "Batch",
                    "Time (ms)",
                ],
                &rows,
            ))
        }
    }
}

#[derive(Parser, Debug)]
pub(super) struct List {
    /// Output format
    #[arg(long, value_enum, default_value_t)]
    pub(super) format: OutputFormat,
}

impl List {
    pub(super) async fn run<DB>(
        &self,
        connection: &mut <DB as Database>::Connection,
        migrator: &Migrator<DB>,
    ) -> Result<(), Error>
    where
        DB: SupportedDatabase,
    {
        let applied_migrations = migrator.fetch_applied_migration_from_db(connection).await?;
        let entries = list_entries(migrator, &applied_migrations)?;
        println!("{}", render_list(&entries, self.format)?);
        Ok(())
    }
}
