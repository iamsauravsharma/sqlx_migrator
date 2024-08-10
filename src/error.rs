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
    /// Error generated when no migration are added
    #[error("no migration are added to migration list")]
    NoMigrationAdded,
    /// Error generated if virtual migration is present while running generate
    /// migration plan
    #[error("virtual migrations which is not replaced is present in migration plan")]
    VirtualMigrationPresent,
    /// Error for failed to create migrations plan
    #[error("failed to create migrations plan")]
    FailedToCreateMigrationPlan,
    /// Parent is not applied
    #[error("children is applied before parent")]
    ParentIsNotApplied,
    /// Error when one migration is replaced by multiple times which is not
    /// supported
    #[error("migration is replaced multiple times")]
    MigrationReplacedMultipleTimes,
    /// Error when migration plan has applied replaces migrations as well as
    /// current migration
    #[error("both replace migrations and current migration are applied")]
    BothMigrationTypeApplied,
    /// Error for irreversible operation
    #[error("operation is irreversible")]
    IrreversibleOperation,
    /// Error for pending migration present
    #[cfg(feature = "cli")]
    #[error("pending migration present")]
    PendingMigrationPresent,
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
    #[cfg(feature = "cli")]
    #[error("applied migrations exists. Revert all using revert subcommand")]
    AppliedMigrationExists,
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
