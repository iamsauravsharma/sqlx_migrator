//! Migrator module
//!
//! It contains common enum and trait for implementing migrator for sqlx
//! supported database
//!
//! It also provides its own struct [`Migrator`] which supports
//! [`Any`](sqlx::Any), [`Postgres`](sqlx::Postgres), [`Sqlite`](sqlx::Sqlite)
//! and [`MySql`](sqlx::MySql) database when corresponding feature is enabled
#![cfg_attr(
    feature = "postgres",
    doc = r#"
# Example
Create own custom Migrator which only supports postgres and uses own unique
table name instead of default table name

```rust,no_run
use sqlx::{Database, Pool, Postgres};
use sqlx_migrator::error::Error;
use sqlx_migrator::migration::{AppliedMigrationSqlRow, Migration};
use sqlx_migrator::migrator::{DatabaseOperation, Info, Migrate};
use sqlx_migrator::sync::Synchronize;

#[derive(Default)]
pub struct CustomMigrator {
    migrations: Vec<Box<dyn Migration<Postgres>>>,
}

impl Info<Postgres> for CustomMigrator {
    fn migrations(&self) -> &[Box<dyn Migration<Postgres>>] {
        &self.migrations
    }

    fn migrations_mut(&mut self) -> &mut Vec<Box<dyn Migration<Postgres>>> {
        &mut self.migrations
    }
}

#[async_trait::async_trait]
impl DatabaseOperation<Postgres> for CustomMigrator {
    async fn ensure_migration_table_exists(
        &self,
        connection: &mut <Postgres as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS _custom_table_name (
        id INT PRIMARY KEY NOT NULL GENERATED ALWAYS AS IDENTITY,
        app TEXT NOT NULL,
        name TEXT NOT NULL,
        applied_time TIMESTAMPTZ NOT NULL DEFAULT now(),
        UNIQUE (app, name)
    )",
        )
        .execute(connection)
        .await?;
        Ok(())
    }

    async fn drop_migration_table_if_exists(
        &self,
        connection: &mut <Postgres as Database>::Connection,
    ) -> Result<(), Error> {
        sqlx::query("DROP TABLE IF EXISTS _custom_table_name")
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn add_migration_to_db_table(
        &self,
        connection: &mut <Postgres as Database>::Connection,
        migration: &dyn Migration<Postgres>,
    ) -> Result<(), Error> {
        sqlx::query("INSERT INTO _custom_table_name(app, name) VALUES ($1, $2)")
            .bind(migration.app())
            .bind(migration.name())
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn delete_migration_from_db_table(
        &self,
        connection: &mut <Postgres as Database>::Connection,
        migration: &dyn Migration<Postgres>,
    ) -> Result<(), Error> {
        sqlx::query("DELETE FROM _custom_table_name WHERE app = $1 AND name = $2")
            .bind(migration.app())
            .bind(migration.name())
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn fetch_applied_migration_from_db(
        &self,
        connection: &mut <Postgres as Database>::Connection,
    ) -> Result<Vec<AppliedMigrationSqlRow>, Error> {
        Ok(sqlx::query_as::<_, AppliedMigrationSqlRow>(
            "SELECT id, app, name, applied_time FROM _custom_table_name",
        )
        .fetch_all(connection)
        .await?)
    }

    async fn lock(
        &self,
        connection: &mut <Postgres as Database>::Connection,
    ) -> Result<(), Error> {
        let (database_name,): (String,) = sqlx::query_as("SELECT CURRENT_DATABASE()")
            .fetch_one(&mut *connection)
            .await?;
        let lock_id = i64::from(crc32fast::hash(database_name.as_bytes()));
        sqlx::query("SELECT pg_advisory_lock($1)")
            .bind(lock_id)
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn unlock(
        &self,
        connection: &mut <Postgres as Database>::Connection,
    ) -> Result<(), Error> {
        let (database_name,): (String,) = sqlx::query_as("SELECT CURRENT_DATABASE()")
            .fetch_one(&mut *connection)
            .await?;
        let lock_id = i64::from(crc32fast::hash(database_name.as_bytes()));
        sqlx::query("SELECT pg_advisory_unlock($1)")
            .bind(lock_id)
            .execute(connection)
            .await?;
        Ok(())
    }
}
impl Migrate<Postgres> for CustomMigrator {}
impl Synchronize<Postgres> for CustomMigrator {}
```
"#
)]

use std::collections::{HashMap, HashSet};

use sqlx::{Connection as _, Database};

use crate::error::Error;
use crate::migration::{AppliedMigrationSqlRow, Migration};

/// Any database module which support mysql, sqlite and postgres by default
#[cfg(all(
    any(feature = "postgres", feature = "mysql", feature = "sqlite"),
    feature = "any"
))]
mod any;

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
    pub fn fake(self, fake: bool) -> Self {
        let mut plan = self;
        plan.fake = fake;
        plan
    }

    /// Creates a new plan to apply all migrations.
    #[must_use]
    pub fn apply_all() -> Self {
        Self::new(PlanType::Apply, None, None)
    }

    /// Creates a new plan to apply a specific migration by name. If migration
    /// name is not provided it will apply app all migrations
    #[must_use]
    pub fn apply_name(app: &str, name: &Option<String>) -> Self {
        Self::new(PlanType::Apply, Some((app.to_string(), name.clone())), None)
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
    /// name is not provided it will revert app all migrations
    #[must_use]
    pub fn revert_name(app: &str, name: &Option<String>) -> Self {
        Self::new(
            PlanType::Revert,
            Some((app.to_string(), name.clone())),
            None,
        )
    }

    /// Creates a new plan to revert a limited number of migrations.
    #[must_use]
    pub fn revert_count(count: usize) -> Self {
        Self::new(PlanType::Revert, None, Some(count))
    }
}

/// The [`Info`] trait provides database-agnostic methods for managing
/// migrations and interacting with migration states.
pub trait Info<DB> {
    /// Returns a reference to the list of migrations.
    fn migrations(&self) -> &[BoxMigration<DB>];

    /// Returns a mutable reference to the list of migrations.
    fn migrations_mut(&mut self) -> &mut Vec<BoxMigration<DB>>;

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
    fn add_migrations(&mut self, migrations: Vec<BoxMigration<DB>>) -> Result<(), Error> {
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
    fn add_migration(&mut self, migration: BoxMigration<DB>) -> Result<(), Error> {
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
            if self.migrations().contains(&migration) {
                return Ok(());
            }
            self.migrations_mut().push(migration);
        } else {
            let mut skip_add = false;
            if let Some((migration_index, found_migration)) = self
                .migrations()
                .iter()
                .enumerate()
                .find(|(_, elem)| elem == &&migration)
            {
                if found_migration.is_virtual() {
                    // Replace the placeholder virtual migration with the concrete one
                    self.migrations_mut().remove(migration_index);
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
                // Add this migration and recursively register its parents, replaces,
                // and run_before migrations
                let migration_parents = migration.parents();
                let migration_replaces = migration.replaces();
                let migration_run_before = migration.run_before();

                self.migrations_mut().push(migration);

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
}

/// The [`DatabaseOperation`] trait defines a set of methods for performing
/// operations related to migration management on the database.
///
/// This trait is typically implemented for a database to support migration
/// operations, such as ensuring the migration table exists, adding or
/// removing migrations from the table, and locking the database during
/// migration processes.
///
/// # Security
/// The methods in this trait receive a `table_name` (via
/// [`Info::migrations`] and [`Migrator::table_name`]) that is interpolated
/// directly into SQL strings. The built-in [`Migrator`] implementation
/// validates the table name prefix and schema before use, so the risk is
/// mitigated for default users. **Custom implementations must ensure that
/// `table_name` is derived exclusively from validated output of
/// [`Migrator::table_name`]** and is never constructed from untrusted
/// sources (e.g., config files, CLI arguments, environment variables),
/// or they will be vulnerable to SQL injection.
#[async_trait::async_trait]
pub trait DatabaseOperation<DB>
where
    DB: Database,
{
    /// Ensure migration table is created before running migrations. If not
    /// create one
    async fn ensure_migration_table_exists(
        &self,
        connection: &mut <DB as Database>::Connection,
    ) -> Result<(), Error>;

    /// Drop migration table if migration table exists
    async fn drop_migration_table_if_exists(
        &self,
        connection: &mut <DB as Database>::Connection,
    ) -> Result<(), Error>;

    /// Adds a migration record to the migration table in the database.
    async fn add_migration_to_db_table(
        &self,
        connection: &mut <DB as Database>::Connection,
        migration: &dyn Migration<DB>,
    ) -> Result<(), Error>;

    /// Removes a migration record from the migration table in the database.
    async fn delete_migration_from_db_table(
        &self,
        connection: &mut <DB as Database>::Connection,
        migration: &dyn Migration<DB>,
    ) -> Result<(), Error>;

    /// Fetches the list of applied migrations from the migration table in the
    /// database.
    async fn fetch_applied_migration_from_db(
        &self,
        connection: &mut <DB as Database>::Connection,
    ) -> Result<Vec<AppliedMigrationSqlRow>, Error>;

    /// Lock database while doing migrations so no two migrations run together
    async fn lock(&self, connection: &mut <DB as Database>::Connection) -> Result<(), Error>;

    /// Unlock locked database
    async fn unlock(&self, connection: &mut <DB as Database>::Connection) -> Result<(), Error>;
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
            message: "two migrations replaces each other".to_string(),
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
///
/// ## Algorithm — BFS expansion
///
/// The related set is seeded with the explicit target migrations. A BFS queue
/// is then used to expand the set one hop at a time: whenever a migration is
/// added to the related set it is enqueued, and when it is dequeued every
/// migration in `migration_list` is checked to see whether it is directly
/// linked to the just-dequeued migration. Transitivity is handled naturally:
/// a migration added in one hop is itself dequeued and can pull in further
/// migrations.
///
/// A migration `M` is pulled in when processing the already-included
/// migration `C` if:
///
/// ### Apply direction
/// - `M` is a **direct parent** of `C`, OR a parent of any migration that `C`
///   transitively **replaces** (replacement-inherited parent), OR
/// - `C` (or any migration `C` transitively replaces) is in `M`'s **direct
///   `run_before`** list — making `M` an implicit prerequisite.
///
/// ### Revert direction
/// - `C` is a **direct parent** of `M`, OR a parent of any migration that `M`
///   transitively **replaces** (replacement-inherited parent), OR
/// - `M` (or any migration `M` transitively replaces) is in `C`'s **direct
///   `run_before`** list — `C` runs before `M`, so reverting `C` requires `M`
///   to be reverted first.
///
/// Replacement-inherited edges are necessary because the ordering phase places
/// a replacer after all of its replaced migrations' prerequisites. A targeted
/// plan must therefore pull in those same prerequisites.
///
/// Because `PartialEq` for `dyn Migration` compares only `(app, name)`,
/// virtual tuple references and concrete struct references are resolved
/// transparently without needing a separate lookup table.
///
/// ## Note
/// This function assumes that `replaces` relationships have already been
/// resolved before it is called — replaced migrations will not appear in
/// `migration_list` at this point.
fn only_related_migration<DB>(
    migration_list: &mut MigrationVec<'_, DB>,
    with_list: Vec<&BoxMigration<DB>>,
    plan_type: &PlanType,
    replace_children: &HashMap<&BoxMigration<DB>, Vec<&BoxMigration<DB>>>,
) {
    use std::collections::VecDeque;

    let mut related: Vec<&BoxMigration<DB>> = vec![];
    let mut queue: VecDeque<&BoxMigration<DB>> = VecDeque::new();

    // Seed the BFS with the explicitly requested targets (deduplicated).
    for with in with_list {
        if !related.contains(&with) {
            related.push(with);
            queue.push_back(with);
        }
    }

    // Expand one hop at a time until no new migrations can be reached.
    while let Some(current) = queue.pop_front() {
        for &m in migration_list.iter() {
            if related.contains(&m) {
                continue;
            }
            let should_include = match plan_type {
                PlanType::Apply => {
                    // Collect the migrations that `current` transitively replaces so that
                    // replacement-inherited parent/run_before edges can be checked below.
                    let current_replaced = replace_children.get(current);

                    // m is a direct parent of current, or a parent of any
                    // migration current transitively replaces (inherited edge).
                    let is_parent = current.parents().iter().any(|p| p.as_ref() == m.as_ref())
                        || current_replaced.is_some_and(|children| {
                            children.iter().any(|child| {
                                child.parents().iter().any(|p| p.as_ref() == m.as_ref())
                            })
                        });

                    // current (or any migration it replaces) is in m's run_before,
                    // meaning m must precede current.
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

                    // m (or any migration m replaces) is in current's run_before,
                    // meaning current runs before m, so m must be reverted before current.
                    let is_run_before = current
                        .run_before()
                        .iter()
                        .any(|rb| rb.as_ref() == m.as_ref())
                        || m_replaced.is_some_and(|children| {
                            children.iter().any(|child| {
                                current
                                    .run_before()
                                    .iter()
                                    .any(|rb| rb.as_ref() == child.as_ref())
                            })
                        });

                    is_parent || is_run_before
                }
            };
            if should_include {
                related.push(m);
                queue.push_back(m);
            }
        }
    }

    migration_list.retain(|m| related.contains(m));
}

/// Process plan to provided migrations list
fn process_plan<DB>(
    migration_list: &mut MigrationVec<'_, DB>,
    applied_migrations: &MigrationVec<'_, DB>,
    plan: &Plan,
    replace_children: &HashMap<&BoxMigration<DB>, Vec<&BoxMigration<DB>>>,
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
        // Find position of last migration which matches condition of provided app and
        // migration name
        let position = if let Some(name) = migration_name {
            let Some(pos) = migration_list
                .iter()
                .rposition(|migration| migration.app() == app && migration.name() == name)
            else {
                if migration_list
                    .iter()
                    .any(|migration| migration.app() == app)
                {
                    return Err(Error::PlanError {
                        message: format!("migration {app}:{name} doesn't exists for app"),
                    });
                }
                return Err(Error::PlanError {
                    message: format!("app {app} doesn't exists"),
                });
            };
            pos
        } else {
            let Some(pos) = migration_list
                .iter()
                .rposition(|migration| migration.app() == app)
            else {
                return Err(Error::PlanError {
                    message: format!("app {app} doesn't exists"),
                });
            };
            pos
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

/// The [`Migrate`] trait defines methods to manage and apply database
/// migrations according to a given plan.
///
/// This trait combines the functionalities of the [`Info`] and
/// [`DatabaseOperation`] traits, providing a full set of migration
/// capabilities. All methods have default implementations, meaning no explicit
/// implementation is required. Additionally, all methods are database-agnostic.
#[async_trait::async_trait]
pub trait Migrate<DB>: Info<DB> + DatabaseOperation<DB> + Send + Sync
where
    DB: Database,
{
    /// Generate migration plan using pre-fetched applied migration rows.
    ///
    /// This is the core plan-generation logic that performs **no database
    /// access**. It is useful when you have already called
    /// [`DatabaseOperation::fetch_applied_migration_from_db`] (e.g., to
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
    fn generate_migration_plan_with_rows(
        &self,
        plan: Option<&Plan>,
        applied_migration_sql_rows: &[AppliedMigrationSqlRow],
    ) -> MigrationVecResult<'_, DB> {
        if self.migrations().is_empty() {
            return Err(Error::PlanError {
                message: "no migration are added to migration list".to_string(),
            });
        }
        // if there is any virtual migration which is not replaced than return
        // error since virtual migration should only be used for replacing
        // another migration
        if self
            .migrations()
            .iter()
            .any(|migration| migration.is_virtual())
        {
            return Err(Error::PlanError {
                message: "virtual migrations which is not replaced is present".to_string(),
            });
        }

        tracing::debug!("generating {:?} migration plan", plan);

        // hashmap which contains key as child migration and value is parent
        // migration which replaces this child migration. One migration can
        // only have one parent
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

        // Hashmap which contains key as migration and value is vector of migration
        // which are children of this migration due to replace. One migration can
        // have multiple children
        let mut replace_children = HashMap::<_, Vec<_>>::new();
        // in first loop add direct children of parent due to replace
        for (child, &parent) in &replaces_child_parent_hash_map {
            // if child is virtual than we need to find non virtual migration
            let children_migration = if child.is_virtual() {
                self.migrations()
                    .iter()
                    .find(|&search_migration| search_migration == child)
                    .ok_or(Error::PlanError {
                        message: "Failed finding non virtual migration for virtual migration"
                            .to_string(),
                    })?
            } else {
                child
            };
            replace_children
                .entry(parent)
                .or_default()
                .push(children_migration);
        }
        // in second loop add recursive children of parent due to replace
        for (child, &parent) in &replaces_child_parent_hash_map {
            let children_migration = if child.is_virtual() {
                self.migrations()
                    .iter()
                    .find(|&search_migration| search_migration == child)
                    .ok_or(Error::PlanError {
                        message: "Failed finding non virtual migration for virtual migration"
                            .to_string(),
                    })?
            } else {
                child
            };
            populate_replace_recursive(&mut replace_children, parent, children_migration)?;
        }
        // Hashmap which contains key as migration and value is vector of migration
        // which should run before this migration. One migration can have
        // multiple run before migration
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
        // HashSet tracks which migrations are already in migration_list by
        // (app, name), giving O(1) membership checks in the inner loop and
        // reducing overall plan-generation complexity from O(n³) to O(n²).
        let mut migration_set: HashSet<(String, String)> =
            HashSet::with_capacity(self.migrations().len());

        // keep looping until all migration are added to migration list. In each loop
        // check if any migration can be added to migration list or not. A migration
        // can be added if all its parents are already added to migration list
        let original_migration_length = self.migrations().len();
        while migration_list.len() != original_migration_length {
            let loop_initial_migration_list_length = migration_list.len();
            for migration in self.migrations() {
                let migration_key = (migration.app().to_string(), migration.name().to_string());
                // check if all parents and run before migration are already added to
                // migration list and if it replaces any migration than that migration
                // should be added to migration list as well before adding this migration
                // to migration list. Also if this migration have children due to replace
                // than their parents and run before should be added to migration list
                // before adding this migration to migration list
                let all_required_added = !migration_set.contains(&migration_key)
                    && migration.parents().iter().all(|p| {
                        migration_set.contains(&(p.app().to_string(), p.name().to_string()))
                    })
                    && run_before_child_parent_hash_map
                        .get(migration)
                        .unwrap_or(&vec![])
                        .iter()
                        .all(|rb| {
                            migration_set.contains(&(rb.app().to_string(), rb.name().to_string()))
                        })
                    && replaces_child_parent_hash_map
                        .get(migration)
                        .is_none_or(|r| {
                            migration_set.contains(&(r.app().to_string(), r.name().to_string()))
                        })
                    && replace_children.get(migration).is_none_or(|children| {
                        // if children are present than their parents and run before should be
                        // added to migration list before adding replace migration
                        children.iter().all(|&child| {
                            child.parents().iter().all(|p| {
                                migration_set.contains(&(p.app().to_string(), p.name().to_string()))
                            }) && run_before_child_parent_hash_map
                                .get(child)
                                .unwrap_or(&vec![])
                                .iter()
                                .all(|rb| {
                                    migration_set
                                        .contains(&(rb.app().to_string(), rb.name().to_string()))
                                        || children.contains(rb)
                                })
                        })
                    });
                if all_required_added {
                    migration_list.push(migration);
                    migration_set.insert(migration_key);
                }
            }

            // if no migration is added in this loop than it means there is a deadlock
            // and we cannot proceed further
            if loop_initial_migration_list_length == migration_list.len() {
                return Err(Error::PlanError {
                    message: "reached deadlock stage during plan generation".to_string(),
                });
            }
        }

        // if plan is provided than modify migration list according to plan else
        // return all migration in order of apply
        if let Some(some_plan) = plan {
            let mut applied_migrations = Vec::new();
            for migration in self.migrations() {
                if applied_migration_sql_rows
                    .iter()
                    .any(|sqlx_migration| sqlx_migration == migration)
                {
                    applied_migrations.push(migration);
                }
            }

            // Check if any child migration is applied before its parent migration
            // according to parents and run before field. If yes than return error
            for &migration in &applied_migrations {
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
                                "children migration {}:{} applied before its parent migration \
                                 {}:{}",
                                migration.app(),
                                migration.name(),
                                parent.app(),
                                parent.name()
                            ),
                        });
                    }
                }
            }

            // Check if any migration and its replaces are applied together or not.
            // Collect all removals first to avoid cloning migration_list.
            let mut to_remove_replacers: Vec<&BoxMigration<DB>> = vec![];
            let mut to_remove_replaced: Vec<&BoxMigration<DB>> = vec![];

            for &migration in &migration_list {
                // Check if this migration have any children due to replace
                if let Some(children) = replace_children.get(&migration) {
                    // Check if any one of replaced migration is applied or not
                    let replaces_applied = children
                        .iter()
                        .any(|&replace_migration| applied_migrations.contains(&replace_migration));

                    // If replaces migration is applied than we cannot apply this migration
                    // If replaces migration is not applied than we can remove all replaced
                    // migration from migration list since this migration will apply in
                    // place of them
                    if replaces_applied {
                        // Errors out if this migration is also applied since both
                        // migration and its replaces cannot be applied together
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
                        // remove all replaced migration from migration list since this
                        // migration will apply in place of them
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
            )?;
        }

        Ok(migration_list)
    }

    /// Generate migration plan according to plan.
    ///
    /// Returns a vector of migrations. If `plan` is `None`, returns all
    /// migrations in apply order without accessing the database. If `plan` is
    /// `Some`, calls [`DatabaseOperation::ensure_migration_table_exists`] and
    /// [`DatabaseOperation::fetch_applied_migration_from_db`] before delegating
    /// to [`Migrate::generate_migration_plan_with_rows`].
    ///
    /// If you have already fetched the applied rows for another purpose (e.g.,
    /// displaying status), call [`Migrate::generate_migration_plan_with_rows`]
    /// directly to avoid a redundant database round-trip.
    async fn generate_migration_plan(
        &self,
        connection: &mut <DB as Database>::Connection,
        plan: Option<&Plan>,
    ) -> MigrationVecResult<'_, DB> {
        if plan.is_some() {
            self.ensure_migration_table_exists(connection).await?;
            let rows = self.fetch_applied_migration_from_db(connection).await?;
            self.generate_migration_plan_with_rows(plan, &rows)
        } else {
            self.generate_migration_plan_with_rows(plan, &[])
        }
    }

    /// Run provided plan migrations
    ///
    /// # Errors
    /// If failed to run provided plan migrations
    async fn run(
        &self,
        connection: &mut <DB as Database>::Connection,
        plan: &Plan,
    ) -> Result<(), Error> {
        tracing::debug!("running plan {:?}", plan);
        self.lock(connection).await?;
        // store result of applying migration so that we can unlock lock before
        // returning result
        let result = async {
            for migration in self.generate_migration_plan(connection, Some(plan)).await? {
                match plan.plan_type {
                    PlanType::Apply => {
                        tracing::debug!("applying {} : {}", migration.app(), migration.name());
                        let operations = migration.operations();
                        if migration.is_atomic() {
                            let mut transaction = connection.begin().await?;
                            if !plan.fake {
                                for operation in operations {
                                    operation.up(&mut transaction).await?;
                                }
                            }
                            self.add_migration_to_db_table(&mut transaction, migration.as_ref())
                                .await?;
                            transaction.commit().await?;
                        } else {
                            if !plan.fake {
                                for operation in operations {
                                    operation.up(connection).await?;
                                }
                            }
                            self.add_migration_to_db_table(connection, migration.as_ref())
                                .await?;
                        }
                    }
                    PlanType::Revert => {
                        tracing::debug!("reverting {} : {}", migration.app(), migration.name());

                        // Reverse operation since last applied operation need to be reverted first
                        let mut operations = migration.operations();
                        operations.reverse();

                        if migration.is_atomic() {
                            let mut transaction = connection.begin().await?;
                            if !plan.fake {
                                for operation in operations {
                                    operation.down(&mut transaction).await?;
                                }
                            }
                            self.delete_migration_from_db_table(
                                &mut transaction,
                                migration.as_ref(),
                            )
                            .await?;
                            transaction.commit().await?;
                        } else {
                            if !plan.fake {
                                for operation in operations {
                                    operation.down(connection).await?;
                                }
                            }
                            self.delete_migration_from_db_table(connection, migration.as_ref())
                                .await?;
                        }
                    }
                }
            }
            Ok(())
        }
        .await;
        // unlock before returning; if both migration and unlock fail, the
        // migration error takes precedence over the unlock error.
        let unlock_result = self.unlock(connection).await;
        result.and(unlock_result)
    }
}

const DEFAULT_TABLE_NAME: &str = "_sqlx_migrator_migrations";

/// A struct that stores migration-related metadata, including the list of
/// migrations and configuration such as table and schema name
pub struct Migrator<DB> {
    migrations: Vec<BoxMigration<DB>>,
    table_prefix: Option<String>,
    schema: Option<String>,
}

impl<DB> Migrator<DB> {
    /// Creates a new migrator
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature="sqlite")]
    /// # fn main() {
    /// let migrator = sqlx_migrator::Migrator::<sqlx::Sqlite>::new();
    /// assert_eq!(&migrator.table_name(), "_sqlx_migrator_migrations")
    /// # }
    /// # #[cfg(not(feature="sqlite"))]
    /// # fn main() {
    /// # }
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            migrations: Vec::default(),
            table_prefix: None,
            schema: None,
        }
    }

    /// Configures a prefix for the migrator table name.
    ///
    /// The table name will be formatted as
    /// `_{prefix}_sqlx_migrator_migrations`. Only ASCII lowercase, numeric
    /// characters and underscores are allowed in the prefix.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature="sqlite")]
    /// # fn main() {
    /// let migrator = sqlx_migrator::Migrator::<sqlx::Sqlite>::new()
    ///     .set_table_prefix("prefix_value")
    ///     .unwrap();
    /// assert_eq!(
    ///     &migrator.table_name(),
    ///     "_prefix_value_sqlx_migrator_migrations"
    /// )
    /// # }
    /// # #[cfg(not(feature="sqlite"))]
    /// # fn main() {
    /// # }
    /// ```
    ///
    /// # Errors
    /// When passed table prefix name contains invalid characters
    pub fn set_table_prefix(mut self, prefix: impl Into<String>) -> Result<Self, Error> {
        let prefix_str = prefix.into();
        if prefix_str.is_empty()
            || !prefix_str
                .chars()
                .all(|c| char::is_ascii_lowercase(&c) || char::is_numeric(c) || c == '_')
        {
            return Err(Error::InvalidTablePrefix);
        }
        self.table_prefix = Some(prefix_str);
        Ok(self)
    }

    /// Configures a schema for the migrator table.
    ///
    /// When set, the table name will be formatted as `{schema}.{table_name}`.
    /// Schema name can only contain [a-z0-9_] and begin with [a-z_]
    ///
    /// # Examples
    /// ```rust
    /// # #[cfg(feature="sqlite")]
    /// # fn main() {
    /// let migrator = sqlx_migrator::Migrator::<sqlx::Sqlite>::new()
    ///     .set_schema("migrations")
    ///     .unwrap();
    /// assert_eq!(
    ///     &migrator.table_name(),
    ///     "migrations._sqlx_migrator_migrations"
    /// );
    /// # }
    /// # #[cfg(not(feature="sqlite"))]
    /// # fn main() {}
    /// ```
    ///
    /// # Errors
    /// When passed schema name contains invalid characters
    pub fn set_schema(mut self, schema: impl Into<String>) -> Result<Self, Error> {
        let schema_str = schema.into();
        if schema_str.is_empty()
            || !schema_str
                .chars()
                .next()
                .is_some_and(|c| char::is_ascii_lowercase(&c) || c == '_')
            || !schema_str
                .chars()
                .all(|c| char::is_ascii_lowercase(&c) || char::is_numeric(c) || c == '_')
        {
            return Err(Error::InvalidSchema);
        }
        self.schema = Some(schema_str);
        Ok(self)
    }

    /// Get name of table which is used for storing migrations related
    /// information in database
    ///
    /// Format depends on configuration:
    /// - With schema: `{schema}._sqlx_migrator_migrations`
    /// - With prefix: `_{prefix}_sqlx_migrator_migrations`
    /// - With both: `{schema}._{prefix}_sqlx_migrator_migrations`
    /// - Default: `_sqlx_migrator_migrations`
    ///
    /// # Examples
    /// ```rust
    /// # #[cfg(feature="sqlite")]
    /// # fn main() {
    /// let migrator = sqlx_migrator::Migrator::<sqlx::Sqlite>::new()
    ///     .set_schema("app_schema")
    ///     .unwrap()
    ///     .set_table_prefix("v1")
    ///     .unwrap();
    /// assert_eq!(
    ///     &migrator.table_name(),
    ///     "app_schema._v1_sqlx_migrator_migrations"
    /// );
    /// # }
    /// # #[cfg(not(feature="sqlite"))]
    /// # fn main() {}
    /// ```
    #[must_use]
    pub fn table_name(&self) -> String {
        let mut table_name = DEFAULT_TABLE_NAME.to_string();
        if let Some(prefix) = &self.table_prefix {
            table_name = format!("_{prefix}{table_name}");
        }
        if let Some(schema) = &self.schema {
            table_name = format!("{schema}.{table_name}");
        }
        table_name
    }
}

impl<DB> Default for Migrator<DB> {
    fn default() -> Self {
        Self::new()
    }
}

impl<DB> Info<DB> for Migrator<DB> {
    fn migrations(&self) -> &[BoxMigration<DB>] {
        &self.migrations
    }

    fn migrations_mut(&mut self) -> &mut Vec<BoxMigration<DB>> {
        &mut self.migrations
    }
}

impl<DB> Migrate<DB> for Migrator<DB>
where
    DB: Database,
    Self: DatabaseOperation<DB>,
{
}
