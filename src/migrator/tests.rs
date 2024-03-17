use std::collections::HashSet;

use sqlx::{Sqlite, SqlitePool};

use super::{DatabaseOperation, Info, Migrate};
use crate::error::Error;
use crate::migration::{AppliedMigrationSqlRow, Migration};
use crate::migrator::Plan;
use crate::vec_box;

#[derive(Default)]
struct CustomMigrator {
    migrations: HashSet<Box<dyn Migration<Sqlite>>>,
}

impl Info<Sqlite, ()> for CustomMigrator {
    fn migrations(&self) -> &HashSet<Box<dyn Migration<Sqlite>>> {
        &self.migrations
    }

    fn migrations_mut(&mut self) -> &mut HashSet<Box<dyn Migration<Sqlite>>> {
        &mut self.migrations
    }

    fn state(&self) -> &() {
        &()
    }
}

#[async_trait::async_trait]
impl DatabaseOperation<Sqlite, ()> for CustomMigrator {
    async fn ensure_migration_table_exists(
        &self,
        _connection: &mut <Sqlite as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        Ok(())
    }

    async fn drop_migration_table_if_exists(
        &self,
        _connection: &mut <Sqlite as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        Ok(())
    }

    async fn add_migration_to_db_table(
        &self,
        _migration: &Box<dyn Migration<Sqlite>>,
        _connection: &mut <Sqlite as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        Ok(())
    }

    async fn delete_migration_from_db_table(
        &self,
        _migration: &Box<dyn Migration<Sqlite>>,
        _connection: &mut <Sqlite as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        Ok(())
    }

    async fn fetch_applied_migration_from_db(
        &self,
        _connection: &mut <Sqlite as sqlx::Database>::Connection,
    ) -> Result<Vec<AppliedMigrationSqlRow>, Error> {
        Ok(vec![])
    }

    async fn lock(
        &self,
        _connection: &mut <Sqlite as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        Ok(())
    }

    async fn unlock(
        &self,
        _connection: &mut <Sqlite as sqlx::Database>::Connection,
    ) -> Result<(), Error> {
        Ok(())
    }
}

impl Migrate<Sqlite, ()> for CustomMigrator {}

macro_rules! migration {
    ($op:ty, $name:expr, $parents:expr, $operations:expr, $replaces:expr, $run_before:expr) => {
        #[async_trait::async_trait]
        impl crate::migration::Migration<sqlx::Sqlite, ()> for $op {
            fn app(&self) -> &str {
                "test"
            }

            fn name(&self) -> &str {
                $name
            }

            fn parents(&self) -> Vec<Box<dyn crate::migration::Migration<sqlx::Sqlite, ()>>> {
                $parents
            }

            fn operations(&self) -> Vec<Box<dyn crate::operation::Operation<sqlx::Sqlite, ()>>> {
                $operations
            }

            fn replaces(&self) -> Vec<Box<dyn crate::migration::Migration<sqlx::Sqlite, ()>>> {
                $replaces
            }

            fn run_before(&self) -> Vec<Box<dyn crate::migration::Migration<sqlx::Sqlite, ()>>> {
                $run_before
            }
        }
    };
}

async fn generate_plan(
    migrator: &mut CustomMigrator,
    migration_list: Vec<Box<dyn Migration<Sqlite>>>,
) -> Result<Vec<&Box<dyn Migration<Sqlite>>>, Error> {
    migrator.add_migrations(migration_list);
    let sqlite = SqlitePool::connect("sqlite::memory:").await.unwrap();
    let mut conn = sqlite.acquire().await.unwrap();
    migrator
        .generate_migration_plan(Some(&Plan::apply_all()), &mut conn)
        .await
}

#[tokio::test]
async fn simple_test() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!(), vec_box!());
    struct C;
    migration!(C, "c", vec_box!(B), vec_box!(), vec_box!(), vec_box!());
    let mut migrator = CustomMigrator::default();
    let plan = generate_plan(&mut migrator, vec_box!(A, B, C))
        .await
        .unwrap();
    assert!(plan.contains(&&(Box::new(A) as Box<dyn Migration<Sqlite>>)));
    assert!(plan.contains(&&(Box::new(B) as Box<dyn Migration<Sqlite>>)));
    assert!(plan.contains(&&(Box::new(C) as Box<dyn Migration<Sqlite>>)));
}

#[tokio::test]
async fn replace_test() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!(), vec_box!());
    struct C;
    migration!(C, "c", vec_box!(), vec_box!(), vec_box!(B), vec_box!());
    struct D;
    migration!(D, "d", vec_box!(), vec_box!(), vec_box!(C), vec_box!());
    let mut migrator = CustomMigrator::default();
    let plan = generate_plan(&mut migrator, vec_box!(A, B, C, D))
        .await
        .unwrap();
    assert!(!plan.contains(&&(Box::new(B) as Box<dyn Migration<Sqlite>>)));
    assert!(!plan.contains(&&(Box::new(C) as Box<dyn Migration<Sqlite>>)));
    assert!(plan.contains(&&(Box::new(D) as Box<dyn Migration<Sqlite>>)));
}

#[tokio::test]
async fn run_before_test() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!(), vec_box!());
    struct C;
    migration!(C, "c", vec_box!(), vec_box!(), vec_box!(), vec_box!(B));
    struct D;
    migration!(D, "d", vec_box!(), vec_box!(), vec_box!(), vec_box!(C));
    let mut migrator = CustomMigrator::default();
    let plan = generate_plan(&mut migrator, vec_box!(A, B, C, D))
        .await
        .unwrap();
    assert!(
        plan.get(1).unwrap() == &&(Box::new(D) as Box<dyn Migration<Sqlite>>),
        "1 index is not D"
    );
    assert!(
        plan.get(2).unwrap() == &&(Box::new(C) as Box<dyn Migration<Sqlite>>),
        "2 index is not C"
    );
    assert!(
        plan.get(3).unwrap() == &&(Box::new(B) as Box<dyn Migration<Sqlite>>),
        "3 index is not B"
    );
}

#[tokio::test]
async fn replaces_multiple_times() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!(), vec_box!());
    struct C;
    migration!(C, "c", vec_box!(), vec_box!(), vec_box!(B), vec_box!());
    struct D;
    migration!(D, "d", vec_box!(), vec_box!(), vec_box!(B), vec_box!());
    let mut migrator = CustomMigrator::default();
    let plan = generate_plan(&mut migrator, vec_box!(A, B, C, D)).await;
    assert!(plan.is_err());
}

#[tokio::test]
async fn replaces_run_before_mismatch_1() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!(), vec_box!());
    struct C;
    migration!(C, "c", vec_box!(), vec_box!(), vec_box!(B), vec_box!());
    struct D;
    migration!(D, "d", vec_box!(), vec_box!(), vec_box!(), vec_box!(B));
    let mut migrator = CustomMigrator::default();
    let plan = generate_plan(&mut migrator, vec_box!(A, B, C, D)).await;
    assert!(plan.is_err());
}

#[tokio::test]
async fn replaces_run_before_mismatch_2() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!(), vec_box!());
    struct C;
    migration!(C, "c", vec_box!(), vec_box!(), vec_box!(B), vec_box!());
    struct D;
    migration!(D, "d", vec_box!(), vec_box!(), vec_box!(C), vec_box!());
    struct E;
    migration!(E, "e", vec_box!(), vec_box!(), vec_box!(), vec_box!(C));
    let mut migrator = CustomMigrator::default();
    let plan = generate_plan(&mut migrator, vec_box!(A, B, C, D, E)).await;
    assert!(plan.is_err());
}

#[tokio::test]
async fn replaces_run_before_ok_1() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!(), vec_box!());
    struct C;
    migration!(C, "c", vec_box!(), vec_box!(), vec_box!(B), vec_box!());
    struct D;
    migration!(D, "d", vec_box!(), vec_box!(), vec_box!(C), vec_box!());
    struct E;
    migration!(E, "e", vec_box!(), vec_box!(), vec_box!(), vec_box!(D));
    let mut migrator = CustomMigrator::default();
    let plan = generate_plan(&mut migrator, vec_box!(A, B, C, D, E)).await;
    assert!(plan.is_ok(), "{:?}", plan.err());
}

#[tokio::test]
async fn replaces_run_before_ok_2() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!(), vec_box!());
    struct C;
    migration!(C, "c", vec_box!(), vec_box!(), vec_box!(B), vec_box!());
    struct D;
    migration!(D, "d", vec_box!(), vec_box!(), vec_box!(C, E), vec_box!());
    struct E;
    migration!(E, "e", vec_box!(), vec_box!(), vec_box!(), vec_box!(C));
    let mut migrator = CustomMigrator::default();
    let plan = generate_plan(&mut migrator, vec_box!(A, B, C, D, E)).await;
    assert!(plan.is_ok(), "{:?}", plan.err());
}

#[tokio::test]
async fn loop_error() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!(), vec_box!(A));
    let mut migrator = CustomMigrator::default();
    let plan = generate_plan(&mut migrator, vec_box!(A, B)).await;
    assert!(plan.is_err());
}
