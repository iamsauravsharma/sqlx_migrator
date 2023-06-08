use sqlx::any::{AnyArguments, AnyKind};
use sqlx::{Any, Arguments, Pool};

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
        let pool = &self.pool;
        let sql_query = match pool.any_kind() {
            #[cfg(feature = "postgres")]
            AnyKind::Postgres => postgres::create_migrator_table_query(),
            #[cfg(feature = "sqlite")]
            AnyKind::Sqlite => sqlite::create_migrator_table_query(),
            #[cfg(feature = "mysql")]
            AnyKind::MySql => mysql::create_migrator_table_query(),
        };
        sqlx::query(sql_query).execute(pool).await?;
        Ok(())
    }

    async fn drop_migration_table_if_exists(&self) -> Result<(), Error> {
        let pool = &self.pool;
        let sql_query = match pool.any_kind() {
            #[cfg(feature = "postgres")]
            AnyKind::Postgres => postgres::drop_table_query(),
            #[cfg(feature = "sqlite")]
            AnyKind::Sqlite => sqlite::drop_table_query(),
            #[cfg(feature = "mysql")]
            AnyKind::MySql => mysql::drop_table_query(),
        };
        sqlx::query(sql_query).execute(&self.pool).await?;
        Ok(())
    }

    async fn add_migration_to_db_table(
        &self,
        migration: &Box<dyn Migration<Any>>,
        connection: &mut <Any as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        let pool = &self.pool;
        let sql_query = match pool.any_kind() {
            #[cfg(feature = "postgres")]
            AnyKind::Postgres => postgres::add_migration_query(),
            #[cfg(feature = "sqlite")]
            AnyKind::Sqlite => sqlite::add_migration_query(),
            #[cfg(feature = "mysql")]
            AnyKind::MySql => mysql::add_migration_query(),
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
        let pool = &self.pool;
        let sql_query = match pool.any_kind() {
            #[cfg(feature = "postgres")]
            AnyKind::Postgres => postgres::delete_migration_query(),
            #[cfg(feature = "sqlite")]
            AnyKind::Sqlite => sqlite::delete_migration_query(),
            #[cfg(feature = "mysql")]
            AnyKind::MySql => mysql::delete_migration_query(),
        };
        sqlx::query(sql_query)
            .bind(migration.app())
            .bind(migration.name())
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn fetch_applied_migration_from_db(&self) -> Result<Vec<AppliedMigrationSqlRow>, Error> {
        let pool = &self.pool;
        let sql_query = match pool.any_kind() {
            #[cfg(feature = "postgres")]
            AnyKind::Postgres => postgres::fetch_row_query(),
            #[cfg(feature = "sqlite")]
            AnyKind::Sqlite => sqlite::fetch_row_query(),
            #[cfg(feature = "mysql")]
            AnyKind::MySql => mysql::fetch_row_query(),
        };
        let rows = sqlx::query_as(sql_query).fetch_all(&self.pool).await?;
        Ok(rows)
    }

    async fn lock(
        &self,
        connection: &mut <Any as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        let connect_options = self.pool.connect_options();
        let mut query = None;
        let mut arguments = AnyArguments::default();
        match connect_options.kind() {
            #[cfg(feature = "postgres")]
            AnyKind::Postgres => {
                let connect_options = connect_options
                    .as_postgres()
                    .ok_or(Error::FailedDatabaseConversion)?
                    .clone();
                let pool = Pool::connect_with(connect_options).await?;
                query = Some(postgres::lock_database_query());
                arguments.add(postgres::lock_id(&pool).await?);
            }
            #[cfg(feature = "sqlite")]
            AnyKind::Sqlite => {}
            #[cfg(feature = "mysql")]
            AnyKind::MySql => {
                let connect_options = connect_options
                    .as_mysql()
                    .ok_or(Error::FailedDatabaseConversion)?
                    .clone();
                let pool = Pool::connect_with(connect_options).await?;
                query = Some(mysql::lock_database_query());
                arguments.add(mysql::lock_id(&pool).await?);
            }
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
        let connect_options = self.pool.connect_options();
        let mut query = None;
        let mut arguments = AnyArguments::default();
        match connect_options.kind() {
            #[cfg(feature = "postgres")]
            AnyKind::Postgres => {
                let connect_options = connect_options
                    .as_postgres()
                    .ok_or(Error::FailedDatabaseConversion)?
                    .clone();
                let pool = Pool::connect_with(connect_options).await?;
                query = Some(postgres::unlock_database_query());
                arguments.add(postgres::lock_id(&pool).await?);
            }
            #[cfg(feature = "sqlite")]
            AnyKind::Sqlite => {}
            #[cfg(feature = "mysql")]
            AnyKind::MySql => {
                let connect_options = connect_options
                    .as_mysql()
                    .ok_or(Error::FailedDatabaseConversion)?
                    .clone();
                let pool = Pool::connect_with(connect_options).await?;
                query = Some(mysql::unlock_database_query());
                arguments.add(mysql::lock_id(&pool).await?);
            }
        };
        if let Some(sql) = query {
            sqlx::query_with(sql, arguments).execute(connection).await?;
        }
        Ok(())
    }
}

impl Migrate<Any> for Migrator<Any> {}
