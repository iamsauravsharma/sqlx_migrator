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
}
