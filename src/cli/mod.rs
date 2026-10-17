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
use clap::{Parser, Subcommand};
use sqlx::Database;

use self::apply::Apply;
use self::list::List;
use self::prune::Prune;
use self::redo::Redo;
use self::revert::Revert;
use crate::error::Error;
use crate::migrator::Migrate;

mod apply;
mod drop;
mod list;
mod output;
mod prune;
mod redo;
mod revert;
#[cfg(test)]
mod tests;

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
    List(List),
    /// Remove entries from the migration table whose migration no longer
    /// exists in the migration list
    Prune(Prune),
    /// Revert the last applied migration and apply it again
    Redo(Redo),
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
            SubCommand::Drop => drop::drop_migrations(connection, migrator).await?,
            SubCommand::List(list) => list.run(connection, migrator).await?,
            SubCommand::Prune(prune) => prune.run(connection, migrator).await?,
            SubCommand::Redo(redo) => redo.run(connection, migrator).await?,
            SubCommand::Revert(revert) => revert.run(connection, migrator).await?,
        }
        Ok(())
    }
}
