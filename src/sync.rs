//! Module which is used to sync a external migration schema to `sqlx_migrator`
//! sqlx migration

use sqlx::Database;

use crate::migrator::DatabaseOperation;
use crate::{Error, Info, Migration, Migrator};

/// Trait which is implemented for syncing a migration from old migrator to
/// new migrator
#[async_trait::async_trait]
pub trait OldMigrator<DB>: Send + Sync
where
    DB: Database,
{
    /// Returns a list of applied migrations from the old migrator
    async fn applied_migrations(
        &self,
        connection: &mut <DB as Database>::Connection,
    ) -> Result<Vec<Box<dyn Migration<DB>>>, Error>;
}

#[async_trait::async_trait]
impl<DB, T> OldMigrator<DB> for T
where
    DB: Database,
    Self: DatabaseOperation<DB> + Send + Sync,
{
    async fn applied_migrations(
        &self,
        connection: &mut <DB as Database>::Connection,
    ) -> Result<Vec<Box<dyn Migration<DB>>>, Error> {
        let mut applied_list: Vec<Box<dyn Migration<DB>>> = vec![];
        for migration in self.fetch_applied_migration_from_db(connection).await? {
            applied_list.push(Box::new((
                migration.app().to_string(),
                migration.name().to_string(),
            )));
        }
        Ok(applied_list)
    }
}

/// Trait which is implemented for syncing a migration from old migrator to new
/// migrator for a specific database
#[async_trait::async_trait]
pub trait Synchronize<DB>: Info<DB> + DatabaseOperation<DB>
where
    DB: Database,
{
    /// Syncs the applied migrations from an old migrator to the current
    /// migrator
    ///
    /// This function serves two primary purposes:
    /// 1. Migrating from external migrations to`sqlx_migrator` migration
    /// 2. Handling table renames
    ///
    /// Note: Migration timestamps are not preserved - the current time is used
    /// for entries
    ///
    /// # Errors
    /// If sync cannot be completed
    async fn sync<O>(
        &self,
        connection: &mut <DB as Database>::Connection,
        old_migrator: &O,
    ) -> Result<(), Error>
    where
        O: OldMigrator<DB>,
    {
        tracing::debug!("syncing old migrator");
        self.lock(connection).await?;
        // Use a block to ensure the lock is released before returning
        let result = async {
            let old_migrator_applied_migrations =
                old_migrator.applied_migrations(connection).await?;
            let already_applied_migration =
                self.fetch_applied_migration_from_db(connection).await?;
            let full_migration_list = self.migrations();
            for migration in old_migrator_applied_migrations {
                // Only add migration if it exists in the full migration list
                // and has not already been applied in the new migrator
                if full_migration_list.contains(&migration)
                    && !already_applied_migration
                        .iter()
                        .any(|applied| applied == &migration)
                {
                    self.add_migration_to_db_table(connection, &migration)
                        .await?;
                }
            }
            Ok(())
        }
        .await;
        self.unlock(connection).await?;
        result
    }
}

impl<DB> Synchronize<DB> for Migrator<DB>
where
    DB: Database,
    Self: DatabaseOperation<DB>,
{
}
