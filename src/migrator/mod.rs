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
use sqlx_migrator::{Dialect, Migrator};

let migrator = Migrator::<Postgres>::new(Dialect::postgres().set_lock(false));
```
"
)]

use std::collections::{HashMap, HashSet};

use sqlx::{Connection as _, Database};
use tracing::Instrument as _;

pub use self::dialect::{Dialect, SupportedDatabase};
use crate::error::Error;
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

#[derive(Debug)]
enum PlanType {
    Apply,
    Revert,
}

/// Struct that determines the type of migration plan to execute.
///
/// A [`Plan`] can specify whether to apply or revert migrations, and may target
/// all migrations, specific migrations, or a limited number of migrations.
#[derive(Debug)]
pub struct Plan {
    #[expect(
        clippy::struct_field_names,
        reason = "type is a keyword so it cannot be used"
    )]
    plan_type: PlanType,
    app_migration: Option<(String, Option<String>)>,
    count: Option<usize>,
    fake: bool,
}

impl Plan {
    fn new(
        plan_type: PlanType,
        app_migration: Option<(String, Option<String>)>,
        count: Option<usize>,
    ) -> Self {
        Self {
            plan_type,
            app_migration,
            count,
            fake: false,
        }
    }

    /// Sets the plan as a "fake" plan.
    ///
    /// When the plan is marked as fake, the migration status is updated to
    /// either "applied" or "reverted" without actually performing any
    /// migration operations. This is useful for scenarios where you want to
    /// simulate the effect of applying or reverting a migration, but
    /// without making changes to the database.
    ///
    /// By default, the `fake` flag is set to `false`, and the migration
    /// operations are executed as expected.
    #[must_use]
    pub fn fake(mut self, fake: bool) -> Self {
        self.fake = fake;
        self
    }

    /// Creates a new plan to apply all migrations.
    #[must_use]
    pub fn apply_all() -> Self {
        Self::new(PlanType::Apply, None, None)
    }

    /// Creates a new plan to apply a specific migration by name. If migration
    /// name is not provided it will apply all migrations of the app
    #[must_use]
    pub fn apply_name(app: &str, name: Option<&str>) -> Self {
        Self::new(
            PlanType::Apply,
            Some((app.to_string(), name.map(ToString::to_string))),
            None,
        )
    }

    /// Creates a new plan to apply a limited number of migrations.
    #[must_use]
    pub fn apply_count(count: usize) -> Self {
        Self::new(PlanType::Apply, None, Some(count))
    }

    /// Creates a new plan to revert all migrations.
    #[must_use]
    pub fn revert_all() -> Self {
        Self::new(PlanType::Revert, None, None)
    }

    /// Creates a new plan to revert a specific migration by name. If migration
    /// name is not provided it will revert all migrations of the app
    #[must_use]
    pub fn revert_name(app: &str, name: Option<&str>) -> Self {
        Self::new(
            PlanType::Revert,
            Some((app.to_string(), name.map(ToString::to_string))),
            None,
        )
    }

    /// Creates a new plan to revert a limited number of migrations.
    #[must_use]
    pub fn revert_count(count: usize) -> Self {
        Self::new(PlanType::Revert, None, Some(count))
    }
}

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

/// populate replace hash map recursively
fn populate_replace_recursive<'populate, DB>(
    replace_hash_map: &mut HashMap<&'populate BoxMigration<DB>, Vec<&'populate BoxMigration<DB>>>,
    key: &'populate BoxMigration<DB>,
    value: &'populate BoxMigration<DB>,
) -> Result<(), Error> {
    // protect against a case where two migration replaces each other
    if key == value {
        return Err(Error::PlanError {
            message: "two migrations replace each other".to_string(),
        });
    }
    let replace_hash_map_vec = replace_hash_map.entry(key).or_default();
    if !replace_hash_map_vec.contains(&value) {
        replace_hash_map_vec.push(value);
    }
    // Clone only the inner Vec (cheap: Vec of references) to release the
    // mutable borrow on replace_hash_map before the recursive call.
    let grand_values = replace_hash_map.get(value).cloned();
    if let Some(grand_values) = grand_values {
        for grand_value in grand_values {
            populate_replace_recursive(replace_hash_map, key, grand_value)?;
        }
    }
    Ok(())
}

/// Narrows `migration_list` down to only the migrations that are required to
/// apply or revert the specific set of *target* migrations supplied in
/// `with_list`.
///
/// When a user targets a particular app/migration (e.g. `--app foo
/// --migration bar`), the full ordered list already contains every migration in
/// dependency order. This function keeps only the subset that is actually
/// needed to safely apply or revert the requested targets — discarding
/// unrelated migrations from other apps or branches.
fn only_related_migration<DB>(
    migration_list: &mut MigrationVec<'_, DB>,
    with_list: Vec<&BoxMigration<DB>>,
    plan_type: &PlanType,
    replace_children: &HashMap<&BoxMigration<DB>, Vec<&BoxMigration<DB>>>,
) {
    use std::collections::VecDeque;

    // `related` is a membership set only; BFS visitation order is kept by
    // `queue`, so a HashSet keeps the inner-loop `contains` checks O(1).
    let mut related: HashSet<&BoxMigration<DB>> = HashSet::new();
    let mut queue: VecDeque<&BoxMigration<DB>> = VecDeque::new();

    // Seed the BFS with the explicitly requested targets
    for with in with_list {
        if related.insert(with) {
            queue.push_back(with);
        }
    }

    // Expand one hop at a time until no new migrations can be reached.
    while let Some(current) = queue.pop_front() {
        // These depend only on `current`, so compute them once per BFS node
        // instead of once per (current, m) pair. `parents()`/`run_before()`
        // each allocate a fresh Vec, so hoisting avoids repeated allocations.
        let current_parents = current.parents();
        let current_run_before = current.run_before();
        let current_replaced = replace_children.get(current);
        for &m in migration_list.iter() {
            if related.contains(&m) {
                continue;
            }
            let should_include = match plan_type {
                PlanType::Apply => {
                    // m is a direct parent of current, or a parent of any
                    // migration current transitively replaces (inherited edge).
                    let is_parent = current_parents.iter().any(|p| p.as_ref() == m.as_ref())
                        || current_replaced.is_some_and(|children| {
                            children.iter().any(|child| {
                                child.parents().iter().any(|p| p.as_ref() == m.as_ref())
                            })
                        });

                    // current (or any migration it replaces) is in m's
                    // run_before, meaning m must precede
                    // current.
                    let is_run_before = m
                        .run_before()
                        .iter()
                        .any(|rb| rb.as_ref() == current.as_ref())
                        || current_replaced.is_some_and(|children| {
                            children.iter().any(|child| {
                                m.run_before()
                                    .iter()
                                    .any(|rb| rb.as_ref() == child.as_ref())
                            })
                        });

                    is_parent || is_run_before
                }
                PlanType::Revert => {
                    // Collect what m transitively replaces.
                    let m_replaced = replace_children.get(m);

                    // current is a direct parent of m, or a parent of any
                    // migration m transitively replaces (inherited edge).
                    let is_parent = m.parents().iter().any(|p| p.as_ref() == current.as_ref())
                        || m_replaced.is_some_and(|children| {
                            children.iter().any(|child| {
                                child
                                    .parents()
                                    .iter()
                                    .any(|p| p.as_ref() == current.as_ref())
                            })
                        });

                    // m (or any migration m replaces) is in current's
                    // run_before, meaning current runs
                    // before m, so m must be reverted before current.
                    let is_run_before = current_run_before
                        .iter()
                        .any(|rb| rb.as_ref() == m.as_ref())
                        || m_replaced.is_some_and(|children| {
                            children.iter().any(|child| {
                                current_run_before
                                    .iter()
                                    .any(|rb| rb.as_ref() == child.as_ref())
                            })
                        });

                    is_parent || is_run_before
                }
            };
            if should_include {
                related.insert(m);
                queue.push_back(m);
            }
        }
    }

    migration_list.retain(|m| related.contains(m));
}

/// Process plan to provided migrations list
fn process_plan<'process, DB>(
    migration_list: &mut MigrationVec<'process, DB>,
    applied_migrations: &HashSet<&'process BoxMigration<DB>>,
    plan: &Plan,
    replace_children: &HashMap<&BoxMigration<DB>, Vec<&BoxMigration<DB>>>,
    all_migrations: &[BoxMigration<DB>],
) -> Result<(), Error>
where
    DB: Database,
{
    // Modify migration list according to plan type
    match plan.plan_type {
        PlanType::Apply => {
            migration_list.retain(|migration| !applied_migrations.contains(migration));
        }
        PlanType::Revert => {
            migration_list.retain(|migration| applied_migrations.contains(migration));
            migration_list.reverse();
        }
    }

    if let Some((app, migration_name)) = &plan.app_migration {
        // Find position of last migration which matches condition of provided
        // app and migration name. A target that exists in the full migration
        // list but not in the processed list has nothing to do (it is already
        // applied/reverted or replaced), so the plan is empty rather than an
        // error — this keeps targeted apply/revert idempotent. Only a target
        // that doesn't exist at all is an error.
        let position = if let Some(name) = migration_name {
            let pos = migration_list
                .iter()
                .rposition(|migration| migration.app() == app && migration.name() == name);
            if pos.is_none() {
                if !all_migrations
                    .iter()
                    .any(|migration| migration.app() == app)
                {
                    return Err(Error::PlanError {
                        message: format!("app {app} doesn't exist"),
                    });
                }
                if !all_migrations
                    .iter()
                    .any(|migration| migration.app() == app && migration.name() == name)
                {
                    return Err(Error::PlanError {
                        message: format!("migration {name} doesn't exist for app {app}"),
                    });
                }
            }
            pos
        } else {
            let pos = migration_list
                .iter()
                .rposition(|migration| migration.app() == app);
            if pos.is_none()
                && !all_migrations
                    .iter()
                    .any(|migration| migration.app() == app)
            {
                return Err(Error::PlanError {
                    message: format!("app {app} doesn't exist"),
                });
            }
            pos
        };
        let Some(position) = position else {
            migration_list.clear();
            return Ok(());
        };
        migration_list.truncate(position + 1);
        let with_list = if migration_name.is_some() {
            vec![migration_list[position]]
        } else {
            migration_list
                .iter()
                .filter(|pos_migration| pos_migration.app() == app)
                .copied()
                .collect::<Vec<_>>()
        };
        only_related_migration(migration_list, with_list, &plan.plan_type, replace_children);
    } else if let Some(count) = plan.count {
        if count == 0 {
            return Err(Error::PlanError {
                message: "count must be greater than 0".to_string(),
            });
        }
        let actual_len = migration_list.len();
        if count > actual_len {
            return Err(Error::PlanError {
                message: format!(
                    "passed count value is larger than migration length: {actual_len}"
                ),
            });
        }
        migration_list.truncate(count);
    }
    Ok(())
}

/// Resolve a replaced migration to the concrete migration registered in the
/// list.
///
/// A virtual migration is only a placeholder for a real migration sharing the
/// same app/name, so when `child` is virtual the matching non-virtual migration
/// must be located; otherwise `child` itself is returned.
fn resolve_replaced_migration<'resolve, DB>(
    migrations: &'resolve [BoxMigration<DB>],
    child: &'resolve BoxMigration<DB>,
) -> Result<&'resolve BoxMigration<DB>, Error> {
    if child.is_virtual() {
        migrations
            .iter()
            .find(|&search_migration| search_migration == child)
            .ok_or(Error::PlanError {
                message: "Failed finding non virtual migration for virtual migration".to_string(),
            })
    } else {
        Ok(child)
    }
}

// get all replaces migration recursively for a migration
fn get_recursive_replaces<'get, DB>(
    hash_map: &'get HashMap<BoxMigration<DB>, &'get BoxMigration<DB>>,
    val: &'get BoxMigration<DB>,
) -> Vec<&'get BoxMigration<DB>> {
    let mut recursive_vec = vec![val];
    if let Some(&parent) = hash_map.get(val) {
        recursive_vec.extend(get_recursive_replaces(hash_map, parent));
    }
    recursive_vec
}

/// A struct that stores migration-related metadata, including the list of
/// migrations and configuration such as table and schema name
pub struct Migrator<DB> {
    migrations: Vec<BoxMigration<DB>>,
    dialect: Dialect<DB>,
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
        }
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

    /// Record a migration as applied
    ///
    /// # Errors
    /// If the SQL fails
    pub(crate) async fn add_migration_to_db_table(
        &self,
        connection: &mut <DB as Database>::Connection,
        migration: &dyn Migration<DB>,
    ) -> Result<(), Error> {
        DB::execute_bound(
            connection,
            &self.dialect.sql().add_migration,
            &[migration.app(), migration.name()],
        )
        .await
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
        .await
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

    /// Take the dialect's advisory lock, if any, so no two migrators run at
    /// once
    ///
    /// # Errors
    /// If the SQL fails
    pub(crate) async fn lock(
        &self,
        connection: &mut <DB as Database>::Connection,
    ) -> Result<(), Error> {
        if let Some(lock) = self.dialect.lock_statements() {
            DB::execute_bound(connection, lock.acquire(), &[&self.table_name()]).await?;
        }
        Ok(())
    }

    /// Release the advisory lock taken by [`Migrator::lock`]
    ///
    /// # Errors
    /// If the SQL fails
    pub(crate) async fn unlock(
        &self,
        connection: &mut <DB as Database>::Connection,
    ) -> Result<(), Error> {
        if let Some(lock) = self.dialect.lock_statements() {
            DB::execute_bound(connection, lock.release(), &[&self.table_name()]).await?;
        }
        Ok(())
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
    #[expect(clippy::too_many_lines)]
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

        // Hashmap which contains key as child migration and value as parent
        // migration Child migration is migration which is replaced by
        // parent migration. One migration can only be replaced by one
        // migration but one migration can replace multiple migration so
        // we can have multiple child for one parent but one child
        // can only have one parent
        let mut replaces_child_parent_hash_map = HashMap::new();

        for parent_migration in self.migrations() {
            for child_migration in parent_migration.replaces() {
                let child_name = format!("{}:{}", child_migration.app(), child_migration.name());
                if replaces_child_parent_hash_map
                    .insert(child_migration, parent_migration)
                    .is_some()
                {
                    return Err(Error::PlanError {
                        message: format!("migration {child_name} replaced multiple times"),
                    });
                }
            }
        }

        // Hashmap which contains key as migration and value is vector of
        // migration which are children of this migration due to
        // replace. One migration can have multiple children
        let mut replace_children = HashMap::<_, Vec<_>>::new();
        // in first loop add direct children of parent due to replace
        for (child, &parent) in &replaces_child_parent_hash_map {
            let children_migration = resolve_replaced_migration(self.migrations(), child)?;
            replace_children
                .entry(parent)
                .or_default()
                .push(children_migration);
        }
        // in second loop add recursive children of parent due to replace
        for (child, &parent) in &replaces_child_parent_hash_map {
            let children_migration = resolve_replaced_migration(self.migrations(), child)?;
            populate_replace_recursive(&mut replace_children, parent, children_migration)?;
        }

        // Hashmap which contains key as migration and value is vector of
        // migration which should run before this migration. One
        // migration can have multiple run before migration
        let mut run_before_child_parent_hash_map = HashMap::<_, Vec<_>>::new();

        for parent_migration in self.migrations() {
            for run_before_migration in parent_migration.run_before() {
                run_before_child_parent_hash_map
                    .entry(run_before_migration)
                    .or_default()
                    .push(parent_migration);
            }
        }

        let mut migration_list = Vec::new();
        // HashSet to keep track of migration which are already added to
        // migration list to avoid adding same migration multiple times
        // to migration list since one migration can be parent, run
        // before or replace multiple migration
        let mut migration_set = HashSet::new();

        // keep looping until all migration are added to migration list. In each
        // loop check if any migration can be added to migration list or
        // not. A migration can be added if all its parents are already
        // added to migration list
        let original_migration_length = self.migrations().len();
        while migration_list.len() != original_migration_length {
            let loop_initial_migration_list_length = migration_list.len();
            for migration in self.migrations() {
                // check if all parents and run before migration are already
                // added to migration list and if it replaces
                // any migration then that migration
                // should be added to migration list as well before adding this
                // migration to migration list. Also if this
                // migration have children due to replace
                // then their parents and run before should be added to
                // migration list before adding this migration
                // to migration list
                let all_required_added = !migration_set.contains(migration)
                    && migration
                        .parents()
                        .iter()
                        .all(|p| migration_set.contains(p))
                    && run_before_child_parent_hash_map
                        .get(migration)
                        .unwrap_or(&vec![])
                        .iter()
                        .all(|rb| migration_set.contains(rb))
                    && replaces_child_parent_hash_map
                        .get(migration)
                        .is_none_or(|r| migration_set.contains(r))
                    && replace_children.get(migration).is_none_or(|children| {
                        // if children are present then their parents and run
                        // before should be
                        // added to migration list before adding replace
                        // migration
                        children.iter().all(|&child| {
                            child.parents().iter().all(|p| migration_set.contains(p))
                                && run_before_child_parent_hash_map
                                    .get(child)
                                    .unwrap_or(&vec![])
                                    .iter()
                                    .all(|rb| migration_set.contains(rb) || children.contains(rb))
                        })
                    });
                if all_required_added {
                    migration_list.push(migration);
                    migration_set.insert(migration);
                }
            }

            // if no migration is added in this loop then it means there is a
            // deadlock and we cannot proceed further
            if loop_initial_migration_list_length == migration_list.len() {
                return Err(Error::PlanError {
                    message: "reached deadlock stage during plan generation".to_string(),
                });
            }
        }

        // if plan is provided then modify migration list according to plan else
        // return all migration in order of apply
        if let Some(some_plan) = plan {
            // Index the applied rows by (app, name) once so matching each
            // migration is an O(1) lookup rather than an O(rows) rescan, and
            // keep the result in a set for O(1) membership tests below.
            let applied_row_keys: HashSet<(&str, &str)> = applied_migration_sql_rows
                .iter()
                .map(|row| (row.app(), row.name()))
                .collect();
            let mut applied_migrations = HashSet::new();
            for migration in self.migrations() {
                if applied_row_keys.contains(&(migration.app(), migration.name())) {
                    applied_migrations.insert(migration);
                }
            }

            // Check if any child migration is applied before its parent
            // migration according to parents and run before field.
            // If yes then return error. Iterate `self.migrations()`
            // (not the set) so the reported error is deterministic
            // when multiple migrations violate the constraint.
            for migration in self.migrations() {
                if !applied_migrations.contains(&migration) {
                    continue;
                }
                let mut parents = vec![];
                if let Some(run_before_list) = run_before_child_parent_hash_map.get(migration) {
                    for &run_before in run_before_list {
                        parents.push(run_before);
                    }
                }
                let main_parents = migration.parents();
                for parent in &main_parents {
                    parents.push(parent);
                }
                for parent in parents {
                    let recursive_vec =
                        get_recursive_replaces(&replaces_child_parent_hash_map, parent);
                    if !applied_migrations
                        .iter()
                        .any(|applied| recursive_vec.contains(applied))
                    {
                        return Err(Error::PlanError {
                            message: format!(
                                "child migration {}:{} applied before its parent migration {}:{}",
                                migration.app(),
                                migration.name(),
                                parent.app(),
                                parent.name()
                            ),
                        });
                    }
                }
            }

            // Check if any migration and its replaces are applied together or
            // not. Collect all removals first to avoid cloning
            // migration_list.
            let mut to_remove_replacers: Vec<&BoxMigration<DB>> = vec![];
            let mut to_remove_replaced: Vec<&BoxMigration<DB>> = vec![];

            for &migration in &migration_list {
                // Check if this migration have any children due to replace
                if let Some(children) = replace_children.get(&migration) {
                    // Check if any one of replaced migration is applied or not
                    let replaces_applied = children
                        .iter()
                        .any(|&replace_migration| applied_migrations.contains(&replace_migration));

                    // If replaces migration is applied then we cannot apply
                    // this migration If replaces migration
                    // is not applied then we can remove all replaced
                    // migration from migration list since this migration will
                    // apply in place of them
                    if replaces_applied {
                        // Errors out if this migration is also applied since
                        // both migration and its
                        // replaces cannot be applied together
                        if applied_migrations.contains(&migration) {
                            return Err(Error::PlanError {
                                message: format!(
                                    "migration {}:{} and its replaces are applied together",
                                    migration.app(),
                                    migration.name(),
                                ),
                            });
                        }
                        to_remove_replacers.push(migration);
                    } else {
                        // remove all replaced migration from migration list
                        // since this migration will
                        // apply in place of them
                        to_remove_replaced.extend(children);
                    }
                }
            }

            migration_list.retain(|&m| {
                let list_contains_migration =
                    to_remove_replacers.contains(&m) || to_remove_replaced.contains(&m);
                !list_contains_migration
            });

            process_plan(
                &mut migration_list,
                &applied_migrations,
                some_plan,
                &replace_children,
                self.migrations(),
            )?;
        }

        Ok(migration_list)
    }

    /// Ensures no pending migrations are present.
    ///
    /// This is useful as an application startup check to refuse serving until
    /// all migrations are applied.
    ///
    /// # Errors
    /// Returns [`Error::PendingMigrationPresent`] when at least one migration
    /// is pending, or other errors when plan generation fails
    pub async fn check_pending(
        &self,
        connection: &mut <DB as Database>::Connection,
    ) -> Result<(), Error> {
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
        self.lock(connection).await?;
        // store result of pruning so that lock is unlocked before returning
        // result
        let result = async {
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
        let unlock_result = self.unlock(connection).await;
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
        self.lock(connection).await?;
        // Use a block to ensure the lock is released before returning
        let result = async {
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
            for migration in old_migrator_applied_migrations {
                let key = (migration.app(), migration.name());
                if known_migrations.contains(&key) && !already_applied.contains(&key) {
                    self.add_migration_to_db_table(connection, migration.as_ref())
                        .await?;
                }
            }
            Ok(())
        }
        .await;
        // unlock before returning; if both sync and unlock fail, the sync
        // error takes precedence over the unlock error.
        let unlock_result = self.unlock(connection).await;
        result.and(unlock_result)
    }

    /// Run provided plan migrations
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
            self.lock(connection).await?;
            // store result of applying migration so that we can unlock lock
            // before returning result
            let result = run_plan(self, connection, plan).await;
            // unlock before returning; if both migration and unlock fail, the
            // migration error takes precedence over the unlock error.
            let unlock_result = self.unlock(connection).await;
            let report = result.and_then(|report| unlock_result.map(|()| report))?;
            tracing::info!(
                count = report.len(),
                elapsed_ms = report.total_elapsed().as_millis(),
                "run finished"
            );
            Ok(report)
        }
        .instrument(run_span)
        .await
    }
}

/// Runs the plan while the lock is held. Split out of [`Migrator::run`] so
/// the lock is always released whatever the outcome.
async fn run_plan<DB>(
    migrator: &Migrator<DB>,
    connection: &mut <DB as Database>::Connection,
    plan: &Plan,
) -> Result<RunReport, Error>
where
    DB: SupportedDatabase,
{
    let applied_rows = migrator.fetch_applied_migration_from_db(connection).await?;
    let plan_migrations = migrator.generate_migration_plan(Some(plan), &applied_rows)?;
    let mut report = RunReport {
        apply: matches!(plan.plan_type, PlanType::Apply),
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
                    let operations = migration.operations();
                    if migration.is_atomic() {
                        let mut transaction = connection.begin().await?;
                        if !plan.fake {
                            for operation in operations {
                                operation.up(&mut transaction).await?;
                            }
                        }
                        migrator
                            .add_migration_to_db_table(&mut transaction, migration.as_ref())
                            .await?;
                        transaction.commit().await?;
                    } else {
                        if !plan.fake {
                            for operation in operations {
                                operation.up(connection).await?;
                            }
                        }
                        migrator
                            .add_migration_to_db_table(connection, migration.as_ref())
                            .await?;
                    }
                }
                PlanType::Revert => {
                    tracing::info!("reverting migration");

                    // Reverse operation since last applied operation need
                    // to be reverted first
                    let mut operations = migration.operations();
                    operations.reverse();

                    if migration.is_atomic() {
                        let mut transaction = connection.begin().await?;
                        if !plan.fake {
                            for operation in operations {
                                operation.down(&mut transaction).await?;
                            }
                        }
                        migrator
                            .delete_migration_from_db_table(&mut transaction, migration.as_ref())
                            .await?;
                        transaction.commit().await?;
                    } else {
                        if !plan.fake {
                            for operation in operations {
                                operation.down(connection).await?;
                            }
                        }
                        migrator
                            .delete_migration_from_db_table(connection, migration.as_ref())
                            .await?;
                    }
                }
            }
            let elapsed = start_instant.elapsed();
            tracing::info!(elapsed_ms = elapsed.as_millis(), "migration finished");
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
    Ok(report)
}
