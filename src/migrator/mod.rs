//! Migrator module
//!
//! [`Migrator`] plans and runs migrations for [`Postgres`](sqlx::Postgres),
//! [`Sqlite`](sqlx::Sqlite) and [`MySql`](sqlx::MySql) when the matching
//! feature is enabled. Its configuration, such as table prefix, schema and
//! lock, is a [`Dialect`] value which can be adjusted with setters.
#![cfg_attr(
    feature = "postgres",
    doc = r"
# Example
Postgres migrator with no lock

```rust,no_run
use sqlx::Postgres;
use sqlx_migrator::migrator::LockStrategy;
use sqlx_migrator::{Dialect, Migrator};

let dialect = Dialect::postgres().set_lock_strategy(LockStrategy::None).unwrap();
let migrator = Migrator::<Postgres>::new(dialect);
```
"
)]
#![doc = include_str!("../../docs/migrator.md")]

use std::collections::{HashMap, HashSet};

use sqlx::{Connection as _, Database};
use tracing::Instrument as _;

pub use self::dialect::{Dialect, LockStrategy, SupportedDatabase};
pub use self::plan::Plan;
use self::plan::PlanType;
use self::relations::Relations;
pub use self::run::{RunRecord, RunReport};
use self::run::{next_batch, run_plan};
use crate::error::Error;
use crate::hook::{BoxHook, HookContext, HookFuture, HookType};
use crate::migration::{AppliedMigrationSqlRow, Migration};
use crate::sync::OldMigrator;

/// Module holding backend SQL as data and executing it generically
mod dialect;

/// Module for the internal migrations of the migrator tables
mod internal_migration;

/// Module for mysql
#[cfg(feature = "mysql")]
mod mysql;

/// Module for sqlite
#[cfg(feature = "sqlite")]
mod sqlite;

/// Module for postgres
#[cfg(feature = "postgres")]
mod postgres;

/// Module for ordering migrations and narrowing them down to a plan
mod relations;

/// Module for the plan of a run
mod plan;

/// Module for running a plan
mod run;

/// Module for testing
#[cfg(feature = "sqlite")]
#[cfg(test)]
mod tests;

type BoxMigration<DB> = Box<dyn Migration<DB>>;
type MigrationVec<'migration, DB> = Vec<&'migration BoxMigration<DB>>;
type MigrationVecResult<'migration, DB> = Result<MigrationVec<'migration, DB>, Error>;
type StatusVec<'migration, 'row, DB> = Vec<(
    &'migration BoxMigration<DB>,
    Option<&'row AppliedMigrationSqlRow>,
)>;

/// What a lock is held on
enum Held<'connection, DB: Database> {
    /// The connection, the lock is released by statements or nothing was taken
    Connection(&'connection mut <DB as Database>::Connection),
    /// A transaction which holds the lock until it ends
    Transaction(sqlx::Transaction<'connection, DB>),
}

/// A lock taken by [`Migrator::lock`]
pub(crate) struct Locked<'connection, DB: Database>(Held<'connection, DB>);

impl<DB: Database> Locked<'_, DB> {
    /// Connection to run what needs the lock on
    pub(crate) fn connection(&mut self) -> &mut <DB as Database>::Connection {
        match &mut self.0 {
            Held::Connection(connection) => connection,
            Held::Transaction(transaction) => transaction,
        }
    }
}

/// A struct that stores migration-related metadata, including the list of
/// migrations and configuration such as table and schema name
pub struct Migrator<DB: Database> {
    migrations: Vec<BoxMigration<DB>>,
    dialect: Dialect<DB>,
    hooks: Vec<(HookType, BoxHook<DB>)>,
}

impl<DB> Migrator<DB>
where
    DB: SupportedDatabase,
{
    /// Creates a migrator configured by `dialect`. `Migrator::default` uses
    /// [`Dialect::postgres`], [`Dialect::mysql`] or [`Dialect::sqlite`]; pass
    /// an adjusted one to change its lock, DDL behaviour, table prefix or
    /// schema, for example no lock behind a transaction pooling proxy such as
    /// `PgBouncer`.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature="sqlite")]
    /// # fn main() {
    /// use sqlx_migrator::{Dialect, Migrator};
    /// let dialect = Dialect::sqlite_with(Some("v1"), None).unwrap();
    /// let migrator = Migrator::<sqlx::Sqlite>::new(dialect);
    /// assert_eq!(migrator.table_name(), "_v1_sqlx_migrator_migrations");
    /// # }
    /// # #[cfg(not(feature="sqlite"))]
    /// # fn main() {}
    /// ```
    #[must_use]
    pub fn new(dialect: Dialect<DB>) -> Self {
        Self {
            migrations: Vec::default(),
            dialect,
            hooks: Vec::default(),
        }
    }

    /// Adds a function which runs at the point of a run given by `hook_type`,
    /// see [`HookType`]. The function gets the connection doing the work and a
    /// [`HookContext`]. Functions of the same type run in the order they were
    /// added, and one which fails stops the run.
    ///
    /// The function returns a boxed future so it can borrow the connection,
    /// which a plain `fn` or a closure does by wrapping its body in
    /// `Box::pin(async move { .. })`.
    pub fn add_hook(
        &mut self,
        hook_type: HookType,
        function: impl for<'a> Fn(
            &'a mut <DB as Database>::Connection,
            &'a HookContext<'a, DB>,
        ) -> HookFuture<'a>
        + Send
        + Sync
        + 'static,
    ) {
        self.hooks.push((hook_type, Box::new(function)));
    }

    async fn run_hooks(
        &self,
        hook_type: HookType,
        connection: &mut <DB as Database>::Connection,
        context: &HookContext<'_, DB>,
    ) -> Result<(), Error> {
        for (_, function) in self.hooks.iter().filter(|(kind, _)| *kind == hook_type) {
            function(connection, context).await?;
        }
        Ok(())
    }

    async fn before_run_hooks(
        &self,
        connection: &mut <DB as Database>::Connection,
        apply: bool,
    ) -> Result<(), Error> {
        let context = HookContext::new(apply, None, None);
        self.run_hooks(HookType::BeforeRun, connection, &context)
            .await
    }

    async fn after_run_hooks(
        &self,
        connection: &mut <DB as Database>::Connection,
        report: &RunReport,
    ) -> Result<(), Error> {
        let context = HookContext::new(report.is_apply(), None, Some(report));
        self.run_hooks(HookType::AfterRun, connection, &context)
            .await
    }

    async fn before_migration_hooks(
        &self,
        connection: &mut <DB as Database>::Connection,
        migration: &dyn Migration<DB>,
        apply: bool,
    ) -> Result<(), Error> {
        let context = HookContext::new(apply, Some(migration), None);
        self.run_hooks(HookType::BeforeMigration, connection, &context)
            .await
    }

    async fn after_migration_hooks(
        &self,
        connection: &mut <DB as Database>::Connection,
        migration: &dyn Migration<DB>,
        apply: bool,
    ) -> Result<(), Error> {
        let context = HookContext::new(apply, Some(migration), None);
        self.run_hooks(HookType::AfterMigration, connection, &context)
            .await
    }

    /// Dialect holding the configuration and SQL the migrator runs
    #[must_use]
    pub fn dialect(&self) -> &Dialect<DB> {
        &self.dialect
    }

    /// Returns the migrations added so far
    #[must_use]
    pub fn migrations(&self) -> &[BoxMigration<DB>] {
        &self.migrations
    }

    /// Adds a list of migrations to the migrator.
    ///
    /// This method accepts a vector of migrations and adds each one
    /// individually to ensure proper handling of migration relationships
    /// and duplicates.
    ///
    /// # Errors
    /// If migration is added with same app and name but inconsistent value i.e
    /// its parents, run before, replaces and is atomic differ and do not have
    /// same number of operation
    pub fn add_migrations(&mut self, migrations: Vec<BoxMigration<DB>>) -> Result<(), Error> {
        for migration in migrations {
            self.add_migration(migration)?;
        }
        Ok(())
    }

    /// Adds a single migration to the migrator.
    ///
    /// # Errors
    /// If migration is added with same app and name but inconsistent value i.e
    /// its parents, run before, replaces and is atomic differ and do not have
    /// same number of operation
    pub fn add_migration(&mut self, migration: BoxMigration<DB>) -> Result<(), Error> {
        if migration.is_virtual() {
            // Virtual migrations must not have any fields other than app/name
            if !migration.parents().is_empty()
                || !migration.operations().is_empty()
                || !migration.replaces().is_empty()
                || !migration.run_before().is_empty()
            {
                return Err(Error::InvalidVirtualMigration);
            }
            // Only add virtual migration if not already present
            if self.migrations.contains(&migration) {
                return Ok(());
            }
            self.migrations.push(migration);
        } else {
            let mut skip_add = false;
            if let Some((migration_index, found_migration)) = self
                .migrations()
                .iter()
                .enumerate()
                .find(|(_, elem)| elem == &&migration)
            {
                if found_migration.is_virtual() {
                    // Replace the placeholder virtual migration with the
                    // concrete one
                    self.migrations.remove(migration_index);
                } else if found_migration.parents() != migration.parents()
                    || found_migration.operations().len() != migration.operations().len()
                    || found_migration.replaces() != migration.replaces()
                    || found_migration.run_before() != migration.run_before()
                    || found_migration.is_atomic() != migration.is_atomic()
                {
                    // Non-virtual duplicate with different definition
                    return Err(Error::InconsistentMigration {
                        app: migration.app().to_string(),
                        name: migration.name().to_string(),
                    });
                } else {
                    // Non-virtual consistent duplicate — already registered
                    skip_add = true;
                }
            }

            if !skip_add {
                // Add this migration and recursively register its parents,
                // replaces, and run_before migrations
                let migration_parents = migration.parents();
                let migration_replaces = migration.replaces();
                let migration_run_before = migration.run_before();

                self.migrations.push(migration);

                for parent in migration_parents {
                    self.add_migration(parent)?;
                }
                for replace in migration_replaces {
                    self.add_migration(replace)?;
                }
                for run_before in migration_run_before {
                    self.add_migration(run_before)?;
                }
            }
        }
        Ok(())
    }

    /// Get name of table which is used for storing migrations related
    /// information in database
    ///
    /// Format depends on the prefix and schema of the dialect:
    /// - With schema: `{schema}._sqlx_migrator_migrations`
    /// - With prefix: `_{prefix}_sqlx_migrator_migrations`
    /// - With both: `{schema}._{prefix}_sqlx_migrator_migrations`
    /// - Default: `_sqlx_migrator_migrations`
    ///
    /// # Examples
    /// ```rust
    /// # #[cfg(feature="sqlite")]
    /// # fn main() {
    /// use sqlx_migrator::{Dialect, Migrator};
    /// let dialect = Dialect::sqlite_with(Some("v1"), Some("app_schema")).unwrap();
    /// let migrator = Migrator::<sqlx::Sqlite>::new(dialect);
    /// assert_eq!(
    ///     migrator.table_name(),
    ///     "app_schema._v1_sqlx_migrator_migrations"
    /// );
    /// # }
    /// # #[cfg(not(feature="sqlite"))]
    /// # fn main() {}
    /// ```
    #[must_use]
    pub fn table_name(&self) -> String {
        self.dialect.table_name()
    }

    /// Get name of the meta table which records the migrator's internal
    /// migrations. It follows the same prefix and schema rules as
    /// [`Migrator::table_name`].
    ///
    /// # Examples
    /// ```rust
    /// # #[cfg(feature="sqlite")]
    /// # fn main() {
    /// use sqlx_migrator::{Dialect, Migrator};
    /// let dialect = Dialect::sqlite_with(Some("v1"), None).unwrap();
    /// let migrator = Migrator::<sqlx::Sqlite>::new(dialect);
    /// assert_eq!(migrator.meta_table_name(), "_v1_sqlx_migrator_meta");
    /// # }
    /// # #[cfg(not(feature="sqlite"))]
    /// # fn main() {}
    /// ```
    #[must_use]
    pub fn meta_table_name(&self) -> String {
        self.dialect.meta_table_name()
    }

    /// Ensure the migration table exists, creating or upgrading it with the
    /// dialect's internal migrations
    ///
    /// # Errors
    /// If the SQL fails or the meta table records an internal migration this
    /// crate does not know
    pub(crate) async fn ensure_migration_table_exists(
        &self,
        connection: &mut <DB as Database>::Connection,
    ) -> Result<(), Error> {
        let sql = self.dialect.sql();
        if let Some(create_schema) = self.dialect.create_schema_statement() {
            DB::execute(connection, create_schema).await?;
        }
        internal_migration::ensure_tables::<DB>(connection, &sql.internal).await
    }

    /// Drop the migration and meta tables if they exist
    ///
    /// # Errors
    /// If the SQL fails
    pub async fn drop_migration_table_if_exists(
        &self,
        connection: &mut <DB as Database>::Connection,
    ) -> Result<(), Error> {
        DB::execute(connection, &self.dialect.sql().drop_tables).await
    }

    /// Record a migration as applied. `execution_time` is zero when no
    /// operation ran
    ///
    /// # Errors
    /// If the SQL fails
    pub(crate) async fn add_migration_to_db_table(
        &self,
        connection: &mut <DB as Database>::Connection,
        migration: &dyn Migration<DB>,
        execution_time: std::time::Duration,
        batch: i32,
    ) -> Result<(), Error> {
        DB::execute_bound(
            connection,
            &self.dialect.sql().add_migration,
            &[
                migration.app(),
                migration.name(),
                &execution_time.as_nanos().to_string(),
                &batch.to_string(),
            ],
        )
        .await?;
        Ok(())
    }

    /// Record a migration which is about to run as failed. It is marked
    /// successful by [`Migrator::finish_migration_in_db_table`], so a
    /// migration which fails part way stays recorded as failed
    ///
    /// # Errors
    /// If the SQL fails
    pub(crate) async fn start_migration_in_db_table(
        &self,
        connection: &mut <DB as Database>::Connection,
        migration: &dyn Migration<DB>,
        batch: i32,
    ) -> Result<(), Error> {
        DB::execute_bound(
            connection,
            &self.dialect.sql().start_migration,
            &[migration.app(), migration.name(), &batch.to_string()],
        )
        .await?;
        Ok(())
    }

    /// Mark a migration recorded by [`Migrator::start_migration_in_db_table`]
    /// as successful
    ///
    /// # Errors
    /// If the SQL fails
    pub(crate) async fn finish_migration_in_db_table(
        &self,
        connection: &mut <DB as Database>::Connection,
        migration: &dyn Migration<DB>,
        execution_time: std::time::Duration,
    ) -> Result<(), Error> {
        DB::execute_bound(
            connection,
            &self.dialect.sql().finish_migration,
            &[
                &execution_time.as_nanos().to_string(),
                migration.app(),
                migration.name(),
            ],
        )
        .await?;
        Ok(())
    }

    /// Remove a migration from the applied ones
    ///
    /// # Errors
    /// If the SQL fails
    pub(crate) async fn delete_migration_from_db_table(
        &self,
        connection: &mut <DB as Database>::Connection,
        migration: &dyn Migration<DB>,
    ) -> Result<(), Error> {
        DB::execute_bound(
            connection,
            &self.dialect.sql().delete_migration,
            &[migration.app(), migration.name()],
        )
        .await?;
        Ok(())
    }

    /// Fetch the applied migrations, creating or upgrading the migration
    /// table first
    ///
    /// # Errors
    /// If the SQL fails or the meta table records an internal migration this
    /// crate does not know
    pub async fn fetch_applied_migration_from_db(
        &self,
        connection: &mut <DB as Database>::Connection,
    ) -> Result<Vec<AppliedMigrationSqlRow>, Error> {
        self.ensure_migration_table_exists(connection).await?;
        DB::fetch_rows(connection, &self.dialect.sql().fetch_rows).await
    }

    /// Take the lock of the dialect's [`LockStrategy`]
    ///
    /// # Errors
    /// If the SQL fails or the lock row is held by another run
    pub(crate) async fn lock<'connection>(
        &self,
        connection: &'connection mut <DB as Database>::Connection,
    ) -> Result<Locked<'connection, DB>, Error> {
        match self.dialect.lock_strategy() {
            LockStrategy::None => {}
            LockStrategy::Advisory => {
                if let Some(lock) = self.dialect.advisory_lock() {
                    DB::execute_bound(connection, lock.acquire(), &[&self.table_name()]).await?;
                }
            }
            LockStrategy::Row => self.take_row_lock(connection).await?,
            strategy @ (LockStrategy::TransactionAdvisory
            | LockStrategy::TableShareUpdateExclusive
            | LockStrategy::TableAccessExclusive) => {
                return self.begin_locked_transaction(connection, strategy).await;
            }
        }
        Ok(Locked(Held::Connection(connection)))
    }

    /// Release the lock taken by [`Migrator::lock`]. A transaction lock commits
    /// its transaction when `succeeded` and rolls it back otherwise
    ///
    /// # Errors
    /// If the SQL fails or the transaction cannot be committed
    pub(crate) async fn unlock(
        &self,
        locked: Locked<'_, DB>,
        succeeded: bool,
    ) -> Result<(), Error> {
        match locked.0 {
            Held::Transaction(transaction) => {
                if succeeded {
                    transaction.commit().await?;
                } else {
                    transaction.rollback().await?;
                }
            }
            Held::Connection(connection) => {
                match self.dialect.lock_strategy() {
                    LockStrategy::Advisory => {
                        if let Some(lock) = self.dialect.advisory_lock() {
                            DB::execute_bound(connection, lock.release(), &[&self.table_name()])
                                .await?;
                        }
                    }
                    LockStrategy::Row => {
                        DB::execute_bound(
                            connection,
                            &self.dialect.sql().row_lock.release,
                            &[&self.table_name()],
                        )
                        .await?;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    /// Begin the transaction holding the lock of a transaction lock strategy
    async fn begin_locked_transaction<'connection>(
        &self,
        connection: &'connection mut <DB as Database>::Connection,
        strategy: LockStrategy,
    ) -> Result<Locked<'connection, DB>, Error> {
        let Some(locks) = &self.dialect.sql().transaction_locks else {
            return Err(Error::UnsupportedLockStrategy { strategy });
        };
        let mut transaction = connection.begin().await?;
        // the advisory lock keeps two nodes from creating the tables at once,
        // which a table lock cannot since the table has to exist to be locked
        DB::execute_bound(&mut transaction, &locks.advisory, &[&self.table_name()]).await?;
        DB::execute(&mut transaction, &locks.after_lock).await?;
        let table_lock = match strategy {
            LockStrategy::TableShareUpdateExclusive => Some(&locks.share_update_exclusive),
            LockStrategy::TableAccessExclusive => Some(&locks.access_exclusive),
            _ => None,
        };
        if let Some(table_lock) = table_lock {
            self.ensure_migration_table_exists(&mut transaction).await?;
            DB::execute(&mut transaction, table_lock).await?;
        }
        Ok(Locked(Held::Transaction(transaction)))
    }

    /// Insert the lock row
    async fn take_row_lock(
        &self,
        connection: &mut <DB as Database>::Connection,
    ) -> Result<(), Error> {
        let sql = self.dialect.sql();
        if let Some(create_schema) = self.dialect.create_schema_statement() {
            DB::execute(connection, create_schema).await?;
        }
        // two processes creating the table at once may fail one of them, the
        // statement taking the lock then reports a table which is missing
        if let Err(error) = DB::execute(connection, &sql.row_lock.create_table).await {
            tracing::debug!(%error, "creating the lock table failed");
        }
        match DB::execute_bound(connection, &sql.row_lock.acquire, &[&self.table_name()]).await {
            Err(Error::Sqlx(sqlx::Error::Database(error))) if error.is_unique_violation() => {
                Err(Error::LockHeld {
                    table: self.dialect.lock_table_name(),
                })
            }
            result => result,
        }
    }

    /// Drop the lock table of [`LockStrategy::Row`] if it exists. Call it
    /// after [`Migrator::drop_migration_table_if_exists`] once nothing is
    /// locked, since the table holds the row of the lock
    ///
    /// # Errors
    /// If the SQL fails
    pub async fn drop_lock_table_if_exists(
        &self,
        connection: &mut <DB as Database>::Connection,
    ) -> Result<(), Error> {
        DB::execute(connection, &self.dialect.sql().drop_lock_table).await
    }

    /// The heads of every app in migration list order
    ///
    /// # Errors
    /// If a migration refers to one which is not in the migration list, is
    /// replaced twice or is replaced by a migration it replaces
    pub fn heads(&self) -> Result<Vec<&BoxMigration<DB>>, Error> {
        let relations = Relations::new(self.migrations())?;
        let mut heads = relations.heads();
        heads.sort_unstable();
        Ok(heads
            .into_iter()
            .map(|index| &self.migrations()[index])
            .collect())
    }

    /// Ensures every app has one head. Two heads mean two migrations were
    /// written next to each other without one depending on the other.
    ///
    /// # Errors
    /// [`Error::MultipleHeads`] for the first app with more than one head, or
    /// the errors of [`Migrator::heads`]
    pub fn check_heads(&self) -> Result<(), Error> {
        let mut apps = Vec::<(&str, Vec<&str>)>::new();
        for head in self.heads()? {
            match apps.iter_mut().find(|(app, _)| *app == head.app()) {
                Some((_, names)) => names.push(head.name()),
                None => apps.push((head.app(), vec![head.name()])),
            }
        }
        match apps.into_iter().find(|(_, names)| names.len() > 1) {
            Some((app, names)) => {
                Err(Error::MultipleHeads {
                    app: app.to_string(),
                    heads: names.into_iter().map(ToString::to_string).collect(),
                })
            }
            None => Ok(()),
        }
    }

    /// Generate migration plan using pre-fetched applied migration rows.
    ///
    /// This is the core plan-generation logic that performs **no database
    /// access**. It is useful when you have already called
    /// [`Migrator::fetch_applied_migration_from_db`] (e.g., to
    /// display status alongside the plan) and want to avoid a redundant
    /// database round-trip.
    ///
    /// If `plan` is `None`, returns all migrations in apply order. If `plan`
    /// is `Some`, processes the plan using the supplied
    /// `applied_migration_sql_rows`.
    ///
    /// # Errors
    /// Returns an error if the migration list is empty, contains unresolved
    /// virtual migrations, has a dependency deadlock, or if the plan
    /// references an app/migration that does not exist.
    pub fn generate_migration_plan(
        &self,
        plan: Option<&Plan>,
        applied_migration_sql_rows: &[AppliedMigrationSqlRow],
    ) -> MigrationVecResult<'_, DB> {
        if self.migrations().is_empty() {
            return Err(Error::PlanError {
                message: "no migrations are added to the migration list".to_string(),
            });
        }
        // if there is any virtual migration which is not replaced then return
        // error since virtual migration should only be used for replacing
        // another migration
        if self
            .migrations()
            .iter()
            .any(|migration| migration.is_virtual())
        {
            return Err(Error::PlanError {
                message: "virtual migration which is not replaced is present".to_string(),
            });
        }

        tracing::debug!("generating {:?} migration plan", plan);

        let relations = Relations::new(self.migrations())?;
        let order = relations.order()?;

        // if plan is provided then modify migration list according to plan else
        // return all migration in order of apply
        let order = if let Some(some_plan) = plan {
            let applied_keys: HashSet<(&str, &str)> = applied_migration_sql_rows
                .iter()
                .map(|row| (row.app(), row.name()))
                .collect();
            let applied = self
                .migrations()
                .iter()
                .map(|migration| applied_keys.contains(&(migration.app(), migration.name())))
                .collect::<Vec<_>>();
            let last_batch_keys: HashSet<(&str, &str)> = match &some_plan.plan_type {
                PlanType::Revert(revert_plan) if revert_plan.last_batch => {
                    let last_batch = applied_migration_sql_rows
                        .iter()
                        .map(AppliedMigrationSqlRow::batch)
                        .max();
                    applied_migration_sql_rows
                        .iter()
                        .filter(|row| Some(row.batch()) == last_batch)
                        .map(|row| (row.app(), row.name()))
                        .collect()
                }
                _ => HashSet::new(),
            };
            relations.plan(order, &applied, some_plan, &last_batch_keys)?
        } else {
            order
        };

        Ok(order
            .into_iter()
            .map(|index| &self.migrations()[index])
            .collect())
    }

    /// Ensures no pending migrations are present.
    ///
    /// This is useful as an application startup check to refuse serving until
    /// all migrations are applied.
    ///
    /// # Errors
    /// Returns [`Error::PendingMigrationPresent`] when at least one migration
    /// is pending, [`Error::MultipleHeads`] when an app has more than one head,
    /// or other errors when plan generation fails
    pub async fn check_pending(
        &self,
        connection: &mut <DB as Database>::Connection,
    ) -> Result<(), Error> {
        self.check_heads()?;
        let applied_rows = self.fetch_applied_migration_from_db(connection).await?;
        let pending = self.generate_migration_plan(Some(&Plan::apply_all()), &applied_rows)?;
        if !pending.is_empty() {
            return Err(Error::PendingMigrationPresent);
        }
        Ok(())
    }

    /// Generate migration status using pre-fetched applied migration rows.
    ///
    /// This is the core status logic which performs **no database access**.
    /// It is useful when you have already called
    /// [`Migrator::fetch_applied_migration_from_db`] for another
    /// purpose and want to avoid a redundant database round-trip. Returns
    /// every migration in apply order paired with a reference to its applied
    /// migration row when applied, or an empty list when no migrations are
    /// added.
    ///
    /// # Errors
    /// If plan generation fails
    pub fn status<'get>(
        &self,
        applied_migration_sql_rows: &'get [AppliedMigrationSqlRow],
    ) -> Result<StatusVec<'_, 'get, DB>, Error> {
        if self.migrations().is_empty() {
            return Ok(vec![]);
        }
        let applied_by_key = applied_migration_sql_rows
            .iter()
            .map(|row| ((row.app(), row.name()), row))
            .collect::<HashMap<_, _>>();
        Ok(self
            .generate_migration_plan(None, &[])?
            .into_iter()
            .map(|migration| {
                let applied = applied_by_key
                    .get(&(migration.app(), migration.name()))
                    .copied();
                (migration, applied)
            })
            .collect())
    }

    /// Prune the migration table by deleting applied migration entries which
    /// no longer have a corresponding migration in the migration list.
    ///
    /// This is useful for cleaning up the migration table after migrations
    /// are removed from the codebase, since such entries would otherwise
    /// linger in the migration table forever. Returns the number of pruned
    /// entries.
    ///
    /// # Errors
    /// If fetching or deleting applied migrations fails
    pub async fn prune(
        &self,
        connection: &mut <DB as Database>::Connection,
    ) -> Result<usize, Error> {
        tracing::debug!("pruning unknown applied migrations");
        let mut locked = self.lock(connection).await?;
        // store result of pruning so that lock is unlocked before returning
        // result
        let result = async {
            let connection = locked.connection();
            let applied_migrations = self.fetch_applied_migration_from_db(connection).await?;
            let known_migrations = self
                .migrations()
                .iter()
                .map(|migration| (migration.app(), migration.name()))
                .collect::<HashSet<_>>();
            let mut pruned_count = 0;
            for row in &applied_migrations {
                if !known_migrations.contains(&(row.app(), row.name())) {
                    let unknown_migration = (row.app().to_string(), row.name().to_string());
                    self.delete_migration_from_db_table(connection, &unknown_migration)
                        .await?;
                    pruned_count += 1;
                }
            }
            Ok(pruned_count)
        }
        .await;
        // unlock before returning; if both prune and unlock fail, the prune
        // error takes precedence over the unlock error.
        let unlock_result = self.unlock(locked, result.is_ok()).await;
        result.and_then(|count| unlock_result.map(|()| count))
    }

    /// Removes the migrations which started running and never finished from
    /// the migration table, so they run again. Returns the number of removed
    /// entries.
    ///
    /// Only the migration table is changed: the operations of a failed
    /// migration ran partly, so the database has to be fixed by hand first.
    ///
    /// # Errors
    /// If fetching or deleting failed migrations fails
    pub async fn repair(
        &self,
        connection: &mut <DB as Database>::Connection,
    ) -> Result<usize, Error> {
        tracing::debug!("repairing failed migrations");
        let mut locked = self.lock(connection).await?;
        // store result of repairing so that lock is unlocked before returning
        // result
        let result = async {
            let connection = locked.connection();
            let applied_migrations = self.fetch_applied_migration_from_db(connection).await?;
            let failed_count = applied_migrations
                .iter()
                .filter(|row| !row.success())
                .count();
            if failed_count > 0 {
                DB::execute(connection, &self.dialect.sql().delete_failed_migrations).await?;
            }
            Ok(failed_count)
        }
        .await;
        // unlock before returning; if both repair and unlock fail, the repair
        // error takes precedence over the unlock error.
        let unlock_result = self.unlock(locked, result.is_ok()).await;
        result.and_then(|count| unlock_result.map(|()| count))
    }

    /// Records the migrations an old migrator reports as applied, skipping
    /// ones this migrator does not know or which are already recorded.
    /// Applied times are not preserved. Use it to move from another
    /// migration system or to rename the migration table.
    ///
    /// # Errors
    /// If sync cannot be completed
    pub async fn sync<O>(
        &self,
        connection: &mut <DB as Database>::Connection,
        old_migrator: &O,
    ) -> Result<(), Error>
    where
        O: OldMigrator<DB>,
    {
        tracing::debug!("syncing old migrator");
        let mut locked = self.lock(connection).await?;
        // Use a block to ensure the lock is released before returning
        let result = async {
            let connection = locked.connection();
            let old_migrator_applied_migrations =
                old_migrator.applied_migrations(connection).await?;
            let already_applied_migration =
                self.fetch_applied_migration_from_db(connection).await?;
            let known_migrations = self
                .migrations
                .iter()
                .map(|migration| (migration.app(), migration.name()))
                .collect::<HashSet<_>>();
            let already_applied = already_applied_migration
                .iter()
                .map(|row| (row.app(), row.name()))
                .collect::<HashSet<_>>();
            // synced migrations are recorded as one batch
            let batch = next_batch(&already_applied_migration);
            for migration in old_migrator_applied_migrations {
                let key = (migration.app(), migration.name());
                if known_migrations.contains(&key) && !already_applied.contains(&key) {
                    self.add_migration_to_db_table(
                        connection,
                        migration.as_ref(),
                        std::time::Duration::ZERO,
                        batch,
                    )
                    .await?;
                }
            }
            Ok(())
        }
        .await;
        // unlock before returning; if both sync and unlock fail, the sync
        // error takes precedence over the unlock error.
        let unlock_result = self.unlock(locked, result.is_ok()).await;
        result.and(unlock_result)
    }

    /// Run provided plan migrations. Fails with [`Error::MultipleHeads`] before
    /// anything runs when an app has more than one head, see
    /// [`Migrator::check_heads`].
    ///
    /// Returns a [`RunReport`] listing every migration which was applied or
    /// reverted along with the time each one took. Progress is emitted as
    /// `tracing` events at `info` level inside a span per run and per
    /// migration.
    ///
    /// # Errors
    /// If failed to run provided plan migrations
    pub async fn run(
        &self,
        connection: &mut <DB as Database>::Connection,
        plan: &Plan,
    ) -> Result<RunReport, Error> {
        let run_span = tracing::info_span!(
            "sqlx_migrator_run",
            plan_type = ?plan.plan_type,
            fake = plan.fake
        );
        async {
            tracing::debug!("running plan {:?}", plan);
            self.check_heads()?;
            let mut locked = self.lock(connection).await?;
            // store result of applying migration so that we can unlock lock
            // before returning result
            let result = run_plan(self, locked.connection(), plan).await;
            // unlock before returning; if both migration and unlock fail, the
            // migration error takes precedence over the unlock error.
            let unlock_result = self.unlock(locked, result.is_ok()).await;
            let report = result.and_then(|report| unlock_result.map(|()| report))?;
            tracing::info!(
                count = report.len(),
                elapsed_ns = report.total_elapsed().as_nanos(),
                "run finished"
            );
            Ok(report)
        }
        .instrument(run_span)
        .await
    }
}
