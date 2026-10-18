//! `prune` subcommand
#![expect(clippy::print_stdout, reason = "allow printing to stdout in cli")]
use std::collections::HashSet;

use clap::Parser;
use sqlx::Database;

use super::output::confirm;
use crate::error::Error;
use crate::migrator::{Migrator, SupportedDatabase};

#[derive(Parser, Debug)]
pub(super) struct Prune {
    /// Prune without asking for confirmation
    #[arg(long)]
    pub(super) force: bool,
}

impl Prune {
    pub(super) async fn run<DB>(
        &self,
        connection: &mut <DB as Database>::Connection,
        migrator: &Migrator<DB>,
    ) -> Result<(), Error>
    where
        DB: SupportedDatabase,
    {
        let applied_migrations = migrator.fetch_applied_migration_from_db(connection).await?;
        let known_keys = migrator
            .migrations()
            .iter()
            .map(|migration| (migration.app(), migration.name()))
            .collect::<HashSet<_>>();
        let unknown = applied_migrations
            .iter()
            .filter(|row| !known_keys.contains(&(row.app(), row.name())))
            .map(|row| (row.app(), row.name()))
            .collect::<Vec<_>>();
        if unknown.is_empty() {
            println!("No unknown migration to prune");
            return Ok(());
        }
        if !self.force
            && !confirm(
                &format!(
                    "Do you want to prune {} unknown migrations from the migration table?",
                    unknown.len()
                ),
                &unknown,
            )?
        {
            println!("Aborted pruning migrations");
            return Ok(());
        }
        let pruned_count = migrator.prune(connection).await?;
        println!("Pruned {pruned_count} unknown migrations from migration table");
        Ok(())
    }
}
