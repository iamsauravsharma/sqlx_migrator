//! Module for creating and running cli with help of migrator
use std::ops::Not;

use clap::{Parser, Subcommand};
use sqlx::Pool;

use crate::error::Error;
use crate::migrator::{Migrate, Plan};

#[derive(Parser, Debug)]
struct Args {
    #[command(subcommand)]
    sub_command: SubCommand,
}

#[derive(Subcommand, Debug)]
/// Subcommand for sqlx migrator cli
pub enum SubCommand {
    /// Apply migrations
    #[command()]
    Apply(Apply),
    /// Drop sqlx information migrations table. Needs all migrations to be
    /// reverted
    #[command()]
    Drop,
    /// List migrations along with their status
    #[command()]
    List,
    /// Revert migrations
    #[command()]
    Revert(Revert),
}

impl SubCommand {
    /// Handle all subcommand operations
    ///
    /// # Errors
    ///  If any subcommand operations fail running
    pub async fn handle_subcommand<DB>(
        &self,
        migrator: Box<dyn Migrate<DB>>,
        pool: &Pool<DB>,
    ) -> Result<(), Error>
    where
        DB: sqlx::Database,
    {
        let mut connection = pool.acquire().await?;
        match self {
            SubCommand::Apply(apply) => apply.run(migrator, &mut connection).await?,
            SubCommand::Drop => drop_migrations(migrator, &mut connection).await?,
            SubCommand::List => list_migrations(migrator, &mut connection).await?,
            SubCommand::Revert(revert) => revert.run(migrator, &mut connection).await?,
        }
        connection.close().await?;
        Ok(())
    }
}

async fn drop_migrations<DB>(
    migrator: Box<dyn Migrate<DB>>,
    connection: &mut <DB as sqlx::Database>::Connection,
) -> Result<(), Error>
where
    DB: sqlx::Database,
{
    migrator.ensure_migration_table_exists(connection).await?;
    if migrator
        .fetch_applied_migration_from_db(connection)
        .await?
        .is_empty()
        .not()
    {
        return Err(Error::AppliedMigrationExists);
    }

    migrator.drop_migration_table_if_exists(connection).await?;
    println!("Dropped migrations table");
    Ok(())
}

async fn list_migrations<DB>(
    migrator: Box<dyn Migrate<DB>>,
    connection: &mut <DB as sqlx::Database>::Connection,
) -> Result<(), Error>
where
    DB: sqlx::Database,
{
    migrator.ensure_migration_table_exists(connection).await?;
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
    let plan = Plan::new(crate::migrator::PlanType::All, None, None)?;
    for migration in migrator.generate_migration_plan(plan, connection).await? {
        let applied_migration_info = applied_migrations
            .iter()
            .find(|&applied_migration| applied_migration == migration);

        let mut id = String::from("N/A");
        let mut status = "\u{2717}";
        let mut applied_time = String::from("N/A");

        if let Some(sqlx_migration) = applied_migration_info {
            id = sqlx_migration.id().to_string();
            status = "\u{2713}";
            applied_time = sqlx_migration.applied_time().to_string();
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
/// CLI struct for apply subcommand
pub struct Apply {
    /// Apply migration till all app migration are applied
    #[arg(long)]
    app: Option<String>,
    /// Check for pending migration
    #[arg(long)]
    check: bool,
    /// Make migration applied without applying
    #[arg(long)]
    fake: bool,
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
        migrator: Box<dyn Migrate<DB>>,
        connection: &mut <DB as sqlx::Database>::Connection,
    ) -> Result<(), Error>
    where
        DB: sqlx::Database,
    {
        migrator.lock(connection).await?;
        let plan = Plan::new(
            crate::migrator::PlanType::Apply,
            self.app.clone(),
            self.migration.clone(),
        )?;
        let migrations = migrator.generate_migration_plan(plan, connection).await?;
        if self.check && !migrations.is_empty() {
            return Err(Error::PendingMigrationPresent);
        }
        if self.plan {
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
        } else if self.fake {
            for migration in migrations {
                migrator
                    .add_migration_to_db_table(migration, connection)
                    .await?;
            }
        } else {
            for migration in migrations {
                migrator.apply_migration(migration, connection).await?;
                println!("Applied {} : {}", migration.app(), migration.name());
            }
        }
        migrator.unlock(connection).await?;
        Ok(())
    }
}

#[derive(Parser, Debug)]
/// CLI struct for revert subcommand
pub struct Revert {
    /// Revert all migration
    #[arg(long)]
    all: bool,
    /// Revert migration till all app migration are reverted
    #[arg(long)]
    app: Option<String>,
    /// Make migration reverted without reverting
    #[arg(long)]
    fake: bool,
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
        migrator: Box<dyn Migrate<DB>>,
        connection: &mut <DB as sqlx::Database>::Connection,
    ) -> Result<(), Error>
    where
        DB: sqlx::Database,
    {
        migrator.lock(connection).await?;
        let app_is_some = self.app.is_some();
        let plan = Plan::new(
            crate::migrator::PlanType::Revert,
            self.app.clone(),
            self.migration.clone(),
        )?;
        let revert_plan = migrator.generate_migration_plan(plan, connection).await?;
        let revert_migrations;
        if self.all || app_is_some {
            revert_migrations = revert_plan;
        } else if let Some(latest_migration) = revert_plan.first() {
            revert_migrations = vec![latest_migration];
        } else {
            revert_migrations = vec![];
        }
        if self.plan {
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
        } else if self.fake {
            for migration in revert_migrations {
                migrator
                    .delete_migration_from_db_table(migration, connection)
                    .await?;
            }
        } else {
            for migration in revert_migrations {
                migrator.revert_migration(migration, connection).await?;
                println!("Reverted {} : {}", migration.app(), migration.name());
            }
        }
        migrator.unlock(connection).await?;
        Ok(())
    }
}

/// Run full cli by parsing args with help of migrator. If you only need to add
/// subcommand to your app than use `SubCommand` enum `handle_subcommand`
/// function
///
/// # Errors
/// When command fails to run
pub async fn run<DB>(migrator: Box<dyn Migrate<DB>>, pool: &Pool<DB>) -> Result<(), Error>
where
    DB: sqlx::Database,
{
    let args = Args::parse();
    args.sub_command.handle_subcommand(migrator, pool).await?;
    Ok(())
}
