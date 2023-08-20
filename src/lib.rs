#![warn(missing_docs, unreachable_pub, unused_crate_dependencies)]
#![deny(unsafe_code)]
#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

//! Library to create sqlx migration using rust code instead of sql. Visit
//! examples and readme file to learn of how to use sqlx migrator on sqlx
//! project

// use serde to handle unused_crate_dependencies issue for serde
use serde as _;

#[cfg(feature = "cli")]
pub mod cli;
pub mod error;
pub mod migration;
pub mod migrator;
pub mod operation;

pub use sqlx;
