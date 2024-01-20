//! Module for creating and running cli with help of migrator
use std::io::Write;
use std::ops::Not;

use clap::{Parser, Subcommand};
use sqlx::Pool;

use crate::error::Error;
use crate::migrator::{Migrate, Plan};

/// Migration command for performing rust based sqlx migrations
#[derive(Parser, Debug)]
pub struct MigrationCommand {
    #[command(subcommand)]
    sub_command: SubCommand,
}

impl MigrationCommand {
    /// Parse `MigrationCommand` and run migration command line interface
    ///
    /// # Errors
    /// If migration command fails to complete and raise some issue
    pub async fn parse_and_run<DB>(
        migrator: Box<dyn Migrate<DB>>,
        pool: &Pool<DB>,
    ) -> Result<(), Error>
    where
        DB: sqlx::Database,
    {
        let migration_command = Self::parse();
        migration_command.run(migrator, pool).await
    }

    /// Run migration command line interface
    ///
    /// # Errors
    /// If migration command fails to complete and raise some issue
    pub async fn run<DB>(
        &self,
        migrator: Box<dyn Migrate<DB>>,
        pool: &Pool<DB>,
    ) -> Result<(), Error>
    where
        DB: sqlx::Database,
    {
        self.sub_command.handle_subcommand(migrator, pool).await?;
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
#[allow(clippy::struct_excessive_bools)]
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
        let mut migrations = migrator.generate_migration_plan(plan, connection).await?;
        if let Some(count) = self.count {
            let actual_len = migrations.len();
            if count > actual_len {
                return Err(Error::CountGreater { actual_len, count });
            }
            migrations.truncate(count);
        }
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
            let destructible_migrations = migrations
                .iter()
                .filter(|m| m.operations().iter().any(|o| o.is_destructible()))
                .collect::<Vec<_>>();
            if !self.force && !destructible_migrations.is_empty() {
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
                let input = input.trim().to_ascii_lowercase();
                // If answer is not y or yes then return
                if !["y", "yes"].contains(&input.as_str()) {
                    return Ok(());
                }
            }
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
#[allow(clippy::struct_excessive_bools)]
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
        migrator: Box<dyn Migrate<DB>>,
        connection: &mut <DB as sqlx::Database>::Connection,
    ) -> Result<(), Error>
    where
        DB: sqlx::Database,
    {
        migrator.lock(connection).await?;
        let plan = Plan::new(
            crate::migrator::PlanType::Revert,
            self.app.clone(),
            self.migration.clone(),
        )?;
        let mut revert_migrations = migrator.generate_migration_plan(plan, connection).await?;
        if let Some(count) = self.count {
            let actual_len = revert_migrations.len();
            if count > actual_len {
                return Err(Error::CountGreater { actual_len, count });
            }
            revert_migrations.truncate(count);
        } else if !self.all && self.app.is_none() && !revert_migrations.is_empty() {
            revert_migrations.truncate(1);
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
            if !self.force {
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
                let input = input.trim().to_ascii_lowercase();
                // If answer is not y or yes then return
                if !["y", "yes"].contains(&input.as_str()) {
                    return Ok(());
                }
            }
            for migration in revert_migrations {
                migrator.revert_migration(migration, connection).await?;
                println!("Reverted {} : {}", migration.app(), migration.name());
            }
        }
        migrator.unlock(connection).await?;
        Ok(())
    }
}
