#![cfg_attr(docsrs, feature(doc_auto_cfg))]

//! Library to create sqlx migration using rust code instead of sql.
//!
//! Check `README.MD` for more detailed information of how to use a crate
//! and visit [`Operation`], [`Migration`] and [`Migrator`]

#[cfg(feature = "cli")]
#[doc(inline)]
pub use crate::cli::MigrationCommand;
#[doc(inline)]
pub use crate::error::Error;
#[doc(inline)]
pub use crate::migration::Migration;
#[doc(inline)]
pub use crate::migrator::{Info, Migrate, Migrator, Plan};
#[doc(inline)]
pub use crate::operation::Operation;
#[doc(inline)]
pub use crate::sync::{OldMigrator, Synchronize};

#[cfg(feature = "cli")]
pub mod cli;
pub mod error;
mod macros;
pub mod migration;
pub mod migrator;
pub mod operation;
pub mod sync;
