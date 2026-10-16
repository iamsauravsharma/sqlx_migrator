/// Macro for vector of [`Box`]
#[macro_export]
macro_rules! vec_box {
    ($($x:expr),* $(,)?) => (vec![$(Box::new($x)),*]);
}

/// Macro for implementing the [Migration](crate::migration::Migration) trait
/// for the provided database.
///
/// The macro covers the simple case: app, name, parents and operations. For
/// migrations which need `replaces`, `run_before` or `is_atomic`, implement
/// the [Migration](crate::migration::Migration) trait by hand.
///
/// The database specific macros
/// ([`postgres_migration!`](crate::postgres_migration),
/// [`sqlite_migration!`](crate::sqlite_migration),
/// [`mysql_migration!`](crate::mysql_migration) and
/// [`any_migration!`](crate::any_migration)) take the same arguments without
/// the leading database type.
///
/// Arguments, in order:
/// - `$db:ty`: the type of database
/// - `$op:ty`: The type for which the migration is being implemented
/// - `$app_name:expr`: Name of app to be used for app variable
/// - `$migration_name:expr`: Name of migration to be used for the `name`
///   function
/// - `$parents:expr`: List of parents migration.
/// - `$operations:expr`: List of operations
///
/// ```ignore
/// sqlx_migrator::migration!(
///     sqlx::Postgres,
///     FirstMigration,
///     "main",
///     "first_migration",
///     sqlx_migrator::vec_box![],
///     sqlx_migrator::vec_box![FirstOperation]
/// );
/// ```
#[macro_export]
macro_rules! migration {
    (
        $db:ty, $op:ty, $app_name:expr, $migration_name:expr, $parents:expr, $operations:expr $(,)?
    ) => {
        impl $crate::migration::Migration<$db> for $op {
            fn app(&self) -> &str {
                $app_name
            }

            fn name(&self) -> &str {
                $migration_name
            }

            fn parents(&self) -> Vec<Box<dyn $crate::migration::Migration<$db>>> {
                $parents
            }

            fn operations(&self) -> Vec<Box<dyn $crate::operation::Operation<$db>>> {
                $operations
            }
        }
    };
}

/// Macro for building an operation from SQL files embedded at compile time.
///
/// Paths are resolved relative to the file which invokes the macro, the same
/// way [`include_str!`] resolves them.
///
/// Forms:
/// - `include_sql!("sql/0001")` reads `sql/0001.up.sql` and `sql/0001.down.sql`
///   and builds a reversible operation.
/// - `include_sql!("a.sql", "b.sql")` or `include_sql!(up: "a.sql", down:
///   "b.sql")` builds a reversible operation from explicit files.
/// - `include_sql!(up: "a.sql")` builds an irreversible operation which only
///   has an `up` step.
///
/// ```ignore
/// sqlx_migrator::postgres_migration!(
///     FirstMigration,
///     "main",
///     "first_migration",
///     sqlx_migrator::vec_box![],
///     sqlx_migrator::vec_box![
///         sqlx_migrator::include_sql!("sql/0001"),
///         sqlx_migrator::include_sql!(up: "sql/0001_seed.sql"),
///     ]
/// );
/// ```
#[macro_export]
macro_rules! include_sql {
    (up: $up:literal,down: $down:literal $(,)?) => {
        (include_str!($up), include_str!($down))
    };
    (up: $up:literal $(,)?) => {
        include_str!($up)
    };
    ($up:literal, $down:literal $(,)?) => {
        (include_str!($up), include_str!($down))
    };
    ($base:literal $(,)?) => {
        (
            include_str!(concat!($base, ".up.sql")),
            include_str!(concat!($base, ".down.sql")),
        )
    };
}

/// Macro for implementing the [`migration`] macro for the `Any`.
///
/// Takes the same arguments as [`migration`] without the leading database
/// type.
#[macro_export]
#[cfg(all(
    any(feature = "postgres", feature = "mysql", feature = "sqlite"),
    feature = "any"
))]
macro_rules! any_migration {
    ($($args:tt)*) => {
        $crate::migration!(sqlx::Any, $($args)*);
    };
}

/// Macro for implementing the [`migration`] macro for the `MySql`.
///
/// Takes the same arguments as [`migration`] without the leading database
/// type.
#[macro_export]
#[cfg(feature = "mysql")]
macro_rules! mysql_migration {
    ($($args:tt)*) => {
        $crate::migration!(sqlx::MySql, $($args)*);
    };
}

/// Macro for implementing the [`migration`] macro for the `Postgres`.
///
/// Takes the same arguments as [`migration`] without the leading database
/// type.
#[macro_export]
#[cfg(feature = "postgres")]
macro_rules! postgres_migration {
    ($($args:tt)*) => {
        $crate::migration!(sqlx::Postgres, $($args)*);
    };
}

/// Macro for implementing the [`migration`] macro for the `Sqlite`.
///
/// Takes the same arguments as [`migration`] without the leading database
/// type.
#[macro_export]
#[cfg(feature = "sqlite")]
macro_rules! sqlite_migration {
    ($($args:tt)*) => {
        $crate::migration!(sqlx::Sqlite, $($args)*);
    };
}
