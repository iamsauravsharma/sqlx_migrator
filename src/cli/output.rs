//! Output helpers shared by the subcommands: formats, tables and prompts
#![expect(clippy::print_stdout, reason = "allow printing to stdout in cli")]
use std::io::{IsTerminal as _, Write as _};

use clap::ValueEnum;
#[cfg(feature = "json")]
use serde::Serialize;

use crate::error::Error;
use crate::migration::Migration;

/// Output format of commands which print lists
#[derive(ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum OutputFormat {
    /// Human readable table
    #[default]
    Table,
    /// JSON array, one object per migration. Available with the `json`
    /// feature
    #[cfg(feature = "json")]
    Json,
}

/// Render rows as a table whose column widths fit the content. Cells are
/// centered and columns separated by ` | `, with a dashed line below the
/// header.
pub(super) fn render_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let widths = headers
        .iter()
        .enumerate()
        .map(|(column, header)| {
            rows.iter()
                .map(|row| row.get(column).map_or(0, |cell| cell.chars().count()))
                .chain(std::iter::once(header.chars().count()))
                .max()
                .unwrap_or_default()
        })
        .collect::<Vec<_>>();
    let render_row = |cells: &[&str]| {
        cells
            .iter()
            .zip(&widths)
            .map(|(cell, width)| format!("{cell:^width$}"))
            .collect::<Vec<_>>()
            .join(" | ")
    };
    let full_width = widths.iter().sum::<usize>() + widths.len().saturating_sub(1) * 3;
    let mut output = render_row(headers);
    output.push('\n');
    output.push_str(&"-".repeat(full_width));
    for row in rows {
        output.push('\n');
        let cells = row.iter().map(String::as_str).collect::<Vec<_>>();
        output.push_str(&render_row(&cells));
    }
    output
}

/// A migration in a plan, as printed by `apply --plan` and `revert --plan`
#[cfg_attr(feature = "json", derive(Serialize))]
struct PlanEntry<'a> {
    app: &'a str,
    name: &'a str,
}

/// Render a plan in the requested format. `empty_message` is used for the
/// table format when the plan has no migration.
#[cfg_attr(
    not(feature = "json"),
    expect(
        clippy::unnecessary_wraps,
        reason = "only JSON serialization can fail; keep one signature"
    )
)]
pub(super) fn render_plan<'migration, DB>(
    migrations: impl IntoIterator<Item = &'migration dyn Migration<DB>>,
    format: OutputFormat,
    empty_message: &str,
) -> Result<String, Error>
where
    DB: 'migration,
{
    let entries = migrations
        .into_iter()
        .map(|migration| {
            PlanEntry {
                app: migration.app(),
                name: migration.name(),
            }
        })
        .collect::<Vec<_>>();
    match format {
        #[cfg(feature = "json")]
        OutputFormat::Json => Ok(serde_json::to_string_pretty(&entries)?),
        OutputFormat::Table if entries.is_empty() => Ok(empty_message.to_string()),
        OutputFormat::Table => {
            let rows = entries
                .iter()
                .map(|entry| vec![entry.app.to_string(), entry.name.to_string()])
                .collect::<Vec<_>>();
            Ok(render_table(&["App", "Name"], &rows))
        }
    }
}

/// Ask the user to confirm an action listing the affected migrations. Returns
/// `Ok(false)` when the user declines.
///
/// # Errors
/// When stdin is not a terminal so no answer can be read
pub(super) fn confirm(question: &str, items: &[(&str, &str)]) -> Result<bool, Error> {
    if !std::io::stdin().is_terminal() {
        return Err(Error::ConfirmationRequired);
    }
    println!("{question} (y/N)");
    for (position, (app, name)) in items.iter().enumerate() {
        println!("{}. {app} : {name}", position + 1);
    }
    std::io::stdout().flush()?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let input_trimmed = input.trim().to_ascii_lowercase();
    Ok(["y", "yes"].contains(&input_trimmed.as_str()))
}
