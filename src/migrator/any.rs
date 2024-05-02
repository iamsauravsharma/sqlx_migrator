use sqlx::any::AnyArguments;
#[cfg(feature = "mysql")]
use sqlx::MySql;
#[cfg(feature = "postgres")]
use sqlx::Postgres;
#[cfg(feature = "sqlite")]
use sqlx::Sqlite;
use sqlx::{Any, Arguments};

#[cfg(feature = "mysql")]
use super::mysql;
#[cfg(feature = "postgres")]
use super::postgres;
#[cfg(feature = "sqlite")]
use super::sqlite;
use super::{DatabaseOperation, Migrator};
use crate::error::Error;
use crate::migration::{AppliedMigrationSqlRow, Migration};

/// get database name
async fn get_database_name(
    connection: &mut <Any as sqlx::Database>::Connection,
) -> Result<Option<String>, Error> {
    let backend_name = connection.backend_name();
    let database_name_query = match backend_name {
        #[cfg(feature = "postgres")]
        <Postgres as sqlx::Database>::NAME => Some(postgres::current_database_query()),
        #[cfg(feature = "sqlite")]
        <Sqlite as sqlx::Database>::NAME => None,
        #[cfg(feature = "mysql")]
        <MySql as sqlx::Database>::NAME => Some(mysql::current_database_query()),
        _ => return Err(Error::UnsupportedDatabase),
    };
    if let Some(sql) = database_name_query {
        let (database_name,) = sqlx::query_as::<_, (String,)>(sql)
            .fetch_one(connection)
            .await?;
        return Ok(Some(database_name));
    }
    Ok(None)
}

#[async_trait::async_trait]
impl<State> DatabaseOperation<Any, State> for Migrator<Any, State>
where
    State: Send + Sync,
{
    async fn ensure_migration_table_exists(
        &self,
        connection: &mut <Any as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        let sql_query = match connection.backend_name() {
            #[cfg(feature = "postgres")]
            <Postgres as sqlx::Database>::NAME => {
                postgres::create_migrator_table_query(self.table_name())
            }
            #[cfg(feature = "sqlite")]
            <Sqlite as sqlx::Database>::NAME => {
                sqlite::create_migrator_table_query(self.table_name())
            }
            #[cfg(feature = "mysql")]
            <MySql as sqlx::Database>::NAME => {
                mysql::create_migrator_table_query(self.table_name())
            }
            _ => return Err(Error::UnsupportedDatabase),
        };
        sqlx::query(&sql_query).execute(connection).await?;
        Ok(())
    }

    async fn drop_migration_table_if_exists(
        &self,
        connection: &mut <Any as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        let sql_query = match connection.backend_name() {
            #[cfg(feature = "postgres")]
            <Postgres as sqlx::Database>::NAME => postgres::drop_table_query(self.table_name()),
            #[cfg(feature = "sqlite")]
            <Sqlite as sqlx::Database>::NAME => sqlite::drop_table_query(self.table_name()),
            #[cfg(feature = "mysql")]
            <MySql as sqlx::Database>::NAME => mysql::drop_table_query(self.table_name()),
            _ => return Err(Error::UnsupportedDatabase),
        };
        sqlx::query(&sql_query).execute(connection).await?;
        Ok(())
    }

    async fn add_migration_to_db_table(
        &self,
        connection: &mut <Any as sqlx::Database>::Connection,
        migration: &Box<dyn Migration<Any, State>>,
    ) -> Result<(), Error> {
        let sql_query = match connection.backend_name() {
            #[cfg(feature = "postgres")]
            <Postgres as sqlx::Database>::NAME => postgres::add_migration_query(self.table_name()),
            #[cfg(feature = "sqlite")]
            <Sqlite as sqlx::Database>::NAME => sqlite::add_migration_query(self.table_name()),
            #[cfg(feature = "mysql")]
            <MySql as sqlx::Database>::NAME => mysql::add_migration_query(self.table_name()),
            _ => return Err(Error::UnsupportedDatabase),
        };
        sqlx::query(&sql_query)
            .bind(migration.app())
            .bind(migration.name())
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn delete_migration_from_db_table(
        &self,
        connection: &mut <Any as sqlx::Database>::Connection,
        migration: &Box<dyn Migration<Any, State>>,
    ) -> Result<(), Error> {
        let sql_query = match connection.backend_name() {
            #[cfg(feature = "postgres")]
            <Postgres as sqlx::Database>::NAME => {
                postgres::delete_migration_query(self.table_name())
            }
            #[cfg(feature = "sqlite")]
            <Sqlite as sqlx::Database>::NAME => sqlite::delete_migration_query(self.table_name()),
            #[cfg(feature = "mysql")]
            <MySql as sqlx::Database>::NAME => mysql::delete_migration_query(self.table_name()),
            _ => return Err(Error::UnsupportedDatabase),
        };
        sqlx::query(&sql_query)
            .bind(migration.app())
            .bind(migration.name())
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn fetch_applied_migration_from_db(
        &self,
        connection: &mut <Any as sqlx::Database>::Connection,
    ) -> Result<Vec<AppliedMigrationSqlRow>, Error> {
        let backend_name = connection.backend_name();
        let query = match backend_name {
            #[cfg(feature = "postgres")]
            <Postgres as sqlx::Database>::NAME => postgres::fetch_rows_query(self.table_name()),
            #[cfg(feature = "sqlite")]
            <Sqlite as sqlx::Database>::NAME => sqlite::fetch_rows_query(self.table_name()),
            #[cfg(feature = "mysql")]
            <MySql as sqlx::Database>::NAME => mysql::fetch_rows_query(self.table_name()),
            _ => return Err(Error::UnsupportedDatabase),
        };
        Ok(sqlx::query_as::<_, AppliedMigrationSqlRow>(&query)
            .fetch_all(connection)
            .await?)
    }

    async fn lock(
        &self,
        connection: &mut <Any as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        let database_name = get_database_name(connection).await?;
        if let Some(name) = database_name {
            let mut arguments = AnyArguments::default();
            let query = match connection.backend_name() {
                #[cfg(feature = "postgres")]
                <Postgres as sqlx::Database>::NAME => {
                    arguments.add(postgres::get_lock_id(&name, self.table_name()));
                    postgres::lock_database_query()
                }
                #[cfg(feature = "sqlite")]
                <Sqlite as sqlx::Database>::NAME => return Ok(()),
                #[cfg(feature = "mysql")]
                <MySql as sqlx::Database>::NAME => {
                    arguments.add(mysql::get_lock_id(&name, self.table_name()));
                    mysql::lock_database_query()
                }
                _ => return Err(Error::UnsupportedDatabase),
            };
            sqlx::query_with(query, arguments)
                .execute(connection)
                .await?;
        }
        Ok(())
    }

    async fn unlock(
        &self,
        connection: &mut <Any as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        let database_name = get_database_name(connection).await?;
        if let Some(name) = database_name {
            let mut arguments = AnyArguments::default();
            let query = match connection.backend_name() {
                #[cfg(feature = "postgres")]
                <Postgres as sqlx::Database>::NAME => {
                    arguments.add(postgres::get_lock_id(&name, self.table_name()));
                    postgres::unlock_database_query()
                }
                #[cfg(feature = "sqlite")]
                <Sqlite as sqlx::Database>::NAME => return Ok(()),
                #[cfg(feature = "mysql")]
                <MySql as sqlx::Database>::NAME => {
                    arguments.add(mysql::get_lock_id(&name, self.table_name()));
                    mysql::unlock_database_query()
                }
                _ => return Err(Error::UnsupportedDatabase),
            };
            sqlx::query_with(query, arguments)
                .execute(connection)
                .await?;
        }
        Ok(())
    }
}
