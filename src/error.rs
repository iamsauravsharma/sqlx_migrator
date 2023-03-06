//! Module for library error

/// Error enum to store different types of error
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Error type created from error raised by sqlx
    #[error(transparent)]
    SqlxError(#[from] sqlx::error::Error),

    /// Error for failed to create migrations plan
    #[error("Failed to create migrations plan")]
    FailedToCreateMigrationPlan,

    /// Error for irreversible operation
    #[error("Operation is irreversible")]
    IrreversibleOperation,

    /// Error for pending migration present
    #[error("Pending migration present")]
    PendingMigrationPresent,

    /// Error when migration name is only present but not app name
    #[error("App name required. Only migration name present")]
    AppNameRequired,

    /// Error when provided app name doesn't exists
    #[error("Provided app {app} doesn't exists")]
    AppNameNotExists {
        /// Name of app
        app: String,
    },

    /// Error when provided migration name doesn't exists for app
    #[error("Provided migration {migration} doesn't exists for app {app}")]
    MigrationNameNotExists {
        /// Name of app
        app: String,
        /// Name of migration
        migration: String,
    },

    /// Error when applied migrations exists
    #[error("Applied migrations exists. Revert all using revert subcommand")]
    AppliedMigrationExists,
}
