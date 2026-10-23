//! Running a plan: the [`RunReport`] it returns and the functions applying and
//! reverting migrations while the lock is held

use sqlx::{Connection as _, Database};
use tracing::Instrument as _;

use super::plan::{Plan, PlanType};
use super::{BoxMigration, Migrator, SupportedDatabase};
use crate::error::Error;
use crate::migration::AppliedMigrationSqlRow;

/// Record of a single migration which was applied or reverted by
/// [`Migrator::run`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunRecord {
    app: String,
    name: String,
    elapsed: std::time::Duration,
}

impl RunRecord {
    /// Application name of the migration
    #[must_use]
    pub fn app(&self) -> &str {
        &self.app
    }

    /// Name of the migration
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Time taken to apply or revert the migration, including updating the
    /// migration table
    #[must_use]
    pub fn elapsed(&self) -> std::time::Duration {
        self.elapsed
    }
}

/// Summary returned by [`Migrator::run`] describing what the run did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunReport {
    apply: bool,
    fake: bool,
    records: Vec<RunRecord>,
}

impl RunReport {
    /// Returns `true` when the run applied migrations and `false` when it
    /// reverted them
    #[must_use]
    pub fn is_apply(&self) -> bool {
        self.apply
    }

    /// Returns `true` when the run was fake, i.e. only the migration table
    /// was updated and no operation was executed
    #[must_use]
    pub fn is_fake(&self) -> bool {
        self.fake
    }

    /// Migrations which were applied or reverted, in execution order
    #[must_use]
    pub fn records(&self) -> &[RunRecord] {
        &self.records
    }

    /// Number of migrations which were applied or reverted
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Returns `true` when the run did nothing
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Total time spent across all migrations
    #[must_use]
    pub fn total_elapsed(&self) -> std::time::Duration {
        self.records.iter().map(RunRecord::elapsed).sum()
    }
}

/// Batch number for the migrations a run records, one more than the highest
/// batch in the migration table
pub(super) fn next_batch(applied_rows: &[AppliedMigrationSqlRow]) -> i32 {
    applied_rows
        .iter()
        .map(AppliedMigrationSqlRow::batch)
        .max()
        .unwrap_or(0)
        + 1
}

/// Applies one migration and records it in the migration table. A non atomic
/// migration is recorded as failed before its operations run and marked
/// successful after the last one, so one which fails part way stays recorded
/// as failed instead of being run again on top of what it already changed.
async fn apply_migration<DB>(
    migrator: &Migrator<DB>,
    connection: &mut <DB as Database>::Connection,
    migration: &BoxMigration<DB>,
    fake: bool,
    batch: i32,
) -> Result<(), Error>
where
    DB: SupportedDatabase,
{
    let start_instant = std::time::Instant::now();
    let operations = migration.operations();
    if migration.is_atomic() {
        let mut transaction = connection.begin().await?;
        if !fake {
            migrator
                .before_migration_hooks(&mut transaction, migration.as_ref(), true)
                .await?;
            for operation in operations {
                operation.up(&mut transaction).await?;
            }
        }
        let execution_time = if fake {
            std::time::Duration::ZERO
        } else {
            start_instant.elapsed()
        };
        migrator
            .add_migration_to_db_table(&mut transaction, migration.as_ref(), execution_time, batch)
            .await?;
        if !fake {
            migrator
                .after_migration_hooks(&mut transaction, migration.as_ref(), true)
                .await?;
        }
        transaction.commit().await?;
    } else if fake {
        migrator
            .add_migration_to_db_table(
                connection,
                migration.as_ref(),
                std::time::Duration::ZERO,
                batch,
            )
            .await?;
    } else {
        migrator
            .before_migration_hooks(connection, migration.as_ref(), true)
            .await?;
        migrator
            .start_migration_in_db_table(connection, migration.as_ref(), batch)
            .await?;
        for operation in operations {
            operation.up(connection).await?;
        }
        migrator
            .finish_migration_in_db_table(connection, migration.as_ref(), start_instant.elapsed())
            .await?;
        migrator
            .after_migration_hooks(connection, migration.as_ref(), true)
            .await?;
    }
    Ok(())
}

/// Reverts one migration and removes it from the migration table
async fn revert_migration<DB>(
    migrator: &Migrator<DB>,
    connection: &mut <DB as Database>::Connection,
    migration: &BoxMigration<DB>,
    fake: bool,
) -> Result<(), Error>
where
    DB: SupportedDatabase,
{
    // Reverse operation since last applied operation need
    // to be reverted first
    let mut operations = migration.operations();
    operations.reverse();

    if migration.is_atomic() {
        let mut transaction = connection.begin().await?;
        if !fake {
            migrator
                .before_migration_hooks(&mut transaction, migration.as_ref(), false)
                .await?;
            for operation in operations {
                operation.down(&mut transaction).await?;
            }
        }
        migrator
            .delete_migration_from_db_table(&mut transaction, migration.as_ref())
            .await?;
        if !fake {
            migrator
                .after_migration_hooks(&mut transaction, migration.as_ref(), false)
                .await?;
        }
        transaction.commit().await?;
    } else {
        if !fake {
            migrator
                .before_migration_hooks(connection, migration.as_ref(), false)
                .await?;
            for operation in operations {
                operation.down(connection).await?;
            }
        }
        migrator
            .delete_migration_from_db_table(connection, migration.as_ref())
            .await?;
        if !fake {
            migrator
                .after_migration_hooks(connection, migration.as_ref(), false)
                .await?;
        }
    }
    Ok(())
}

/// Runs the plan while the lock is held. Split out of [`Migrator::run`] so
/// the lock is always released whatever the outcome.
pub(super) async fn run_plan<DB>(
    migrator: &Migrator<DB>,
    connection: &mut <DB as Database>::Connection,
    plan: &Plan,
) -> Result<RunReport, Error>
where
    DB: SupportedDatabase,
{
    let applied_rows = migrator.fetch_applied_migration_from_db(connection).await?;
    if let Some(failed) = applied_rows.iter().find(|row| !row.success()) {
        return Err(Error::FailedMigration {
            app: failed.app().to_string(),
            name: failed.name().to_string(),
        });
    }
    let plan_migrations = migrator.generate_migration_plan(Some(plan), &applied_rows)?;
    // the transaction lock holds the whole plan in one transaction
    if !plan.fake
        && migrator.dialect().lock_strategy().in_transaction()
        && plan_migrations
            .iter()
            .any(|migration| !migration.is_atomic())
    {
        return Err(Error::PlanError {
            message: "the transaction lock runs the plan in one transaction so every migration \
                      must be atomic"
                .to_string(),
        });
    }
    // migrations applied by this run share the next batch number
    let batch = next_batch(&applied_rows);
    let apply = matches!(plan.plan_type, PlanType::Apply);
    migrator.before_run_hooks(connection, apply).await?;
    let mut report = RunReport {
        apply,
        fake: plan.fake,
        records: Vec::with_capacity(plan_migrations.len()),
    };
    if plan_migrations.is_empty() {
        tracing::info!("no migration to run");
    }
    for migration in plan_migrations {
        let migration_span = tracing::info_span!(
            "sqlx_migrator_migration",
            app = migration.app(),
            name = migration.name()
        );
        let elapsed = async {
            let start_instant = std::time::Instant::now();
            match plan.plan_type {
                PlanType::Apply => {
                    tracing::info!("applying migration");
                    apply_migration(migrator, connection, migration, plan.fake, batch).await?;
                }
                PlanType::Revert(_) => {
                    tracing::info!("reverting migration");

                    revert_migration(migrator, connection, migration, plan.fake).await?;
                }
            }
            let elapsed = start_instant.elapsed();
            tracing::info!(elapsed_ns = elapsed.as_nanos(), "migration finished");
            Ok::<_, Error>(elapsed)
        }
        .instrument(migration_span)
        .await?;
        report.records.push(RunRecord {
            app: migration.app().to_string(),
            name: migration.name().to_string(),
            elapsed,
        });
    }
    migrator.after_run_hooks(connection, &report).await?;
    Ok(report)
}
