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
    #[error("pending migrations exists. Apply all using migrate subcommand")]
    PendingMigrationPresent,
    /// Error when applied migrations exists
    #[cfg(feature = "cli")]
    #[error("applied migrations exists. Revert all using revert subcommand")]
    AppliedMigrationExists,
    /// Error when unsupported database is used as any database
    #[error("unsupported database")]
    UnsupportedDatabase,
    /// Error when table prefix is invalid
    #[error("table prefix name can only contain [a-z0-9_]")]
    InvalidTablePrefix,
    /// Error when passed schema name is invalid
    #[error("schema name can only contain [a-z0-9_] and begin with [a-z_]")]
    InvalidSchema,
    /// Error raised when two migration with same name are added and there value
    /// is not consistent
    #[error("inconsistent migration found for {app} - {name}")]
    InconsistentMigration {
        /// Migration application name
        app: String,
        /// Migration name
        name: String,
    },
    /// Error raised when virtual migration is invalid virtual migration is
    /// invalid if it have any fields present expect app name and migration name
    #[error("invalid virtual migration")]
    InvalidVirtualMigration,
}
