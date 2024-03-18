//! Migrator module
//!
//! It contains common enum and trait for implementing migrator for sqlx
//! supported database
//!
//! It also provides its own struct Migrator which supports Any, Postgres,
//! Sqlite and Mysql database
#![cfg_attr(
    feature = "postgres",
    doc = r##"
# Example
Create own custom Migrator which only supports postgres and uses own unique
table name instead of default table name

```rust,no_run
use sqlx::{Pool, Postgres};
use sqlx_migrator::error::Error;
use sqlx_migrator::migration::{AppliedMigrationSqlRow, Migration};
use sqlx_migrator::migrator::{DatabaseOperation, Info, Migrate};

#[derive(Default)]
pub struct CustomMigrator {
    migrations: Vec<Box<dyn Migration<Postgres>>>,
}

impl Info<Postgres, ()> for CustomMigrator {
    fn migrations(&self) -> &Vec<Box<dyn Migration<Postgres>>> {
        &self.migrations
    }

    fn migrations_mut(&mut self) -> &mut Vec<Box<dyn Migration<Postgres>>> {
        &mut self.migrations
    }

    fn state(&self) -> &() {
        &()
    }
}

#[async_trait::async_trait]
impl DatabaseOperation<Postgres, ()> for CustomMigrator {
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
        connection: &mut <Postgres as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        sqlx::query("DROP TABLE IF EXISTS _custom_table_name")
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn add_migration_to_db_table(
        &self,
        migration: &Box<dyn Migration<Postgres>>,
        connection: &mut <Postgres as sqlx::Database>::Connection,
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
        migration: &Box<dyn Migration<Postgres>>,
        connection: &mut <Postgres as sqlx::Database>::Connection,
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
        connection: &mut <Postgres as sqlx::Database>::Connection,
    ) -> Result<Vec<AppliedMigrationSqlRow>, Error> {
        Ok(sqlx::query_as::<_, AppliedMigrationSqlRow>(
            "SELECT id, app, name, applied_time FROM _custom_table_name",
        )
        .fetch_all(connection)
        .await?)
    }

    async fn lock(
        &self,
        connection: &mut <Postgres as sqlx::Database>::Connection,
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
        connection: &mut <Postgres as sqlx::Database>::Connection,
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
impl Migrate<Postgres, ()> for CustomMigrator {}
```
"##
)]

use std::collections::HashMap;

use sqlx::Connection;

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

type BoxMigration<DB, State> = Box<dyn Migration<DB, State>>;
type MigrationVec<'migration, DB, State> = Vec<&'migration BoxMigration<DB, State>>;
type MigrationVecResult<'migration, DB, State> = Result<MigrationVec<'migration, DB, State>, Error>;

#[derive(Debug)]
enum PlanType {
    Apply,
    Revert,
}

/// Struct which determine type of plan to use
#[derive(Debug)]
pub struct Plan {
    plan_type: PlanType,
    app_migration: Option<(String, Option<String>)>,
    count: Option<usize>,
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
        }
    }

    /// Create new plan for apply all
    #[must_use]
    pub fn apply_all() -> Self {
        Self::new(PlanType::Apply, None, None)
    }

    /// Create new plan for apply for provided app and migration name
    #[must_use]
    pub fn apply_name(app: &str, name: &Option<String>) -> Self {
        Self::new(PlanType::Apply, Some((app.to_string(), name.clone())), None)
    }

    /// Create new plan for apply count
    #[must_use]
    pub fn apply_count(count: usize) -> Self {
        Self::new(PlanType::Apply, None, Some(count))
    }

    /// Create new plan for revert all
    #[must_use]
    pub fn revert_all() -> Self {
        Self::new(PlanType::Revert, None, None)
    }

    /// Create new plan for revert for provided app and migration name
    #[must_use]
    pub fn revert_name(app: &str, name: &Option<String>) -> Self {
        Self::new(
            PlanType::Revert,
            Some((app.to_string(), name.clone())),
            None,
        )
    }

    /// Create new plan for revert count
    #[must_use]
    pub fn revert_count(count: usize) -> Self {
        Self::new(PlanType::Revert, None, Some(count))
    }
}

/// Info trait which implements some of database agnostic methods to add
/// migration or returns immutable or mutable migrations list
pub trait Info<DB, State> {
    /// Return state used in migrator
    fn state(&self) -> &State;

    /// Return migrations
    fn migrations(&self) -> &Vec<BoxMigration<DB, State>>;

    /// Return mutable reference of migrations
    fn migrations_mut(&mut self) -> &mut Vec<BoxMigration<DB, State>>;

    /// Add vector of migrations to Migrator object
    fn add_migrations(&mut self, migrations: Vec<BoxMigration<DB, State>>) {
        for migration in migrations {
            self.add_migration(migration);
        }
    }

    /// Add single migration to migrator object
    fn add_migration(&mut self, migration: BoxMigration<DB, State>) {
        let migration_parents = migration.parents();
        let migration_replaces = migration.replaces();
        let migration_run_before = migration.run_before();

        // check if migration is already added or not we want to use vec here even if
        // hash set can be used but hash set do not have consistent order which may
        // bring issue such as plan may be different between between dry run and
        // actually running migration
        if !self.migrations().contains(&migration) {
            self.migrations_mut().push(migration);

            for parent in migration_parents {
                self.add_migration(parent);
            }
            for replace in migration_replaces {
                self.add_migration(replace);
            }
            for run_before in migration_run_before {
                self.add_migration(run_before);
            }
        }
    }
}

/// Trait which is implemented for database for performing different
/// operations/action on database. Usually this trait is implemented for
/// database to support database along with info trait
#[async_trait::async_trait]
pub trait DatabaseOperation<DB, State>
where
    DB: sqlx::Database,
{
    /// Ensure migration table is created before running migrations. If not
    /// create one
    async fn ensure_migration_table_exists(
        &self,
        connection: &mut <DB as sqlx::Database>::Connection,
    ) -> Result<(), Error>;

    /// Drop migration table if migration table exists
    async fn drop_migration_table_if_exists(
        &self,
        connection: &mut <DB as sqlx::Database>::Connection,
    ) -> Result<(), Error>;

    /// Add migration to migration db table
    #[allow(clippy::borrowed_box)]
    async fn add_migration_to_db_table(
        &self,
        migration: &BoxMigration<DB, State>,
        connection: &mut <DB as sqlx::Database>::Connection,
    ) -> Result<(), Error>;

    /// Delete migration from migration db table
    #[allow(clippy::borrowed_box)]
    async fn delete_migration_from_db_table(
        &self,
        migration: &BoxMigration<DB, State>,
        connection: &mut <DB as sqlx::Database>::Connection,
    ) -> Result<(), Error>;

    /// List all applied migrations from database as struct
    async fn fetch_applied_migration_from_db(
        &self,
        connection: &mut <DB as sqlx::Database>::Connection,
    ) -> Result<Vec<AppliedMigrationSqlRow>, Error>;

    /// Lock database while doing migrations so no two migrations run together
    async fn lock(&self, connection: &mut <DB as sqlx::Database>::Connection) -> Result<(), Error>;

    /// Unlock locked database
    async fn unlock(
        &self,
        connection: &mut <DB as sqlx::Database>::Connection,
    ) -> Result<(), Error>;
}

fn populate_recursive<'populate, DB, State>(
    populate_hash_map: &mut HashMap<
        &'populate BoxMigration<DB, State>,
        Vec<&'populate BoxMigration<DB, State>>,
    >,
    key: &'populate BoxMigration<DB, State>,
    value: &'populate BoxMigration<DB, State>,
) {
    let populate_hash_map_vec = populate_hash_map.entry(key).or_default();
    if !populate_hash_map_vec.contains(&value) {
        populate_hash_map_vec.push(value);
        if let Some(grand_values) = populate_hash_map.clone().get(value) {
            for grand_value in grand_values {
                populate_recursive(populate_hash_map, key, grand_value);
            }
        }
    }
}

fn get_parent_recursive<DB, State>(with: &BoxMigration<DB, State>) -> Vec<BoxMigration<DB, State>> {
    let mut parents = with.parents();
    for parent in with.parents() {
        parents.extend(get_parent_recursive(&parent));
    }
    parents
}

fn get_run_before_recursive<DB, State>(
    with: &BoxMigration<DB, State>,
) -> Vec<BoxMigration<DB, State>> {
    let mut run_before_list = with.run_before();
    for run_before in with.run_before() {
        run_before_list.extend(get_run_before_recursive(&run_before));
    }
    run_before_list
}

fn is_apply_related<DB, State>(
    with: &BoxMigration<DB, State>,
    migration: &BoxMigration<DB, State>,
) -> bool {
    migration.replaces().iter().any(|migration_replace| {
        migration_replace == with || is_apply_related(with, migration_replace)
    }) || migration.run_before().iter().any(|migration_run_before| {
        migration_run_before == with || is_apply_related(with, migration_run_before)
    })
}

fn is_revert_related<DB, State>(
    with: &BoxMigration<DB, State>,
    migration: &BoxMigration<DB, State>,
) -> bool {
    let parents = get_parent_recursive(migration);
    parents.contains(with)
}

fn only_related_migration<DB, State>(
    migration_list: &mut MigrationVec<DB, State>,
    with: &BoxMigration<DB, State>,
    plan_type: &PlanType,
) {
    let mut related_migrations = vec![];
    match plan_type {
        PlanType::Apply => {
            let with_parents = get_parent_recursive(with);
            for &migration in migration_list.iter() {
                if with_parents.contains(migration) || is_apply_related(with, migration) {
                    related_migrations.push(migration);
                }
            }
        }
        PlanType::Revert => {
            let with_run_before = get_run_before_recursive(with);
            for &migration in migration_list.iter() {
                if with_run_before.contains(migration) || is_revert_related(with, migration) {
                    related_migrations.push(migration);
                }
            }
        }
    }
    migration_list
        .retain(|&migration| related_migrations.contains(&migration) || migration == with);
}

/// Process plan to provided migrations list
fn process_plan<DB, State>(
    migration_list: &mut MigrationVec<DB, State>,
    applied_migrations: &MigrationVec<DB, State>,
    plan: &Plan,
) -> Result<(), Error>
where
    DB: sqlx::Database,
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
                    return Err(Error::MigrationNameNotExists {
                        app: app.to_string(),
                        migration: name.to_string(),
                    });
                }
                return Err(Error::AppNameNotExists {
                    app: app.to_string(),
                });
            };
            pos
        } else {
            let Some(pos) = migration_list
                .iter()
                .rposition(|migration| migration.app() == app)
            else {
                return Err(Error::AppNameNotExists {
                    app: app.to_string(),
                });
            };
            pos
        };
        migration_list.truncate(position + 1);
        let pos_elem = migration_list[position];
        only_related_migration(migration_list, pos_elem, &plan.plan_type);
    } else if let Some(count) = plan.count {
        let actual_len = migration_list.len();
        if count > actual_len {
            return Err(Error::CountGreater { actual_len, count });
        }
        migration_list.truncate(count);
    }
    Ok(())
}

fn get_recursive<'get, DB, State>(
    hash_map: &'get HashMap<BoxMigration<DB, State>, &'get BoxMigration<DB, State>>,
    val: &'get BoxMigration<DB, State>,
) -> Vec<&'get BoxMigration<DB, State>> {
    let mut recursive_vec = vec![val];
    if let Some(&parent) = hash_map.get(val) {
        recursive_vec.extend(get_recursive(hash_map, parent));
    }
    recursive_vec
}

/// Migrate trait which migrate a database according to requirements. This trait
/// implements all methods which depends on `DatabaseOperation` trait and `Info`
/// trait. This trait doesn't requires to implement any method since all
/// function have default implementation and all methods are database agnostics
#[async_trait::async_trait]
pub trait Migrate<DB, State>: Info<DB, State> + DatabaseOperation<DB, State> + Send + Sync
where
    DB: sqlx::Database,
    State: Send + Sync,
{
    /// Generate migration plan according to plan. Returns a vector of
    /// migration. If plan is none than it will generate plan with all
    /// migrations in chronological order of apply
    #[allow(clippy::too_many_lines)]
    async fn generate_migration_plan(
        &self,
        plan: Option<&Plan>,
        connection: &mut <DB as sqlx::Database>::Connection,
    ) -> MigrationVecResult<DB, State> {
        tracing::debug!("fetching applied migrations");

        self.ensure_migration_table_exists(connection).await?;

        tracing::debug!("generating {:?} migration plan", plan);

        // Hashmap which contains key as migration name and value as one replaces
        // migrations which replace it this should be parent of replaces migration since
        // we cannot remove replaces migration down below until migration is
        // parent without possible children side effect.
        let mut parent_due_to_replaces = HashMap::new();

        for parent_migration in self.migrations() {
            for child_migration in parent_migration.replaces() {
                if parent_due_to_replaces
                    .insert(child_migration, parent_migration)
                    .is_some()
                {
                    return Err(Error::MigrationReplacedMultipleTimes);
                }
            }
        }

        // Hashmap which contains all children generated from replace list
        let mut replace_children = HashMap::<_, Vec<_>>::new();
        // in first loop add initial parent and child from parent due to replace
        for (child, &parent) in &parent_due_to_replaces {
            // since parent due to replaces is hash map we can have only one child
            // occurrence
            replace_children.entry(parent).or_default().push(child);
        }
        // in second loop through recursive add all descendants
        for (child, &parent) in &parent_due_to_replaces {
            populate_recursive(&mut replace_children, parent, child);
        }

        // Hashmap which contains key as migration name and value as list of migration
        // which becomes parent for key due to value having key as run before value
        let mut parents_due_to_run_before = HashMap::<_, Vec<_>>::new();

        for parent_migration in self.migrations() {
            for run_before_migration in parent_migration.run_before() {
                parents_due_to_run_before
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
                    && parents_due_to_run_before
                        .get(migration)
                        .unwrap_or(&vec![])
                        .iter()
                        .all(|run_before_migration| migration_list.contains(run_before_migration))
                    && parent_due_to_replaces
                        .get(migration)
                        .map_or(true, |replace_migration| {
                            migration_list.contains(replace_migration)
                        })
                    && replace_children.get(migration).map_or(true, |children| {
                        children.iter().all(|&child| {
                            child
                                .parents()
                                .iter()
                                .all(|child_parent| migration_list.contains(&child_parent));
                            parents_due_to_run_before
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
                return Err(Error::FailedToCreateMigrationPlan);
            }
        }

        // if there is only plan than further process. In further process replaces
        // migrations are also handled for removing conflicting migrations where certain
        // migrations replaces certain other migrations
        if let Some(some_plan) = plan {
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
                if let Some(run_before_list) = parents_due_to_run_before.get(migration) {
                    for &run_before in run_before_list {
                        parents.push(run_before);
                    }
                }
                let main_parents = migration.parents();
                for parent in &main_parents {
                    parents.push(parent);
                }
                for parent in parents {
                    let recursive_vec = get_recursive(&parent_due_to_replaces, parent);
                    if !applied_migrations
                        .iter()
                        .any(|applied| recursive_vec.contains(applied))
                    {
                        return Err(Error::ParentIsNotApplied);
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
                            return Err(Error::BothMigrationTypeApplied);
                        }
                        migration_list.retain(|&plan_migration| migration != plan_migration);
                    } else {
                        // we can remove all children migrations here since migrations which
                        // replaced them will be parent of them so there will be no side effect
                        for replaced_migration in children {
                            migration_list
                                .retain(|plan_migration| replaced_migration != plan_migration);
                        }
                    }
                }
            }

            process_plan(&mut migration_list, &applied_migrations, some_plan)?;
        }

        Ok(migration_list)
    }

    /// Run provided plan migrations
    ///
    /// # Errors
    /// If failed to run provided plan migrations
    async fn run(
        &self,
        connection: &mut <DB as sqlx::Database>::Connection,
        plan: &Plan,
    ) -> Result<(), Error> {
        tracing::debug!("running plan {:?}", plan);
        self.lock(connection).await?;
        for migration in self.generate_migration_plan(Some(plan), connection).await? {
            match plan.plan_type {
                PlanType::Apply => {
                    tracing::debug!("applying {} : {}", migration.app(), migration.name());
                    if migration.is_atomic() {
                        let mut transaction = connection.begin().await?;
                        for operation in migration.operations() {
                            operation.up(&mut transaction, self.state()).await?;
                        }
                        self.add_migration_to_db_table(migration, &mut transaction)
                            .await?;
                        transaction.commit().await?;
                    } else {
                        for operation in migration.operations() {
                            operation.up(connection, self.state()).await?;
                        }
                        self.add_migration_to_db_table(migration, connection)
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
                        for operation in operations {
                            operation.down(&mut transaction, self.state()).await?;
                        }
                        self.delete_migration_from_db_table(migration, &mut transaction)
                            .await?;
                        transaction.commit().await?;
                    } else {
                        for operation in operations {
                            operation.down(connection, self.state()).await?;
                        }
                        self.delete_migration_from_db_table(migration, connection)
                            .await?;
                    }
                }
            };
        }
        self.unlock(connection).await?;
        Ok(())
    }
}

const DEFAULT_TABLE_NAME: &str = "_sqlx_migrator_migrations";

/// Migrator struct which store migrations graph and information related to
/// different library supported migrations
pub struct Migrator<DB, State> {
    migrations: Vec<BoxMigration<DB, State>>,
    table_name: String,
    state: State,
}

impl<DB, State> Migrator<DB, State> {
    /// Create new migrator with provided state
    fn new(state: State) -> Self {
        Self {
            migrations: Vec::default(),
            table_name: DEFAULT_TABLE_NAME.to_string(),
            state,
        }
    }

    /// Use prefix for migrator table name only ascii alpha numeric and
    /// underscore characters are supported for table name. prefix will set
    /// table name as `_{prefix}{default_table_name}` where default table
    /// name is `_sqlx_migrator_migrations`
    ///
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

impl<DB, State> Default for Migrator<DB, State>
where
    State: Default,
{
    fn default() -> Self {
        Self::new(State::default())
    }
}

impl<DB, State> Info<DB, State> for Migrator<DB, State> {
    fn state(&self) -> &State {
        &self.state
    }

    fn migrations(&self) -> &Vec<BoxMigration<DB, State>> {
        &self.migrations
    }

    fn migrations_mut(&mut self) -> &mut Vec<BoxMigration<DB, State>> {
        &mut self.migrations
    }
}

impl<DB, State> Migrate<DB, State> for Migrator<DB, State>
where
    DB: sqlx::Database,
    Self: DatabaseOperation<DB, State>,
    State: Send + Sync,
{
}
