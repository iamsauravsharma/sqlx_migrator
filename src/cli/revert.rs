//! `revert` subcommand
#![expect(clippy::print_stdout, reason = "allow printing to stdout in cli")]
use clap::Parser;
use sqlx::Database;

use super::output::{OutputFormat, confirm, render_plan};
use crate::error::Error;
use crate::migrator::{Migrator, Plan, SupportedDatabase};

#[derive(Parser, Debug)]
#[expect(clippy::struct_excessive_bools)]
pub(super) struct Revert {
    /// Revert all migration. Conflicts with app args
    #[arg(long, conflicts_with = "app")]
    all: bool,
    /// Revert migration till app migrations is reverted. If it is present
    /// alongside migration options then only till migration is reverted
    #[arg(long)]
    app: Option<String>,
    /// Number of migration to revert. Conflicts with all and app args
    #[arg(long, conflicts_with_all = ["all", "app"])]
    count: Option<usize>,
    /// Revert the migrations applied by the last run, which share a batch.
    /// Conflicts with all, app and count args
    #[arg(long, conflicts_with_all = ["all", "app", "count"])]
    last_batch: bool,
    /// Make migration reverted without running revert operation
    #[arg(long)]
    fake: bool,
    /// Force run revert operation without asking question
    #[arg(long)]
    force: bool,
    /// Output format of the plan. Requires plan option to be present
    #[arg(long, value_enum, default_value_t, requires = "plan")]
    format: OutputFormat,
    /// Revert migration till provided migration. Requires app options to be
    /// present
    #[arg(long, requires = "app")]
    migration: Option<String>,
    /// Show plan
    #[arg(long, conflicts_with_all = ["fake", "force"])]
    plan: bool,
}

impl Revert {
    pub(super) async fn run<DB>(
        &self,
        connection: &mut <DB as Database>::Connection,
        migrator: &Migrator<DB>,
    ) -> Result<(), Error>
    where
        DB: SupportedDatabase,
    {
        let mut revert_last = false;
        let plan = if let Some(count) = self.count {
            Plan::revert_count(count)
        } else if let Some(app) = &self.app {
            Plan::revert_name(app, self.migration.as_deref())
        } else if self.all {
            Plan::revert_all()
        } else if self.last_batch {
            Plan::revert_last_batch()
        } else {
            revert_last = true;
            Plan::revert_count(1)
        }
        .fake(self.fake);
        let applied_rows = migrator.fetch_applied_migration_from_db(connection).await?;
        // reverting the last migration when nothing is applied is not an
        // error, unlike an explicit count which is larger than what is applied
        let nothing_applied = revert_last
            && migrator
                .generate_migration_plan(Some(&Plan::revert_all()), &applied_rows)?
                .is_empty();
        let revert_migrations = if nothing_applied {
            vec![]
        } else {
            migrator.generate_migration_plan(Some(&plan), &applied_rows)?
        };
        if self.plan {
            println!(
                "{}",
                render_plan(
                    revert_migrations.iter().map(AsRef::as_ref),
                    self.format,
                    "No migration exists for reverting"
                )?
            );
            return Ok(());
        }
        if revert_migrations.is_empty() {
            println!("No migration exists for reverting");
            return Ok(());
        }
        let items = revert_migrations
            .iter()
            .map(|m| (m.app(), m.name()))
            .collect::<Vec<_>>();
        if !self.force
            && !self.fake
            && !items.is_empty()
            && !confirm(
                &format!("Do you want to revert {} migrations?", items.len()),
                &items,
            )?
        {
            println!("Aborted reverting migrations");
            return Ok(());
        }
        let report = migrator.run(connection, &plan).await?;
        println!(
            "Successfully reverted {} migrations according to plan in {}ms",
            report.len(),
            report.total_elapsed().as_millis()
        );
        Ok(())
    }
}
