#![cfg_attr(docsrs, feature(doc_auto_cfg))]

//! Library to create sqlx migration using rust code instead of sql. Visit
//! examples and readme file to learn of how to use sqlx migrator on sqlx
//! project

#[cfg(feature = "cli")]
pub mod cli;
pub mod error;
mod macros;
pub mod migration;
pub mod migrator;
pub mod operation;

pub use sqlx;
