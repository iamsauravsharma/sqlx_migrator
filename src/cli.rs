//! Module for creating and running cli with help of migrator
use clap::{Parser, Subcommand};

use crate::error::Error;
use crate::migrator::Migrator;

#[derive(Parser, Debug)]
struct Args {
    #[clap(subcommand)]
    sub_command: SubCommand,
}

#[derive(Subcommand, Debug)]
enum SubCommand {
    #[clap(about = "List all migrations along with their status")]
    List,
    #[clap(about = "Apply all migrations")]
    ApplyAll,
    #[clap(about = "Revert all migrations")]
    RevertAll,
    #[clap(about = "Revert latest migration")]
    RevertLatest,
}

async fn list_all_migrations<DB>(migrator: Box<dyn Migrator<Database = DB>>) -> Result<(), Error>
where
    DB: sqlx::Database,
{
    let applied_migrations = migrator.list_applied_migrations().await?;
    let first_width = 30;
    let second_width = 10;
    let full_width = first_width + second_width + 3;
    println!(
        "{:^first_width$} | {:^second_width$}",
        "Migration", "Status"
    );
    println!("{:^full_width$}", "-".repeat(full_width));
    for migration in migrator.generate_full_migration_plan()? {
        println!(
            "{:^first_width$} | {:^second_width$}",
            migration.name(),
            if applied_migrations.contains(&migration) {
                "✅"
            } else {
                "❌"
            }
        );
    }
    Ok(())
}

async fn apply_all_migrations<DB>(migrator: Box<dyn Migrator<Database = DB>>) -> Result<(), Error>
where
    DB: sqlx::Database,
{
    migrator.apply_all().await?;
    Ok(())
}

async fn revert_all_migrations<DB>(migrator: Box<dyn Migrator<Database = DB>>) -> Result<(), Error>
where
    DB: sqlx::Database,
{
    migrator.revert_all().await?;
    Ok(())
}

async fn revert_latest<DB>(migrator: Box<dyn Migrator<Database = DB>>) -> Result<(), Error>
where
    DB: sqlx::Database,
{
    let revert_plan = migrator.revert_all_plan().await?;
    if let Some(latest_migration) = revert_plan.get(0) {
        migrator.revert_migration(latest_migration).await?;
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
    migrator.ensure_migration_table_exists().await?;
    match args.sub_command {
        SubCommand::List => list_all_migrations(migrator).await?,
        SubCommand::ApplyAll => apply_all_migrations(migrator).await?,
        SubCommand::RevertAll => revert_all_migrations(migrator).await?,
        SubCommand::RevertLatest => revert_latest(migrator).await?,
    }
    Ok(())
}
