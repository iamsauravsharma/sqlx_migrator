//! `redo` subcommand
#![expect(clippy::print_stdout, reason = "allow printing to stdout in cli")]
use clap::Parser;
use sqlx::Database;

use super::output::confirm;
use crate::error::Error;
use crate::migrator::{Migrator, Plan, SupportedDatabase};

#[derive(Parser, Debug)]
pub(super) struct Redo {
    /// Redo without asking for confirmation
    #[arg(long)]
    pub(super) force: bool,
}

impl Redo {
    pub(super) async fn run<DB>(
        &self,
        connection: &mut <DB as Database>::Connection,
        migrator: &Migrator<DB>,
    ) -> Result<(), Error>
    where
        DB: SupportedDatabase,
    {
        let applied_rows = migrator.fetch_applied_migration_from_db(connection).await?;
        // the revert all plan lists applied migrations last applied first,
        // so its head is the migration to redo
        let applied = migrator.generate_migration_plan(Some(&Plan::revert_all()), &applied_rows)?;
        let Some((app, name)) = applied
            .first()
            .map(|m| (m.app().to_string(), m.name().to_string()))
        else {
            println!("No applied migration to redo");
            return Ok(());
        };
        let revert_plan = Plan::revert_count(1);
        if !self.force
            && !confirm(
                "Do you want to revert and apply again the last applied migration?",
                &[(&app, &name)],
            )?
        {
            println!("Aborted redoing migration");
            return Ok(());
        }
        migrator.run(connection, &revert_plan).await?;
        let report = migrator
            .run(connection, &Plan::apply_name(&app, Some(&name)))
            .await?;
        println!(
            "Successfully redid {app} : {name} in {}ms",
            report.total_elapsed().as_millis()
        );
        Ok(())
    }
}
