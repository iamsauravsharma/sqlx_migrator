//! `drop` subcommand
#![expect(clippy::print_stdout, reason = "allow printing to stdout in cli")]
use sqlx::Database;

use crate::error::Error;
use crate::migrator::{Migrator, SupportedDatabase};

pub(super) async fn drop_migrations<DB>(
    connection: &mut <DB as Database>::Connection,
    migrator: &Migrator<DB>,
) -> Result<(), Error>
where
    DB: SupportedDatabase,
{
    // hold lock for dropping migration table to avoid race condition where
    // another process is applying migration and we are dropping the table
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
