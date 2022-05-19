//! Module for library error

/// Error enum to store different types of error
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Error type for all sqlx error
    #[error(transparent)]
    SqlxError(#[from] sqlx::error::Error),

    /// Error for failed to get env variable
    #[error("Failed to get env {0}")]
    FailedToGetEnv(&'static str),

    /// Error for failed to create migrations plan from cyclic dependency
    #[error("Failed to create migrations plan due to migration cyclic dependency")]
    FailedToCreateMigrationPlan,
}
