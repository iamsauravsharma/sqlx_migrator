//! Module for library error

/// Error enum to store different types of error
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Error type created from error raised by sqlx
    #[error(transparent)]
    SqlxError(#[from] sqlx::error::Error),

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

    /// Error when applied migrations exists
    #[error("applied migrations exists. Revert all using revert subcommand")]
    AppliedMigrationExists,

    /// Error when database pool cannot be created from any pool
    #[error("failed to create database pool")]
    FailedDatabaseConversion,
}
