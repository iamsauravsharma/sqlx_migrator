//! Module for library error

/// Error enum to store different types of error
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Error type created from error raised by sqlx
    #[error(transparent)]
    SqlxError(#[from] sqlx::error::Error),

    #[cfg(feature = "cli")]
    /// Error type created from error raised by std input output
    #[error(transparent)]
    StdIoError(#[from] std::io::Error),

    /// Error for failed to create migrations plan
    #[error("failed to create migrations plan")]
    FailedToCreateMigrationPlan,

    /// Error when migration plan has applied replaces migrations as well as
    /// current migration
    #[error("both replace migrations and current migration are applied")]
    BothMigrationTypeApplied,

    /// Error for irreversible operation
    #[error("operation is irreversible")]
    IrreversibleOperation,

    #[cfg(feature = "cli")]
    /// Error for pending migration present
    #[error("pending migration present")]
    PendingMigrationPresent,

    /// Error when migration name is only present but not app name
    #[error("app name required only migration name present")]
    AppNameRequired,

    /// Error when provided app name doesn't exists
    #[error("provided app {app} doesn't exists")]
    AppNameNotExists {
        /// Name of app
        app: String,
    },

    /// Error when provided migration name doesn't exists for app
    #[error("provided migration {migration} doesn't exists for app {app}")]
    MigrationNameNotExists {
        /// Name of app
        app: String,
        /// Name of migration
        migration: String,
    },

    #[cfg(feature = "cli")]
    /// Error when applied migrations exists
    #[error("applied migrations exists. Revert all using revert subcommand")]
    AppliedMigrationExists,

    #[cfg(feature = "cli")]
    /// Error when count of migrations is big than total number of migration
    #[error("count of migrations is big only {actual_len} present passed {count}")]
    CountGreater {
        /// Actual length of migration
        actual_len: usize,
        /// Count passed in option
        count: usize,
    },

    /// Error when unsupported database is used as any database
    #[error("database not supported for any migrator")]
    UnsupportedDatabase,

    /// Error when passed prefix is not alpha numeric
    #[error("prefix can only be ascii alphanumeric and underscore character")]
    NonAsciiAlphaNumeric,
}
