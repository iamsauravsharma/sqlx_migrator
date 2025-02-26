//! Module for library error

/// Error enum to store different types of error
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Error type created from error raised by sqlx
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    /// Error type created from error raised by box error
    #[error(transparent)]
    Box(#[from] Box<dyn std::error::Error + Send + Sync>),
    /// Error type created from error raised by std input output
    #[cfg(feature = "cli")]
    #[error(transparent)]
    StdIo(#[from] std::io::Error),
    /// Error generated during planning state
    #[error("plan error: {message}")]
    PlanError {
        /// Message for error
        message: String,
    },
    /// Error for irreversible operation
    #[error("operation is irreversible")]
    IrreversibleOperation,
    /// Error for pending migration present
    #[cfg(feature = "cli")]
    #[error("pending migration present")]
    PendingMigrationPresent,
    /// Error when applied migrations exists
    #[cfg(feature = "cli")]
    #[error("applied migrations exists. Revert all using revert subcommand")]
    AppliedMigrationExists,
    /// Error when unsupported database is used as any database
    #[error("database not supported")]
    UnsupportedDatabase,
    /// Error when passed prefix is not alpha numeric
    #[error("prefix can only be ascii alphanumeric and underscore character")]
    NonAsciiAlphaNumeric,
    /// Error raised when two migration with same name are added and there value
    /// is not consistent
    #[error("migration for app: {app} with name: {name} consists of inconsistent values")]
    InconsistentMigration {
        /// Migration application name
        app: String,
        /// Migration name
        name: String,
    },
}
