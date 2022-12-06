#![warn(missing_docs, unreachable_pub)]
#![deny(unsafe_code)]
#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

//! Library to create sqlx migration using rust code instead of sql
#[cfg(feature = "cli")]
pub mod cli;
pub mod error;
pub mod migration;
pub mod migrator;
pub mod operation;

pub use sqlx;
