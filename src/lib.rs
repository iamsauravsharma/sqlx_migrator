#![cfg_attr(docsrs, feature(doc_cfg))]

//! Library to create sqlx migration using rust code instead of sql.
//!
//! The guide below walks through [`Operation`], [`Migration`] and
//! [`Migrator`]. See the [`hook`], [`cli`] and [`sync`] modules for hooks, the
//! command line and moving from another migration tool.
#![doc = include_str!("../docs/guide.md")]

#[cfg(feature = "cli")]
#[doc(inline)]
pub use crate::cli::MigrationCommand;
#[doc(inline)]
pub use crate::error::Error;
#[doc(inline)]
pub use crate::migration::Migration;
#[doc(inline)]
pub use crate::migrator::{Dialect, Migrator, Plan};
#[doc(inline)]
pub use crate::operation::Operation;

#[cfg(feature = "cli")]
pub mod cli;
pub mod error;
pub mod hook;
mod macros;
pub mod migration;
pub mod migrator;
pub mod operation;
pub mod sync;
