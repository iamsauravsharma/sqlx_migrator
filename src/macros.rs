/// Macro for vector of box
#[macro_export]
macro_rules! vec_box {
    ($elem:expr; $n:expr) => (vec![Box::new($elem); $n]);
    ($($x:expr),*) => (vec![$(Box::new($x)),*]);
    ($($x:expr,)*) => (vec![$(Box::new($x)),*]);
    ($($x:expr,)*) => (vec_box![$($x),*]);
}

/// Macro for defining SQL operations.
///
/// This macro expects four arguments:
///
/// - `$db`: the type of database
/// - `$op`: The type of for which operation is defined.
/// - `$up`: The SQL query string for performing the migration 'up' (forward
///   migration).
/// - `$down`: The SQL query string for performing the migration 'down'
///   (backward migration).
#[macro_export]
macro_rules! operation {
    ($db:ty, $op:ty, $up:literal, $down:literal) => {
        #[async_trait::async_trait]
        impl sqlx_migrator::operation::Operation<$db> for $op {
            async fn up(
                &self,
                connection: &mut <$db as sqlx::Database>::Connection,
            ) -> Result<(), sqlx_migrator::error::Error> {
                sqlx_migrator::sqlx::query($up).execute(connection).await?;
                Ok(())
            }

            async fn down(
                &self,
                connection: &mut <$db as sqlx::Database>::Connection,
            ) -> Result<(), sqlx_migrator::error::Error> {
                sqlx_migrator::sqlx::query($down)
                    .execute(connection)
                    .await?;
                Ok(())
            }
        }
    };
}

/// Macro for defining any SQL operations.
///
/// This macro is extend of operation macro which has already set db to
/// `sqlx::Any`
#[macro_export]
#[cfg(all(
    any(feature = "postgres", feature = "mysql", feature = "sqlite"),
    feature = "any"
))]
macro_rules! any_operation {
    ($op:ty, $up:literal, $down:literal) => {
        sqlx_migrator::operation!(sqlx_migrator::sqlx::Any, $op, $up, $down);
    };
}

/// Macro for defining mysql SQL operations.
///
/// This macro is extend of operation macro which has already set db to
/// `sqlx::MySql`
#[macro_export]
#[cfg(feature = "mysql")]
macro_rules! mysql_operation {
    ($op:ty, $up:literal, $down:literal) => {
        sqlx_migrator::operation!(sqlx_migrator::sqlx::MySql, $op, $up, $down);
    };
}

/// Macro for defining postgres SQL operations.
///
/// This macro is extend of operation macro which has already set db to
/// `sqlx::Postgres`
#[macro_export]
#[cfg(feature = "postgres")]
macro_rules! postgres_operation {
    ($op:ty, $up:literal, $down:literal) => {
        sqlx_migrator::operation!(sqlx_migrator::sqlx::Postgres, $op, $up, $down);
    };
}

/// Macro for defining sqlite SQL operations.
///
/// This macro is extend of operation macro which has already set db to
/// `sqlx::Sqlite`
#[macro_export]
#[cfg(feature = "sqlite")]
macro_rules! sqlite_operation {
    ($op:ty, $up:literal, $down:literal) => {
        sqlx_migrator::operation!(sqlx_migrator::sqlx::Sqlite, $op, $up, $down);
    };
}

/// Macro for implementing the `Migration` trait for the provided type.
///
/// This macro generates implementations for the `Migration` trait from the
/// `sqlx_migrator` crate.
/// This macro will use current file name as name for migration
///
/// This macro expects the following arguments:
/// - `$db:ty`: the type of database
/// - `$app_name:literal`: Name of app to be used for app variable
/// - `$op:ty`: The type for which the migration is being implemented
/// - `$parents:expr`: List of parents migration.
/// - `$operations:expr`: List of operations
#[macro_export]
macro_rules! migration {
    ($db:ty, $app_name:literal, $op:ty, $parents:expr, $operations:expr) => {
        #[async_trait::async_trait]
        impl sqlx_migrator::migration::Migration<$db> for $op {
            fn app(&self) -> &str {
                $app_name
            }

            fn name(&self) -> &str {
                std::path::Path::new(file!())
                    .file_stem()
                    .map(|stem_os_str| stem_os_str.to_str().unwrap_or_default())
                    .unwrap_or_default()
            }

            fn parents(&self) -> Vec<Box<dyn sqlx_migrator::migration::Migration<$db>>> {
                $parents
            }

            fn operations(&self) -> Vec<Box<dyn sqlx_migrator::operation::Operation<$db>>> {
                $operations
            }
        }
    };
}

/// Macro for implementing the `Migration` trait for the `Any`.
///
/// This macro is extend of migration macro which has already set db to
/// `sqlx::Any`
#[macro_export]
#[cfg(all(
    any(feature = "postgres", feature = "mysql", feature = "sqlite"),
    feature = "any"
))]
macro_rules! any_migration {
    ($app_name:literal, $op:ty, $parents:expr, $operations:expr) => {
        sqlx_migrator::migration!(
            sqlx_migrator::sqlx::Any,
            $app_name,
            $op,
            $parents,
            $operations
        );
    };
}

/// Macro for implementing the `Migration` trait for the `MySql`.
///
/// This macro is extend of migration macro which has already set db to
/// `sqlx::MySql`
#[macro_export]
#[cfg(feature = "mysql")]
macro_rules! mysql_migration {
    ($app_name:literal, $op:ty, $parents:expr, $operations:expr) => {
        sqlx_migrator::migration!(
            sqlx_migrator::sqlx::MySql,
            $app_name,
            $op,
            $parents,
            $operations
        );
    };
}

/// Macro for implementing the `Migration` trait for the `Postgres`.
///
/// This macro is extend of migration macro which has already set db to
/// `sqlx::Postgres`
#[macro_export]
#[cfg(feature = "postgres")]
macro_rules! postgres_migration {
    ($app_name:literal, $op:ty, $parents:expr, $operations:expr) => {
        sqlx_migrator::migration!(
            sqlx_migrator::sqlx::Postgres,
            $app_name,
            $op,
            $parents,
            $operations
        );
    };
}

/// Macro for implementing the `Migration` trait for the `Sqlite`.
///
/// This macro is extend of migration macro which has already set db to
/// `sqlx::Sqlite`
#[macro_export]
#[cfg(feature = "sqlite")]
macro_rules! sqlite_migration {
    ($app_name:literal, $op:ty, $parents:expr, $operations:expr) => {
        sqlx_migrator::migration!(
            sqlx_migrator::sqlx::Sqlite,
            $app_name,
            $op,
            $parents,
            $operations
        );
    };
}
