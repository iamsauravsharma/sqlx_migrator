//! `repair` subcommand
#![expect(clippy::print_stdout, reason = "allow printing to stdout in cli")]
use clap::Parser;
use sqlx::Database;

use super::output::confirm;
use crate::error::Error;
use crate::migrator::{Migrator, SupportedDatabase};

#[derive(Parser, Debug)]
pub(super) struct Repair {
    /// Repair without asking for confirmation
    #[arg(long)]
    pub(super) force: bool,
}

impl Repair {
    pub(super) async fn run<DB>(
        &self,
        connection: &mut <DB as Database>::Connection,
        migrator: &Migrator<DB>,
    ) -> Result<(), Error>
    where
        DB: SupportedDatabase,
    {
        let applied_migrations = migrator.fetch_applied_migration_from_db(connection).await?;
        let failed = applied_migrations
            .iter()
            .filter(|row| !row.success())
            .map(|row| (row.app(), row.name()))
            .collect::<Vec<_>>();
        if failed.is_empty() {
            println!("No failed migration to repair");
            return Ok(());
        }
        if !self.force
            && !confirm(
                &format!(
                    "Failed migrations changed the database only partly. Fix the database by hand \
                     first. Do you want to remove {} failed migrations from the migration table \
                     so they run again?",
                    failed.len()
                ),
                &failed,
            )?
        {
            println!("Aborted repairing migrations");
            return Ok(());
        }
        let repaired_count = migrator.repair(connection).await?;
        println!("Removed {repaired_count} failed migrations from migration table");
        Ok(())
    }
}
