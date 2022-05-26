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
    List,
    ApplyAll,
    RevertAll,
    RevertLatest,
}

async fn print_all_migrations<DB>(migrator: Box<dyn Migrator<Database = DB>>) -> Result<(), Error>
where
    DB: sqlx::Database,
{
    let applied_migrations = migrator.list_applied_migrations().await?;
    for migration in migrator.generate_full_migration_plan()? {
        if applied_migrations.contains(&migration) {
            println!("{} (applied)", migration.name());
        } else {
            println!("{} (not applied)", migration.name());
        }
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
        SubCommand::List => print_all_migrations(migrator).await?,
        SubCommand::ApplyAll => apply_all_migrations(migrator).await?,
        SubCommand::RevertAll => revert_all_migrations(migrator).await?,
        SubCommand::RevertLatest => revert_latest(migrator).await?,
    }
    Ok(())
}
