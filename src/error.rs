#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error(transparent)]
    SqlxError(#[from] sqlx::error::Error),
}
