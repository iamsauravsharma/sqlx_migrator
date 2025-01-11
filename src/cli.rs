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
use std::io::Write;

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
    pub async fn parse_and_run<DB>(
        connection: &mut <DB as Database>::Connection,
        migrator: Box<dyn Migrate<DB>>,
    ) -> Result<(), Error>
    where
        DB: Database,
    {
        let migration_command = Self::parse();
        migration_command.run(connection, migrator).await
    }

    /// Run migration command line interface
    ///
    /// # Errors
    /// If migration command fails to complete and raise some issue
    pub async fn run<DB>(
        &self,
        connection: &mut <DB as Database>::Connection,
        migrator: Box<dyn Migrate<DB>>,
    ) -> Result<(), Error>
    where
        DB: Database,
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
    #[command()]
    Apply(Apply),
    /// Drop migration information table. Needs all migrations to be
    /// reverted else raises error
    #[command()]
    Drop,
    /// List migrations along with their status and time applied if migrations
    /// is already applied
    #[command()]
    List,
    /// Revert migrations
    #[command()]
    Revert(Revert),
}

impl SubCommand {
    async fn handle_subcommand<DB>(
        &self,
        migrator: Box<dyn Migrate<DB>>,
        connection: &mut <DB as Database>::Connection,
    ) -> Result<(), Error>
    where
        DB: Database,
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

async fn drop_migrations<DB>(
    connection: &mut <DB as Database>::Connection,
    migrator: Box<dyn Migrate<DB>>,
) -> Result<(), Error>
where
    DB: Database,
{
    migrator.ensure_migration_table_exists(connection).await?;
    if !migrator
        .fetch_applied_migration_from_db(connection)
        .await?
        .is_empty()
    {
        return Err(Error::AppliedMigrationExists);
    }
    migrator.drop_migration_table_if_exists(connection).await?;
    println!("Dropped migrations table");
    Ok(())
}

async fn list_migrations<DB>(
    connection: &mut <DB as Database>::Connection,
    migrator: Box<dyn Migrate<DB>>,
) -> Result<(), Error>
where
    DB: Database,
{
    let migration_plan = migrator.generate_migration_plan(connection, None).await?;

    let apply_plan = migrator
        .generate_migration_plan(connection, Some(&Plan::apply_all()))
        .await?;
    let applied_migrations = migrator.fetch_applied_migration_from_db(connection).await?;

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
    for migration in migration_plan {
        let mut id = String::from("N/A");
        let mut status = "\u{2717}";
        let mut applied_time = String::from("N/A");

        let find_applied_migrations = applied_migrations
            .iter()
            .find(|&applied_migration| applied_migration == migration);

        if let Some(sqlx_migration) = find_applied_migrations {
            id = sqlx_migration.id().to_string();
            status = "\u{2713}";
            applied_time = sqlx_migration.applied_time().to_string();
        } else if !apply_plan
            .iter()
            .any(|&plan_migration| plan_migration == migration)
        {
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
    Ok(())
}

#[derive(Parser, Debug)]
#[expect(clippy::struct_excessive_bools)]
struct Apply {
    /// App name up to which migration needs to be applied. If migration option
    /// is also present than only till migration is applied
    #[arg(long)]
    app: Option<String>,
    /// Check for pending migration
    #[arg(long)]
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
    #[arg(long)]
    plan: bool,
}
impl Apply {
    async fn run<DB>(
        &self,
        connection: &mut <DB as Database>::Connection,
        migrator: Box<dyn Migrate<DB>>,
    ) -> Result<(), Error>
    where
        DB: Database,
    {
        let plan;
        if let Some(count) = self.count {
            plan = Plan::apply_count(count);
        } else if let Some(app) = &self.app {
            plan = Plan::apply_name(app, &self.migration);
        } else {
            plan = Plan::apply_all();
        };
        let plan = plan.fake(self.fake);
        let migrations = migrator
            .generate_migration_plan(connection, Some(&plan))
            .await?;
        if self.check && !migrations.is_empty() {
            return Err(Error::PendingMigrationPresent);
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
                let mut input = String::new();
                println!(
                    "Do you want to apply destructible migrations {} (y/N)",
                    destructible_migrations.len()
                );
                for (position, migration) in destructible_migrations.iter().enumerate() {
                    println!("{position}. {} : {}", migration.app(), migration.name());
                }
                std::io::stdout().flush()?;
                std::io::stdin().read_line(&mut input)?;
                let input_trimmed = input.trim().to_ascii_lowercase();
                // If answer is not y or yes then return
                if !["y", "yes"].contains(&input_trimmed.as_str()) {
                    return Ok(());
                }
            }
            migrator.run(connection, &plan).await?;
            println!("Successfully applied migrations according to plan");
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
    /// alongside migration options than only till migration is reverted
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
    #[arg(long)]
    plan: bool,
}
impl Revert {
    async fn run<DB>(
        &self,
        connection: &mut <DB as Database>::Connection,
        migrator: Box<dyn Migrate<DB>>,
    ) -> Result<(), Error>
    where
        DB: Database,
    {
        let plan;
        if let Some(count) = self.count {
            plan = Plan::revert_count(count);
        } else if let Some(app) = &self.app {
            plan = Plan::revert_name(app, &self.migration);
        } else if self.all {
            plan = Plan::revert_all();
        } else {
            plan = Plan::revert_count(1);
        };
        let plan = plan.fake(self.fake);
        let revert_migrations = migrator
            .generate_migration_plan(connection, Some(&plan))
            .await?;

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
                let mut input = String::new();
                println!(
                    "Do you want to revert {} migrations (y/N)",
                    revert_migrations.len()
                );
                for (position, migration) in revert_migrations.iter().enumerate() {
                    println!("{position}. {} : {}", migration.app(), migration.name());
                }
                std::io::stdout().flush()?;
                std::io::stdin().read_line(&mut input)?;
                let input_trimmed = input.trim().to_ascii_lowercase();
                // If answer is not y or yes then return
                if !["y", "yes"].contains(&input_trimmed.as_str()) {
                    return Ok(());
                }
            }
            migrator.run(connection, &plan).await?;
            println!("Successfully reverted migrations according to plan");
        }
        Ok(())
    }
}
