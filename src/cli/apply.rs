//! `apply` subcommand
#![expect(clippy::print_stdout, reason = "allow printing to stdout in cli")]
use clap::Parser;
use sqlx::Database;

use super::output::{OutputFormat, confirm, render_plan};
use crate::error::Error;
use crate::migrator::{Migrate, Plan};

#[derive(Parser, Debug)]
#[expect(clippy::struct_excessive_bools)]
pub(super) struct Apply {
    /// App name up to which migration needs to be applied. If migration option
    /// is also present then only till migration is applied
    #[arg(long)]
    app: Option<String>,
    /// Check for pending migration
    #[arg(long, conflicts_with_all = ["count", "fake", "force", "plan"])]
    check: bool,
    /// Number of migration to apply. Conflicts with app args
    #[arg(long, conflicts_with = "app")]
    count: Option<usize>,
    /// Make migration applied without running migration operations
    #[arg(long)]
    fake: bool,
    /// Force run apply operation without asking question if migration is
    /// destructible
    #[arg(long)]
    force: bool,
    /// Output format of the plan. Requires plan option to be present
    #[arg(long, value_enum, default_value_t, requires = "plan")]
    pub(super) format: OutputFormat,
    /// Apply migration till provided migration. Requires app options to be
    /// present
    #[arg(long, requires = "app")]
    migration: Option<String>,
    /// Show plan
    #[arg(long, conflicts_with_all = ["fake", "force"])]
    pub(super) plan: bool,
}

impl Apply {
    pub(super) async fn run<DB, M>(
        &self,
        connection: &mut <DB as Database>::Connection,
        migrator: &M,
    ) -> Result<(), Error>
    where
        DB: Database,
        M: Migrate<DB> + ?Sized,
    {
        let plan = if let Some(count) = self.count {
            Plan::apply_count(count)
        } else if let Some(app) = &self.app {
            Plan::apply_name(app, self.migration.as_deref())
        } else {
            Plan::apply_all()
        }
        .fake(self.fake);
        migrator.ensure_migration_table_exists(connection).await?;
        let applied_rows = migrator.fetch_applied_migration_from_db(connection).await?;
        let migrations = migrator.generate_migration_plan(Some(&plan), &applied_rows)?;
        if self.check {
            if !migrations.is_empty() {
                return Err(Error::PendingMigrationPresent);
            }
            println!("No pending migration to apply");
            return Ok(());
        }
        if self.plan {
            println!(
                "{}",
                render_plan(
                    migrations.iter().map(AsRef::as_ref),
                    self.format,
                    "No migration exists for applying"
                )?
            );
            return Ok(());
        }
        let destructible_migrations = migrations
            .iter()
            .filter(|m| m.operations().iter().any(|o| o.is_destructible()))
            .map(|m| (m.app(), m.name()))
            .collect::<Vec<_>>();
        if !self.force
            && !self.fake
            && !destructible_migrations.is_empty()
            && !confirm(
                &format!(
                    "Do you want to apply {} destructible migrations?",
                    destructible_migrations.len()
                ),
                &destructible_migrations,
            )?
        {
            println!("Aborted applying migrations");
            return Ok(());
        }
        let report = migrator.run(connection, &plan).await?;
        println!(
            "Successfully applied {} migrations according to plan in {}ms",
            report.len(),
            report.total_elapsed().as_millis()
        );
        Ok(())
    }
}
