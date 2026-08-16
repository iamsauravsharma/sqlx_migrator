//! Module for creating and running cli with help of migrator
//!
//! CLI Command can directly used or extended
//!
//! For direct usage you can run `parse_and_run` function for `MigrationCommand`
//!
//! OR
//!
//! If you want to extend your own clap based cli then you can add migrator to
//! sub command enum and then run migrator
//! ```rust,no_run
//! #[derive(clap::Parser)]
//! struct Cli {
//!     #[command(subcommand)]
//!     sub_command: CliSubcommand,
//! }
//!
//! #[derive(clap::Subcommand)]
//! enum CliSubcommand {
//!     #[command()]
//!     Migrator(sqlx_migrator::cli::MigrationCommand),
//! }
//! ```
#![expect(clippy::print_stdout, reason = "allow printing to stdout in cli")]
use std::collections::HashSet;
use std::io::{IsTerminal as _, Write as _};

use clap::{Parser, Subcommand};
use sqlx::Database;

use crate::error::Error;
use crate::migrator::{Migrate, Plan};

/// Migration command for performing rust based sqlx migrations
#[derive(Parser, Debug)]
pub struct MigrationCommand {
    #[command(subcommand)]
    sub_command: SubCommand,
}

impl MigrationCommand {
    /// Parse [`MigrationCommand`] and run migration command line interface
    ///
    /// # Errors
    /// If migration command fails to complete and raise some issue
    pub async fn parse_and_run<DB, M>(
        connection: &mut <DB as Database>::Connection,
        migrator: &M,
    ) -> Result<(), Error>
    where
        DB: Database,
        M: Migrate<DB> + ?Sized,
    {
        let migration_command = Self::parse();
        migration_command.run(connection, migrator).await
    }

    /// Run migration command line interface
    ///
    /// # Errors
    /// If migration command fails to complete and raise some issue
    pub async fn run<DB, M>(
        &self,
        connection: &mut <DB as Database>::Connection,
        migrator: &M,
    ) -> Result<(), Error>
    where
        DB: Database,
        M: Migrate<DB> + ?Sized,
    {
        self.sub_command
            .handle_subcommand(migrator, connection)
            .await?;
        Ok(())
    }
}

#[derive(Subcommand, Debug)]
enum SubCommand {
    /// Apply migrations
    Apply(Apply),
    /// Drop migration information table. Needs all migrations to be
    /// reverted else raises error
    Drop,
    /// List migrations along with their status and time applied if migrations
    /// is already applied
    List,
    /// Revert migrations
    Revert(Revert),
}

impl SubCommand {
    async fn handle_subcommand<DB, M>(
        &self,
        migrator: &M,
        connection: &mut <DB as Database>::Connection,
    ) -> Result<(), Error>
    where
        DB: Database,
        M: Migrate<DB> + ?Sized,
    {
        match self {
            SubCommand::Apply(apply) => apply.run(connection, migrator).await?,
            SubCommand::Drop => drop_migrations(connection, migrator).await?,
            SubCommand::List => list_migrations(connection, migrator).await?,
            SubCommand::Revert(revert) => revert.run(connection, migrator).await?,
        }
        Ok(())
    }
}

async fn drop_migrations<DB, M>(
    connection: &mut <DB as Database>::Connection,
    migrator: &M,
) -> Result<(), Error>
where
    DB: Database,
    M: Migrate<DB> + ?Sized,
{
    migrator.ensure_migration_table_exists(connection).await?;
    // hold lock for dropping migration table to avoid race condition where another
    // process is applying migration and we are dropping the table
    migrator.lock(connection).await?;
    let result = async {
        if !migrator
            .fetch_applied_migration_from_db(connection)
            .await?
            .is_empty()
        {
            return Err(Error::AppliedMigrationExists);
        }
        migrator.drop_migration_table_if_exists(connection).await?;
        Ok(())
    }
    .await;
    let unlock_result = migrator.unlock(connection).await;
    result.and(unlock_result)?;
    println!("Dropped migrations table");
    Ok(())
}

async fn list_migrations<DB, M>(
    connection: &mut <DB as Database>::Connection,
    migrator: &M,
) -> Result<(), Error>
where
    DB: Database,
    M: Migrate<DB> + ?Sized,
{
    migrator.ensure_migration_table_exists(connection).await?;
    let applied_migrations = migrator.fetch_applied_migration_from_db(connection).await?;
    let status_list = migrator.status(&applied_migrations)?;

    let apply_plan_keys = if status_list.is_empty() {
        HashSet::new()
    } else {
        migrator
            .generate_migration_plan(Some(&Plan::apply_all()), &applied_migrations)?
            .iter()
            .map(|migration| (migration.app(), migration.name()))
            .collect::<HashSet<_>>()
    };

    let widths = [5, 10, 50, 10, 40];
    let full_width = widths.iter().sum::<usize>() + widths.len() * 3;

    let first_width = widths[0];
    let second_width = widths[1];
    let third_width = widths[2];
    let fourth_width = widths[3];
    let fifth_width = widths[4];

    println!(
        "{:^first_width$} | {:^second_width$} | {:^third_width$} | {:^fourth_width$} | \
         {:^fifth_width$}",
        "ID", "App", "Name", "Status", "Applied time"
    );

    println!("{:^full_width$}", "-".repeat(full_width));
    let known_keys = status_list
        .iter()
        .map(|(migration, _)| (migration.app(), migration.name()))
        .collect::<HashSet<_>>();
    for (migration, applied) in &status_list {
        let mut id = String::from("N/A");
        let mut status = "\u{2717}";
        let mut applied_time = String::from("N/A");

        if let Some(sqlx_migration) = applied {
            id = sqlx_migration.id().to_string();
            status = "\u{2713}";
            applied_time = sqlx_migration.applied_time().to_string();
        } else if !apply_plan_keys.contains(&(migration.app(), migration.name())) {
            status = "\u{2194}";
        }

        println!(
            "{:^first_width$} | {:^second_width$} | {:^third_width$} | {:^fourth_width$} | \
             {:^fifth_width$}",
            id,
            migration.app(),
            migration.name(),
            status,
            applied_time
        );
    }
    // show applied migrations which no longer exist in the migration list
    // (e.g. migration was removed from the codebase) so drift is visible.
    // Such entries can be removed using apply --prune
    for row in &applied_migrations {
        if !known_keys.contains(&(row.app(), row.name())) {
            println!(
                "{:^first_width$} | {:^second_width$} | {:^third_width$} | {:^fourth_width$} | \
                 {:^fifth_width$}",
                row.id(),
                row.app(),
                row.name(),
                "?",
                row.applied_time()
            );
        }
    }
    Ok(())
}

#[derive(Parser, Debug)]
#[expect(clippy::struct_excessive_bools)]
struct Apply {
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
    /// Apply migration till provided migration. Requires app options to be
    /// present
    #[arg(long, requires = "app")]
    migration: Option<String>,
    /// Show plan
    #[arg(long, conflicts_with_all = ["fake", "force"])]
    plan: bool,
    /// Prune applied migration entries from the migration table which no
    /// longer exist in the migration list after applying migrations
    #[arg(long, conflicts_with_all = ["check", "plan"])]
    prune: bool,
}
impl Apply {
    async fn run<DB, M>(
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
            if migrations.is_empty() {
                println!("No migration exists for applying");
            } else {
                let first_width = 10;
                let second_width = 50;
                let full_width = first_width + second_width + 3;
                println!("{:^first_width$} | {:^second_width$}", "App", "Name");
                println!("{:^full_width$}", "-".repeat(full_width));
                for migration in migrations {
                    println!(
                        "{:^first_width$} | {:^second_width$}",
                        migration.app(),
                        migration.name(),
                    );
                }
            }
        } else {
            let destructible_migrations = migrations
                .iter()
                .filter(|m| m.operations().iter().any(|o| o.is_destructible()))
                .collect::<Vec<_>>();
            if !self.force && !destructible_migrations.is_empty() && !self.fake {
                if !std::io::stdin().is_terminal() {
                    return Err(Error::ConfirmationRequired);
                }
                let mut input = String::new();
                println!(
                    "Do you want to apply {} destructible migrations? (y/N)",
                    destructible_migrations.len()
                );
                for (position, migration) in destructible_migrations.iter().enumerate() {
                    println!(
                        "{}. {} : {}",
                        position + 1,
                        migration.app(),
                        migration.name()
                    );
                }
                std::io::stdout().flush()?;
                std::io::stdin().read_line(&mut input)?;
                let input_trimmed = input.trim().to_ascii_lowercase();
                // If answer is not y or yes then return
                if !["y", "yes"].contains(&input_trimmed.as_str()) {
                    println!("Aborted applying migrations");
                    return Ok(());
                }
            }
            migrator.run(connection, &plan).await?;
            println!("Successfully applied migrations according to plan");
            if self.prune {
                let pruned_count = migrator.prune(connection).await?;
                println!("Pruned {pruned_count} unknown migrations from migration table");
            }
        }
        Ok(())
    }
}

#[derive(Parser, Debug)]
#[expect(clippy::struct_excessive_bools)]
struct Revert {
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
    /// Make migration reverted without running revert operation
    #[arg(long)]
    fake: bool,
    /// Force run revert operation without asking question
    #[arg(long)]
    force: bool,
    /// Revert migration till provided migration. Requires app options to be
    /// present
    #[arg(long, requires = "app")]
    migration: Option<String>,
    /// Show plan
    #[arg(long, conflicts_with_all = ["fake", "force"])]
    plan: bool,
}
impl Revert {
    async fn run<DB, M>(
        &self,
        connection: &mut <DB as Database>::Connection,
        migrator: &M,
    ) -> Result<(), Error>
    where
        DB: Database,
        M: Migrate<DB> + ?Sized,
    {
        let plan = if let Some(count) = self.count {
            Plan::revert_count(count)
        } else if let Some(app) = &self.app {
            Plan::revert_name(app, self.migration.as_deref())
        } else if self.all {
            Plan::revert_all()
        } else {
            Plan::revert_count(1)
        }
        .fake(self.fake);
        migrator.ensure_migration_table_exists(connection).await?;
        let applied_rows = migrator.fetch_applied_migration_from_db(connection).await?;
        let revert_migrations = migrator.generate_migration_plan(Some(&plan), &applied_rows)?;

        if self.plan {
            if revert_migrations.is_empty() {
                println!("No migration exists for reverting");
            } else {
                let first_width = 10;
                let second_width = 50;
                let full_width = first_width + second_width + 3;
                println!("{:^first_width$} | {:^second_width$}", "App", "Name");
                println!("{:^full_width$}", "-".repeat(full_width));
                for migration in revert_migrations {
                    println!(
                        "{:^first_width$} | {:^second_width$}",
                        migration.app(),
                        migration.name(),
                    );
                }
            }
        } else {
            if !self.force && !revert_migrations.is_empty() && !self.fake {
                if !std::io::stdin().is_terminal() {
                    return Err(Error::ConfirmationRequired);
                }
                let mut input = String::new();
                println!(
                    "Do you want to revert {} migrations? (y/N)",
                    revert_migrations.len()
                );
                for (position, migration) in revert_migrations.iter().enumerate() {
                    println!(
                        "{}. {} : {}",
                        position + 1,
                        migration.app(),
                        migration.name()
                    );
                }
                std::io::stdout().flush()?;
                std::io::stdin().read_line(&mut input)?;
                let input_trimmed = input.trim().to_ascii_lowercase();
                // If answer is not y or yes then return
                if !["y", "yes"].contains(&input_trimmed.as_str()) {
                    println!("Aborted reverting migrations");
                    return Ok(());
                }
            }
            migrator.run(connection, &plan).await?;
            println!("Successfully reverted migrations according to plan");
        }
        Ok(())
    }
}
