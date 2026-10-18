//! Module which is used to sync a external migration schema to `sqlx_migrator`
//! sqlx migration

use sqlx::Database;

use crate::migrator::SupportedDatabase;
use crate::{Error, Migration, Migrator};

/// Trait which is implemented for syncing a migration from old migrator to
/// new migrator with [`Migrator::sync`]
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

/// A [`Migrator`] can act as the old migrator, which covers renaming the
/// migration table or moving it to another schema: build a migrator with the
/// old configuration and sync from it.
#[async_trait::async_trait]
impl<DB> OldMigrator<DB> for Migrator<DB>
where
    DB: SupportedDatabase,
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
