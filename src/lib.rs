//! Library to create sqlx migration using rust files

#[cfg(feature = "clap")]
pub mod clap;
pub mod error;
pub mod migration;
pub mod migrator;
pub mod operation;
#[cfg(feature = "postgres")]
pub mod postgres;
#[cfg(feature = "sqlite")]
pub mod sqlite;
