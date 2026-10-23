//! `heads` subcommand
#![expect(clippy::print_stdout, reason = "allow printing to stdout in cli")]
use clap::Parser;

use super::output::render_table;
use crate::error::Error;
use crate::migrator::{Migrator, SupportedDatabase};

#[derive(Parser, Debug)]
pub(super) struct Heads;

/// Print the heads of every app, then fail when an app has more than one
pub(super) fn run<DB>(migrator: &Migrator<DB>) -> Result<(), Error>
where
    DB: SupportedDatabase,
{
    let rows = migrator
        .heads()?
        .iter()
        .map(|head| vec![head.app().to_string(), head.name().to_string()])
        .collect::<Vec<_>>();
    println!("{}", render_table(&["App", "Head"], &rows));
    migrator.check_heads()
}
