use sqlx::any::AnyArguments;
use sqlx::{Any, Arguments, MySql, Pool, Postgres, Sqlite};

#[cfg(feature = "mysql")]
use super::mysql;
#[cfg(feature = "postgres")]
use super::postgres;
#[cfg(feature = "sqlite")]
use super::sqlite;
use super::{DatabaseOperation, Migrate, Migrator};
use crate::error::Error;
use crate::migration::{AppliedMigrationSqlRow, Migration};

#[async_trait::async_trait]
impl DatabaseOperation<Any> for Migrator<Any> {
    async fn ensure_migration_table_exists(&self) -> Result<(), Error> {
        let sql_query = match self.pool.acquire().await?.backend_name() {
            #[cfg(feature = "postgres")]
            <Postgres as sqlx::Database>::NAME => postgres::create_migrator_table_query(),
            #[cfg(feature = "sqlite")]
            <Sqlite as sqlx::Database>::NAME => sqlite::create_migrator_table_query(),
            #[cfg(feature = "mysql")]
            <MySql as sqlx::Database>::NAME => mysql::create_migrator_table_query(),
            _ => return Err(Error::UnsupportedDatabase),
        };
        sqlx::query(sql_query).execute(&self.pool).await?;
        Ok(())
    }

    async fn drop_migration_table_if_exists(&self) -> Result<(), Error> {
        let sql_query = match self.pool.acquire().await?.backend_name() {
            #[cfg(feature = "postgres")]
            <Postgres as sqlx::Database>::NAME => postgres::drop_table_query(),
            #[cfg(feature = "sqlite")]
            <Sqlite as sqlx::Database>::NAME => sqlite::drop_table_query(),
            #[cfg(feature = "mysql")]
            <MySql as sqlx::Database>::NAME => mysql::drop_table_query(),
            _ => return Err(Error::UnsupportedDatabase),
        };
        sqlx::query(sql_query).execute(&self.pool).await?;
        Ok(())
    }

    async fn add_migration_to_db_table(
        &self,
        migration: &Box<dyn Migration<Any>>,
        connection: &mut <Any as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        let sql_query = match connection.backend_name() {
            #[cfg(feature = "postgres")]
            <Postgres as sqlx::Database>::NAME => postgres::add_migration_query(),
            #[cfg(feature = "sqlite")]
            <Sqlite as sqlx::Database>::NAME => sqlite::add_migration_query(),
            #[cfg(feature = "mysql")]
            <MySql as sqlx::Database>::NAME => mysql::add_migration_query(),
            _ => return Err(Error::UnsupportedDatabase),
        };
        sqlx::query(sql_query)
            .bind(migration.app())
            .bind(migration.name())
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn delete_migration_from_db_table(
        &self,
        migration: &Box<dyn Migration<Any>>,
        connection: &mut <Any as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        let sql_query = match connection.backend_name() {
            #[cfg(feature = "postgres")]
            <Postgres as sqlx::Database>::NAME => postgres::delete_migration_query(),
            #[cfg(feature = "sqlite")]
            <Sqlite as sqlx::Database>::NAME => sqlite::delete_migration_query(),
            #[cfg(feature = "mysql")]
            <MySql as sqlx::Database>::NAME => mysql::delete_migration_query(),
            _ => return Err(Error::UnsupportedDatabase),
        };
        sqlx::query(sql_query)
            .bind(migration.app())
            .bind(migration.name())
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn fetch_applied_migration_from_db(&self) -> Result<Vec<AppliedMigrationSqlRow>, Error> {
        let connection = self.pool.acquire().await?;
        let backend_name = connection.backend_name();
        let connect_options = self.pool.connect_options();
        let db_url = connect_options.database_url.as_str();
        match backend_name {
            #[cfg(feature = "postgres")]
            <Postgres as sqlx::Database>::NAME => {
                let pool = Pool::connect(db_url).await?;
                postgres::fetch_rows(&pool).await
            }
            #[cfg(feature = "sqlite")]
            <Sqlite as sqlx::Database>::NAME => {
                let pool = Pool::connect(db_url).await?;
                sqlite::fetch_rows(&pool).await
            }
            #[cfg(feature = "mysql")]
            <MySql as sqlx::Database>::NAME => {
                let pool = Pool::connect(db_url).await?;
                mysql::fetch_rows(&pool).await
            }
            _ => return Err(Error::UnsupportedDatabase),
        }
    }

    async fn lock(
        &self,
        connection: &mut <Any as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        let mut query = None;
        let mut arguments = AnyArguments::default();
        let backend_name = connection.backend_name();
        let connect_options = self.pool.connect_options();
        let database_url = connect_options.database_url.as_str();
        match backend_name {
            #[cfg(feature = "postgres")]
            <Postgres as sqlx::Database>::NAME => {
                let pool = Pool::connect(database_url).await?;
                query = Some(postgres::lock_database_query());
                arguments.add(postgres::lock_id(&pool).await?);
            }
            #[cfg(feature = "sqlite")]
            <Sqlite as sqlx::Database>::NAME => {}
            #[cfg(feature = "mysql")]
            <MySql as sqlx::Database>::NAME => {
                let pool = Pool::connect(database_url).await?;
                query = Some(mysql::lock_database_query());
                arguments.add(mysql::lock_id(&pool).await?);
            }
            _ => return Err(Error::UnsupportedDatabase),
        };
        if let Some(sql) = query {
            sqlx::query_with(sql, arguments).execute(connection).await?;
        }
        Ok(())
    }

    async fn unlock(
        &self,
        connection: &mut <Any as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        let mut query = None;
        let mut arguments = AnyArguments::default();
        let backend_name = connection.backend_name();
        let connect_options = self.pool.connect_options();
        let database_url = connect_options.database_url.as_str();
        match backend_name {
            #[cfg(feature = "postgres")]
            <Postgres as sqlx::Database>::NAME => {
                let pool = Pool::connect(database_url).await?;
                query = Some(postgres::unlock_database_query());
                arguments.add(postgres::lock_id(&pool).await?);
            }
            #[cfg(feature = "sqlite")]
            <Sqlite as sqlx::Database>::NAME => {}
            #[cfg(feature = "mysql")]
            <MySql as sqlx::Database>::NAME => {
                let pool = Pool::connect(database_url).await?;
                arguments.add(mysql::lock_id(&pool).await?);
            }
            _ => return Err(Error::UnsupportedDatabase),
        };
        if let Some(sql) = query {
            sqlx::query_with(sql, arguments).execute(connection).await?;
        }
        Ok(())
    }
}

impl Migrate<Any> for Migrator<Any> {}
