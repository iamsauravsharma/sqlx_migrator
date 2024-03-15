use sqlx::MySql;

use super::{DatabaseOperation, Migrator};
use crate::error::Error;
use crate::migration::{AppliedMigrationSqlRow, Migration};

/// create migrator table query
#[must_use]
pub(crate) fn create_migrator_table_query(table_name: &str) -> String {
    format!(
        "CREATE TABLE IF NOT EXISTS {table_name} (
        id INT PRIMARY KEY NOT NULL AUTO_INCREMENT,
        app VARCHAR(384) NOT NULL,
        name VARCHAR(384) NOT NULL,
        applied_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        UNIQUE (app, name)
    )"
    )
}

/// Drop table query
#[must_use]
pub(crate) fn drop_table_query(table_name: &str) -> String {
    format!("DROP TABLE IF EXISTS {table_name}")
}

/// fetch rows
pub(crate) fn fetch_rows_query(table_name: &str) -> String {
    format!(
        "SELECT id, app, name, DATE_FORMAT(applied_time, '%Y-%m-%d %H:%i:%s') AS applied_time \
         FROM {table_name}"
    )
}

/// add migration query
#[must_use]
pub(crate) fn add_migration_query(table_name: &str) -> String {
    format!("INSERT INTO {table_name}(app, name) VALUES (?, ?)")
}

/// delete migration query
#[must_use]
pub(crate) fn delete_migration_query(table_name: &str) -> String {
    format!("DELETE FROM {table_name} WHERE app = ? AND name = ?")
}

/// get current database query
pub(crate) fn current_database_query() -> &'static str {
    "SELECT DATABASE()"
}

/// get lock database query
/// # Errors
/// Failed to lock database
pub(crate) fn lock_database_query() -> &'static str {
    "SELECT GET_LOCK(?, -1)"
}

/// get lock database query
/// # Errors
/// Failed to lock database
pub(crate) fn unlock_database_query() -> &'static str {
    "SELECT RELEASE_LOCK(?)"
}

/// generate lock id
pub(crate) fn get_lock_id(database_name: &str, table_name: &str) -> String {
    let buf = format!("{database_name}/{table_name}");
    crc32fast::hash(buf.as_bytes()).to_string()
}

#[async_trait::async_trait]
impl<State> DatabaseOperation<MySql, State> for Migrator<MySql, State>
where
    State: Send + Sync,
{
    async fn ensure_migration_table_exists(
        &self,
        connection: &mut <MySql as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        sqlx::query(&create_migrator_table_query(self.table_name()))
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn drop_migration_table_if_exists(
        &self,
        connection: &mut <MySql as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        sqlx::query(&drop_table_query(self.table_name()))
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn add_migration_to_db_table(
        &self,
        migration: &Box<dyn Migration<MySql, State>>,
        connection: &mut <MySql as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        sqlx::query(&add_migration_query(self.table_name()))
            .bind(migration.app())
            .bind(migration.name())
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn delete_migration_from_db_table(
        &self,
        migration: &Box<dyn Migration<MySql, State>>,
        connection: &mut <MySql as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        sqlx::query(&delete_migration_query(self.table_name()))
            .bind(migration.app())
            .bind(migration.name())
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn fetch_applied_migration_from_db(
        &self,
        connection: &mut <MySql as sqlx::Database>::Connection,
    ) -> Result<Vec<AppliedMigrationSqlRow>, Error> {
        Ok(
            sqlx::query_as::<_, AppliedMigrationSqlRow>(&fetch_rows_query(self.table_name()))
                .fetch_all(connection)
                .await?,
        )
    }

    async fn lock(
        &self,
        connection: &mut <MySql as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        let (database_name,): (String,) = sqlx::query_as(current_database_query())
            .fetch_one(&mut *connection)
            .await?;
        let lock_id = get_lock_id(&database_name, self.table_name());
        sqlx::query(lock_database_query())
            .bind(lock_id)
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn unlock(
        &self,
        connection: &mut <MySql as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        let (database_name,): (String,) = sqlx::query_as(current_database_query())
            .fetch_one(&mut *connection)
            .await?;
        let lock_id = get_lock_id(&database_name, self.table_name());
        sqlx::query(unlock_database_query())
            .bind(lock_id)
            .execute(connection)
            .await?;
        Ok(())
    }
}
