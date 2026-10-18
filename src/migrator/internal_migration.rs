//! Internal migrations: how the migrator creates and changes its own
//! migration table.
//!
//! Each backend builds them as a `(name, sql)` list in order. The meta table
//! records the names which have run, so only new ones run, and a name this
//! release does not know is rejected. Entries are never edited, only appended
//! to. The first entry is safe to rerun, later ones run exactly once.

use std::collections::HashSet;

use sqlx::Database;

use super::dialect::SupportedDatabase;
use crate::error::Error;

/// Internal migrations of a backend with table names filled in
#[derive(Debug, Clone)]
pub(crate) struct InternalMigrations {
    create_meta_table: String,
    fetch_applied: String,
    insert_applied: String,
    steps: Vec<(&'static str, String)>,
}

impl InternalMigrations {
    /// `insert_applied` binds the name, `fetch_applied` selects the names,
    /// `steps` are the internal migrations as `(name, sql)` in order
    pub(crate) fn new(
        create_meta_table: String,
        fetch_applied: String,
        insert_applied: String,
        steps: Vec<(&'static str, String)>,
    ) -> Self {
        Self {
            create_meta_table,
            fetch_applied,
            insert_applied,
            steps,
        }
    }

    /// Internal migrations in order as `(name, sql)`
    #[cfg(test)]
    pub(crate) fn steps(&self) -> &[(&'static str, String)] {
        &self.steps
    }

    /// Names in order
    pub(crate) fn names(&self) -> Vec<&'static str> {
        self.steps.iter().map(|(name, _)| *name).collect()
    }
}

/// Creates the meta table when missing and runs the internal migrations not
/// yet recorded, recording each one.
pub(crate) async fn ensure_tables<DB>(
    connection: &mut DB::Connection,
    internal: &InternalMigrations,
) -> Result<(), Error>
where
    DB: Database + SupportedDatabase,
{
    DB::execute(connection, &internal.create_meta_table).await?;

    let applied: HashSet<String> = fetch_applied::<DB>(connection, internal)
        .await?
        .into_iter()
        .collect();
    let pending = internal
        .steps
        .iter()
        .filter(|(name, _)| !applied.contains(*name));

    for (name, sql) in pending {
        tracing::info!(name, "running internal migration");
        DB::execute(connection, sql).await?;
        DB::execute_bound(connection, &internal.insert_applied, &[name]).await?;
    }
    Ok(())
}

/// Recorded internal migration names in order.
///
/// # Errors
/// When a recorded name is unknown, meaning a newer release changed the table
pub(crate) async fn fetch_applied<DB>(
    connection: &mut DB::Connection,
    internal: &InternalMigrations,
) -> Result<Vec<String>, Error>
where
    DB: Database + SupportedDatabase,
{
    let mut applied = DB::fetch_text(connection, &internal.fetch_applied).await?;
    applied.sort();

    let known = internal.names();
    let unknown = applied.iter().find(|name| !known.contains(&name.as_str()));
    match unknown {
        Some(name) => Err(Error::UnknownInternalMigration { name: name.clone() }),
        None => Ok(applied),
    }
}
