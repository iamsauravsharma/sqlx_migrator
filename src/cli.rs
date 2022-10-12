//! Module for creating and running cli with help of migrator
use clap::{Parser, Subcommand};

use crate::error::Error;
use crate::migrator::{Migrator, PlanType};

#[derive(Parser, Debug)]
struct Args {
    #[command(subcommand)]
    sub_command: SubCommand,
}

#[derive(Subcommand, Debug)]
enum SubCommand {
    #[command(about = "List migrations along with their status")]
    List,
    #[command(about = "Apply migrations")]
    Apply(Apply),
    #[command(about = "Revert migrations")]
    Revert(Revert),
}

#[derive(Parser, Debug)]
struct Apply {
    #[arg(long = "plan", short = 'p', help = "Show plan")]
    plan: bool,
    #[arg(long = "check", short = 'c', help = "Check for pending migration")]
    check: bool,
    #[arg(
        long = "fake",
        short = 'f',
        help = "Make migration applied without applying"
    )]
    fake: bool,
}

#[derive(Parser, Debug)]
struct Revert {
    #[arg(long = "plan", short = 'p', help = "Show plan")]
    plan: bool,
    #[arg(long = "all", short = 'a', help = "Revert all migration")]
    all: bool,
    #[arg(
        long = "fake",
        short = 'f',
        help = "Make migration reverted without reverting"
    )]
    fake: bool,
}

async fn list_migrations<DB>(migrator: Box<dyn Migrator<Database = DB>>) -> Result<(), Error>
where
    DB: sqlx::Database,
{
    migrator.ensure_migration_table_exists().await?;
    let applied_migrations = migrator.fetch_applied_migration_from_db().await?;

    let widths = [5, 10, 30, 10, 20];
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
    for migration in migrator.generate_migration_plan(PlanType::Full).await? {
        // let mutable_applied_migrations = applied_migrations.clone();
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

async fn apply_migrations<DB>(
    migrator: Box<dyn Migrator<Database = DB>>,
    apply: Apply,
) -> Result<(), Error>
where
    DB: sqlx::Database,
{
    let migrations = migrator.generate_migration_plan(PlanType::Apply).await?;
    if apply.check && !migrations.is_empty() {
        return Err(Error::PendingMigration);
    }
    if apply.plan {
        let first_width = 10;
        let second_width = 30;
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
    } else if apply.fake {
        let mut connection = migrator.pool().acquire().await?;
        for migration in migrations {
            migrator
                .add_migration_to_db_table(migration, &mut connection)
                .await?;
        }
    } else {
        migrator.apply_all().await?;
    }
    Ok(())
}

async fn revert_migrations<DB>(
    migrator: Box<dyn Migrator<Database = DB>>,
    revert: Revert,
) -> Result<(), Error>
where
    DB: sqlx::Database,
{
    let revert_plan = migrator.generate_migration_plan(PlanType::Revert).await?;
    let revert_migrations;
    if revert.all {
        revert_migrations = revert_plan;
    } else if let Some(latest_migration) = revert_plan.first() {
        revert_migrations = vec![latest_migration];
    } else {
        revert_migrations = vec![];
    }
    if revert.plan {
        let first_width = 10;
        let second_width = 30;
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
    } else if revert.fake {
        let mut connection = migrator.pool().acquire().await?;
        for migration in revert_migrations {
            migrator
                .delete_migration_from_db_table(migration, &mut connection)
                .await?;
        }
    } else {
        for migration in revert_migrations {
            migrator.revert_migration(migration).await?;
        }
    }
    Ok(())
}

/// Run cli by parsing args with help of migrator
///
/// # Errors
/// When command fails to run
pub async fn run<DB>(migrator: Box<dyn Migrator<Database = DB>>) -> Result<(), Error>
where
    DB: sqlx::Database,
{
    let args = Args::parse();
    match args.sub_command {
        SubCommand::List => list_migrations(migrator).await?,
        SubCommand::Apply(apply) => apply_migrations(migrator, apply).await?,
        SubCommand::Revert(revert) => revert_migrations(migrator, revert).await?,
    }
    Ok(())
}
