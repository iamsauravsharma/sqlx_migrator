//! Migrator module
//!
//! It contains common enum and trait for implementing migrator for sqlx
//! supported database
//!
//! It also provides its own struct [`Migrator`] which supports
//! [`Any`](sqlx::Any), [`Postgres`](sqlx::Postgres), [`Sqlite`](sqlx::Sqlite)
//! and [`MySql`](sqlx::MySql) database
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

#[derive(Default)]
pub struct CustomMigrator {
    migrations: Vec<Box<dyn Migration<Postgres>>>,
}

impl Info<Postgres> for CustomMigrator {
    fn migrations(&self) -> &Vec<Box<dyn Migration<Postgres>>> {
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
        migration: &Box<dyn Migration<Postgres>>,
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
        migration: &Box<dyn Migration<Postgres>>,
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
```
"#
)]

use std::collections::HashMap;

use sqlx::{Connection, Database};

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
#[cfg(all(test, feature = "sqlite"))]
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
    fn migrations(&self) -> &Vec<BoxMigration<DB>>;

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
        // only check old value if provided migration for adding is not virtual
        if migration.is_virtual() {
            if !migration.parents().is_empty()
                || !migration.operations().is_empty()
                || !migration.replaces().is_empty()
                || !migration.run_before().is_empty()
            {
                return Err(Error::InvalidVirtualMigration);
            }
        } else if let Some((migration_index, found_migration)) = self
            .migrations()
            .iter()
            .enumerate()
            .find(|(_, elem)| elem == &&migration)
        {
            // if virtual migration is present in list with same app and name than remove
            // virtual migration from list first
            if found_migration.is_virtual() {
                self.migrations_mut().remove(migration_index);
            }
            // if found migrations value are not consistent to current provided migration then
            // raise error only raise error when found migration is not virtual
            else if found_migration.parents() != migration.parents()
                || found_migration.operations().len() != migration.operations().len()
                || found_migration.replaces() != migration.replaces()
                || found_migration.run_before() != migration.run_before()
                || found_migration.is_atomic() != migration.is_atomic()
            {
                return Err(Error::InconsistentMigration {
                    app: migration.app().to_string(),
                    name: migration.name().to_string(),
                });
            }
        }

        // check if migration is already added or not and only do operation if migration
        // is not added till now
        if !self.migrations().contains(&migration) {
            // ignore parents, replaces and run before for virtual migration only add
            // migration only. If virtual migration provides those value than it is ignored
            if migration.is_virtual() {
                self.migrations_mut().push(migration);
            } else {
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
        migration: &BoxMigration<DB>,
    ) -> Result<(), Error>;

    /// Removes a migration record from the migration table in the database.
    async fn delete_migration_from_db_table(
        &self,
        connection: &mut <DB as Database>::Connection,
        migration: &BoxMigration<DB>,
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
    if let Some(grand_values) = replace_hash_map.clone().get(value) {
        for grand_value in grand_values {
            populate_replace_recursive(replace_hash_map, key, grand_value)?;
        }
    }
    Ok(())
}

fn get_parent_recursive<DB>(
    migration: &BoxMigration<DB>,
    original_migration: &[BoxMigration<DB>],
) -> Result<Vec<BoxMigration<DB>>, Error> {
    let mut parents: Vec<BoxMigration<DB>> = vec![];
    for parent in migration.parents() {
        parents.push(Box::new((
            parent.app().to_string(),
            parent.name().to_string(),
        )));
        let found_parent = if parent.is_virtual() {
            original_migration
                .iter()
                .find(|&search_parent| search_parent == &parent)
                .ok_or(Error::PlanError {
                    message: "failed to find parent non virtual migration".to_string(),
                })?
        } else {
            &parent
        };
        parents.extend(get_parent_recursive(found_parent, original_migration)?);
    }
    Ok(parents)
}

fn get_run_before_recursive<DB>(
    migration: &BoxMigration<DB>,
    original_migration: &[BoxMigration<DB>],
) -> Result<Vec<BoxMigration<DB>>, Error> {
    let mut run_before_list: Vec<BoxMigration<DB>> = vec![];
    for run_before in migration.run_before() {
        run_before_list.push(Box::new((
            run_before.app().to_string(),
            run_before.name().to_string(),
        )));
        let found_run_before = if run_before.is_virtual() {
            original_migration
                .iter()
                .find(|&search_run_before| search_run_before == &run_before)
                .ok_or(Error::PlanError {
                    message: "failed to find run before non virtual migration".to_string(),
                })?
        } else {
            &run_before
        };
        run_before_list.extend(get_parent_recursive(found_run_before, original_migration)?);
    }
    Ok(run_before_list)
}

// filter migration list to only contains migrations which is related to with
// list migration, removes all migrations which is not related to them according
// to provided plan. We should not check replaces migration since it is already
// handled and all replaces migration are removed as required
fn only_related_migration<DB>(
    migration_list: &mut MigrationVec<DB>,
    with_list: Vec<&BoxMigration<DB>>,
    plan_type: &PlanType,
    original_migration: &[BoxMigration<DB>],
) -> Result<(), Error> {
    let mut related_migrations = vec![];
    for with in with_list {
        // check if with migrations is already added or not. Sometimes with list
        // contains migrations which are interrelated so we do not need to add
        // already added migration again
        if !related_migrations.contains(&with) {
            related_migrations.push(with);
            match plan_type {
                PlanType::Apply => {
                    let with_parents = get_parent_recursive(with, original_migration)?;
                    for &migration in migration_list.iter() {
                        if !related_migrations.contains(&migration)
                            && (with_parents.contains(migration)
                                || get_run_before_recursive(migration, original_migration)?
                                    .contains(with))
                        {
                            related_migrations.push(migration);
                        }
                    }
                }
                PlanType::Revert => {
                    let with_run_before = get_run_before_recursive(with, original_migration)?;
                    for &migration in migration_list.iter() {
                        if !related_migrations.contains(&migration)
                            && (with_run_before.contains(migration)
                                || get_parent_recursive(migration, original_migration)?
                                    .contains(with))
                        {
                            related_migrations.push(migration);
                        }
                    }
                }
            }
        }
    }
    migration_list.retain(|&migration| related_migrations.contains(&migration));
    Ok(())
}

/// Process plan to provided migrations list
fn process_plan<DB>(
    migration_list: &mut MigrationVec<DB>,
    applied_migrations: &MigrationVec<DB>,
    plan: &Plan,
    original_migration: &[BoxMigration<DB>],
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
        only_related_migration(
            migration_list,
            with_list,
            &plan.plan_type,
            original_migration,
        )?;
    } else if let Some(count) = plan.count {
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

fn get_recursive<'get, DB>(
    hash_map: &'get HashMap<BoxMigration<DB>, &'get BoxMigration<DB>>,
    val: &'get BoxMigration<DB>,
) -> Vec<&'get BoxMigration<DB>> {
    let mut recursive_vec = vec![val];
    if let Some(&parent) = hash_map.get(val) {
        recursive_vec.extend(get_recursive(hash_map, parent));
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
    /// Generate migration plan according to plan.
    ///
    /// Returns a vector of migration. If plan is none than it will generate
    /// plan with all migrations in order of apply
    #[expect(clippy::too_many_lines)]
    async fn generate_migration_plan(
        &self,
        connection: &mut <DB as Database>::Connection,
        plan: Option<&Plan>,
    ) -> MigrationVecResult<DB> {
        if self.migrations().is_empty() {
            return Err(Error::PlanError {
                message: "no migration are added to migration list".to_string(),
            });
        }
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

        // Hashmap which contains key as migration and value is migration which replaces
        // this migration. One migration can only have one parent
        let mut replaces_child_parent_hash_map = HashMap::new();

        for parent_migration in self.migrations() {
            for child_migration in parent_migration.replaces() {
                let child_name = format!("{}:{}", child_migration.app(), child_migration.name());
                if replaces_child_parent_hash_map
                    .insert(child_migration, parent_migration)
                    .is_some()
                {
                    return Err(Error::PlanError {
                        message: format!("migration {child_name} replaced multiple times",),
                    });
                }
            }
        }

        // Hashmap which contains all children of migration generated from replace list
        let mut replace_children = HashMap::<_, Vec<_>>::new();
        // in first loop add initial parent and child from parent due to replace
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
            replace_children
                .entry(parent)
                .or_default()
                .push(children_migration);
        }
        // in second loop through recursive add all descendants
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
        // Hashmap which contains key as migration and value as list of migration
        // which becomes parent for key due to value having key as run before value
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

        // Create migration list until migration list length is equal to original vec
        // length
        let original_migration_length = self.migrations().len();
        while migration_list.len() != original_migration_length {
            let loop_initial_migration_list_length = migration_list.len();
            for migration in self.migrations() {
                let all_required_added = !migration_list.contains(&migration)
                    && migration
                        .parents()
                        .iter()
                        .all(|parent_migration| migration_list.contains(&parent_migration))
                    && run_before_child_parent_hash_map
                        .get(migration)
                        .unwrap_or(&vec![])
                        .iter()
                        .all(|run_before_migration| migration_list.contains(run_before_migration))
                    && replaces_child_parent_hash_map
                        .get(migration)
                        .is_none_or(|replace_migration| migration_list.contains(replace_migration))
                    && replace_children.get(migration).is_none_or(|children| {
                        // check if children parents and run before are added or not already before
                        // adding replace migration. Since replace migration may not depend on
                        // children parent its need to be added first
                        children.iter().all(|&child| {
                            child
                                .parents()
                                .iter()
                                .all(|child_parent| migration_list.contains(&child_parent))
                                && run_before_child_parent_hash_map
                                    .get(child)
                                    .unwrap_or(&vec![])
                                    .iter()
                                    .all(|run_before_migration| {
                                        migration_list.contains(run_before_migration)
                                            || children.contains(run_before_migration)
                                    })
                        })
                    });
                if all_required_added {
                    migration_list.push(migration);
                }
            }

            // If old migration plan length is equal to current length than no new migration
            // was added. Next loop also will not add migration so return error. This case
            // can arise due to looping in migration plan i.e If there is two migration A
            // and B, than when B is ancestor of A as well as descendants of A
            if loop_initial_migration_list_length == migration_list.len() {
                return Err(Error::PlanError {
                    message: "reached deadlock stage during plan generation".to_string(),
                });
            }
        }

        // if there is only plan than further process. In further process replaces
        // migrations are also handled for removing conflicting migrations where certain
        // migrations replaces certain other migrations. While initially creating
        // migrations both new and replaced migration are present
        if let Some(some_plan) = plan {
            self.ensure_migration_table_exists(connection).await?;

            let applied_migration_sql_rows =
                self.fetch_applied_migration_from_db(connection).await?;

            // convert applied migration sql rows to vector of migration implemented
            // objects
            let mut applied_migrations = Vec::new();
            for migration in self.migrations() {
                if applied_migration_sql_rows
                    .iter()
                    .any(|sqlx_migration| sqlx_migration == migration)
                {
                    applied_migrations.push(migration);
                }
            }

            // Check if any of parents of certain applied migrations are applied or not. If
            // any parents are not applied for applied migration than raises
            // error also takes consideration of replace migration
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
                    let recursive_vec = get_recursive(&replaces_child_parent_hash_map, parent);
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

            // Remove migration from migration list according to replaces vector
            for migration in migration_list.clone() {
                // Only need to check case when migration have children
                if let Some(children) = replace_children.get(&migration) {
                    // Check if any replaces children are applied or not
                    let replaces_applied = children
                        .iter()
                        .any(|&replace_migration| applied_migrations.contains(&replace_migration));

                    // If any one of replaced migrations is applied than do not add current
                    // migration to migration plan else add only current migration to migration plan
                    if replaces_applied {
                        // Error if current migration as well as replace migration both are applied
                        if applied_migrations.contains(&migration) {
                            return Err(Error::PlanError {
                                message: format!(
                                    "migration {}:{} and its replaces are applied together",
                                    migration.app(),
                                    migration.name(),
                                ),
                            });
                        }
                        migration_list.retain(|&plan_migration| migration != plan_migration);
                    } else {
                        // we can remove all children migrations here since migrations which
                        // replaced them will be above them in generation list so migration will
                        // apply in provided order
                        for replaced_migration in children {
                            migration_list
                                .retain(|plan_migration| replaced_migration != plan_migration);
                        }
                    }
                }
            }

            process_plan(
                &mut migration_list,
                &applied_migrations,
                some_plan,
                self.migrations(),
            )?;
        }

        Ok(migration_list)
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
        // do not return result of migrations early from run function hold it till lock
        // is unlocked
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
                            self.add_migration_to_db_table(&mut transaction, migration)
                                .await?;
                            transaction.commit().await?;
                        } else {
                            if !plan.fake {
                                for operation in operations {
                                    operation.up(connection).await?;
                                }
                            }
                            self.add_migration_to_db_table(connection, migration)
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
                            self.delete_migration_from_db_table(&mut transaction, migration)
                                .await?;
                            transaction.commit().await?;
                        } else {
                            if !plan.fake {
                                for operation in operations {
                                    operation.down(connection).await?;
                                }
                            }
                            self.delete_migration_from_db_table(connection, migration)
                                .await?;
                        }
                    }
                }
            }
            Ok(())
        }
        .await;
        // unlock lock before returning result of applying migration
        self.unlock(connection).await?;
        result
    }
}

const DEFAULT_TABLE_NAME: &str = "_sqlx_migrator_migrations";

/// Migrator struct which store migrations graph and information related to
/// different library supported migrations
pub struct Migrator<DB> {
    migrations: Vec<BoxMigration<DB>>,
    table_name: String,
}

impl<DB> Migrator<DB> {
    /// Create new migrator
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature="sqlite")]
    /// # fn main() {
    /// let migrator = sqlx_migrator::Migrator::<sqlx::Sqlite>::new();
    /// assert_eq!(migrator.table_name(), "_sqlx_migrator_migrations")
    /// # }
    /// # #[cfg(not(feature="sqlite"))]
    /// # fn main() {
    /// # }
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            migrations: Vec::default(),
            table_name: DEFAULT_TABLE_NAME.to_string(),
        }
    }

    /// Use prefix for migrator table name only ascii alpha numeric and
    /// underscore characters are supported for table name. prefix will set
    /// table name as `_{prefix}{default_table_name}` where default table
    /// name is `_sqlx_migrator_migrations`
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature="sqlite")]
    /// # fn main() {
    /// let migrator = sqlx_migrator::Migrator::<sqlx::Sqlite>::new()
    ///     .with_prefix("prefix_value")
    ///     .unwrap();
    /// assert_eq!(
    ///     migrator.table_name(),
    ///     "_prefix_value_sqlx_migrator_migrations"
    /// )
    /// # }
    /// # #[cfg(not(feature="sqlite"))]
    /// # fn main() {
    /// # }
    /// ```
    /// # Errors
    /// When passed prefix is not ascii alpha numeric or underscore character
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Result<Self, Error> {
        let prefix_str = prefix.into();
        if !prefix_str
            .chars()
            .all(|c| char::is_ascii_alphanumeric(&c) || c == '_')
        {
            return Err(Error::NonAsciiAlphaNumeric);
        }
        self.table_name = format!("_{prefix_str}{DEFAULT_TABLE_NAME}");
        Ok(self)
    }

    /// Get name of table which is used for storing migrations related
    /// information in database
    #[must_use]
    pub fn table_name(&self) -> &str {
        &self.table_name
    }
}

impl<DB> Default for Migrator<DB> {
    fn default() -> Self {
        Self::new()
    }
}

impl<DB> Info<DB> for Migrator<DB> {
    fn migrations(&self) -> &Vec<BoxMigration<DB>> {
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
