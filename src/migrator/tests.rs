use sqlx::{Sqlite, SqlitePool};

use super::{DatabaseOperation, Info, Migrate};
use crate::error::Error;
use crate::migration::{AppliedMigrationSqlRow, Migration};
use crate::migrator::Plan;
use crate::vec_box;

#[derive(Default)]
struct CustomMigrator {
    migrations: Vec<Box<dyn Migration<Sqlite>>>,
    applied_migrations: Vec<AppliedMigrationSqlRow>,
}

impl CustomMigrator {
    fn add_applied_migrations(&mut self, migrations: Vec<Box<dyn Migration<Sqlite>>>) {
        for migration in migrations {
            self.add_applied_migration(&migration);
        }
    }

    #[allow(clippy::borrowed_box)]
    fn add_applied_migration(&mut self, migration: &Box<dyn Migration<Sqlite>>) {
        let current_length = self.migrations.len();
        self.applied_migrations.push(AppliedMigrationSqlRow::new(
            i32::try_from(current_length).unwrap(),
            migration.app(),
            migration.name(),
        ));
    }
}

impl Info<Sqlite, ()> for CustomMigrator {
    fn migrations(&self) -> &Vec<Box<dyn Migration<Sqlite>>> {
        &self.migrations
    }

    fn migrations_mut(&mut self) -> &mut Vec<Box<dyn Migration<Sqlite>>> {
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
        Ok(self.applied_migrations.clone())
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
    ($op:ty, $name:expr, $parents:expr, $replaces:expr, $run_before:expr) => {
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
                vec![]
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

async fn generate_apply_all_plan(
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
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    struct C;
    migration!(C, "c", vec_box!(B), vec_box!(), vec_box!());
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C))
        .await
        .unwrap();
    assert!(plan.contains(&&(Box::new(A) as Box<dyn Migration<Sqlite>>)));
    assert!(plan.contains(&&(Box::new(B) as Box<dyn Migration<Sqlite>>)));
    assert!(plan.contains(&&(Box::new(C) as Box<dyn Migration<Sqlite>>)));
}

#[tokio::test]
async fn all_not_added() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(B)).await;
    assert!(plan.is_err());
}

#[tokio::test]
async fn interrelated_test() {
    struct A;
    migration!(A, "a", vec_box!(B), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B)).await;
    assert!(plan.is_err());
}

#[tokio::test]
async fn run_before_interrelated_test() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(B), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(), vec_box!(A), vec_box!());
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B)).await;
    assert!(plan.is_err());
}

#[tokio::test]
async fn replace_interrelated_test() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!(B));
    struct B;
    migration!(B, "b", vec_box!(), vec_box!(), vec_box!(A));
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B)).await;
    assert!(plan.is_err());
}

#[tokio::test]
async fn depend_on_itself() {
    struct A;
    migration!(A, "a", vec_box!(A), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(B), vec_box!(), vec_box!());
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B)).await;
    assert!(plan.is_err());
}

#[tokio::test]
async fn run_before_depend_on_itself() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(A), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(), vec_box!(B), vec_box!());
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B)).await;
    assert!(plan.is_err());
}

#[tokio::test]
async fn replace_depend_on_itself() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!(A));
    struct B;
    migration!(B, "b", vec_box!(), vec_box!(), vec_box!(B));
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B)).await;
    assert!(plan.is_err());
}

#[tokio::test]
async fn replace_test() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    struct C;
    migration!(C, "c", vec_box!(), vec_box!(B), vec_box!());
    struct D;
    migration!(D, "d", vec_box!(), vec_box!(C), vec_box!());
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D))
        .await
        .unwrap();
    let d_position = plan
        .iter()
        .position(|p| p == &&(Box::new(D) as Box<dyn Migration<Sqlite>>))
        .unwrap();
    let a_position = plan
        .iter()
        .position(|p| p == &&(Box::new(A) as Box<dyn Migration<Sqlite>>))
        .unwrap();
    assert!(a_position < d_position);
    assert!(!plan.contains(&&(Box::new(B) as Box<dyn Migration<Sqlite>>)));
    assert!(!plan.contains(&&(Box::new(C) as Box<dyn Migration<Sqlite>>)));
    assert!(plan.contains(&&(Box::new(D) as Box<dyn Migration<Sqlite>>)));
}

#[tokio::test]
async fn run_before_test() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    struct C;
    migration!(C, "c", vec_box!(), vec_box!(), vec_box!(B));
    struct D;
    migration!(D, "d", vec_box!(), vec_box!(), vec_box!(C));
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D))
        .await
        .unwrap();
    let d_position = plan
        .iter()
        .position(|p| p == &&(Box::new(D) as Box<dyn Migration<Sqlite>>))
        .unwrap();
    let c_position = plan
        .iter()
        .position(|p| p == &&(Box::new(C) as Box<dyn Migration<Sqlite>>))
        .unwrap();
    let b_position = plan
        .iter()
        .position(|p| p == &&(Box::new(B) as Box<dyn Migration<Sqlite>>))
        .unwrap();
    assert!(d_position < c_position);
    assert!(c_position < b_position);
}

#[tokio::test]
async fn replaces_multiple_times() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    struct C;
    migration!(C, "c", vec_box!(), vec_box!(B), vec_box!());
    struct D;
    migration!(D, "d", vec_box!(), vec_box!(B), vec_box!());
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D)).await;
    assert!(plan.is_err());
}

#[tokio::test]
async fn replace_run_before_cond_1() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    struct C;
    migration!(C, "c", vec_box!(), vec_box!(B), vec_box!());
    struct D;
    migration!(D, "d", vec_box!(), vec_box!(), vec_box!(B));
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D)).await;
    assert!(plan.is_ok(), "{:?}", plan.err());
}

#[tokio::test]
async fn replaces_run_before_cond_2() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    struct C;
    migration!(C, "c", vec_box!(), vec_box!(B), vec_box!());
    struct D;
    migration!(D, "d", vec_box!(), vec_box!(C), vec_box!());
    struct E;
    migration!(E, "e", vec_box!(), vec_box!(), vec_box!(C));
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D, E)).await;
    assert!(plan.is_ok(), "{:?}", plan.err());
}

#[tokio::test]
async fn replaces_run_before_cond_3() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    struct C;
    migration!(C, "c", vec_box!(), vec_box!(B), vec_box!());
    struct D;
    migration!(D, "d", vec_box!(), vec_box!(C), vec_box!());
    struct E;
    migration!(E, "e", vec_box!(), vec_box!(), vec_box!(D));
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D, E)).await;
    assert!(plan.is_ok(), "{:?}", plan.err());
}

#[tokio::test]
async fn replaces_run_before_cond_4() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    struct C;
    migration!(C, "c", vec_box!(), vec_box!(B), vec_box!());
    struct D;
    migration!(D, "d", vec_box!(), vec_box!(C, E), vec_box!());
    struct E;
    migration!(E, "e", vec_box!(), vec_box!(), vec_box!(C));
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D, E)).await;
    assert!(plan.is_ok(), "{:?}", plan.err());
}

#[tokio::test]
async fn replaces_run_before_cond_5() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    struct C;
    migration!(C, "c", vec_box!(), vec_box!(B), vec_box!());
    struct D;
    migration!(D, "d", vec_box!(), vec_box!(C), vec_box!(C));
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D)).await;
    assert!(plan.is_err());
}

#[tokio::test]
async fn replaces_run_before_cond_6() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    struct C;
    migration!(C, "c", vec_box!(), vec_box!(B), vec_box!());
    struct D;
    migration!(D, "d", vec_box!(), vec_box!(C, E), vec_box!());
    struct E;
    migration!(E, "e", vec_box!(), vec_box!(D), vec_box!(C));
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D, E)).await;
    assert!(plan.is_err());
}

#[tokio::test]
async fn replaces_run_before_cond_7() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    struct C;
    migration!(C, "c", vec_box!(), vec_box!(B), vec_box!(D));
    struct D;
    migration!(D, "d", vec_box!(), vec_box!(C), vec_box!());
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D)).await;
    assert!(plan.is_err());
}

#[tokio::test]
async fn loop_error() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!(A));
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B)).await;
    assert!(plan.is_err());
}

#[tokio::test]
async fn parent_not_applied() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    let mut migrator = CustomMigrator::default();
    migrator.add_applied_migrations(vec_box!(B));
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B)).await;
    assert!(plan.is_err());
}

#[tokio::test]
async fn replace_grand_child() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    struct C;
    migration!(C, "c", vec_box!(), vec_box!(B), vec_box!());
    struct D;
    migration!(D, "d", vec_box!(), vec_box!(C), vec_box!());
    let mut migrator = CustomMigrator::default();
    migrator.add_applied_migrations(vec_box!(A, D));
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(D, C, B, A))
        .await
        .unwrap();
    assert!(plan.is_empty());
}

#[tokio::test]
async fn virtual_not_added() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, ("test", "b"))).await;
    assert!(plan.is_err());
}

#[tokio::test]
async fn virtual_replaced() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    let mut migrator = CustomMigrator::default();
    let plan =
        generate_apply_all_plan(&mut migrator, vec_box!(A, ("test", "b"), B, ("test", "b"))).await;
    assert!(plan.is_ok(), "{:?}", plan.err());
    assert!(plan.unwrap().len() == 2);
}

#[tokio::test]
async fn virtual_reference() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(("test", "a")), vec_box!(), vec_box!());
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(B, A)).await;
    assert!(plan.is_ok(), "{:?}", plan.err());
    assert!(plan.unwrap().len() == 2);
}

#[tokio::test]
async fn apply_plan_size_test() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    struct C;
    migration!(C, "c", vec_box!(B), vec_box!(), vec_box!());
    struct D;
    migration!(D, "d", vec_box!(B), vec_box!(), vec_box!());
    struct E;
    migration!(E, "e", vec_box!(C), vec_box!(), vec_box!());
    struct F;
    migration!(F, "f", vec_box!(D), vec_box!(), vec_box!());
    struct G;
    migration!(G, "g", vec_box!(E), vec_box!(), vec_box!());
    let mut migrator = CustomMigrator::default();
    migrator.add_migrations(vec_box!(A, B, C, D, E, F, G));
    let sqlite = SqlitePool::connect("sqlite::memory:").await.unwrap();
    let mut conn = sqlite.acquire().await.unwrap();
    let full_plan = migrator
        .generate_migration_plan(Some(&Plan::apply_all()), &mut conn)
        .await
        .unwrap();
    assert!(full_plan.len() == 7);
    let plan_till_f = migrator
        .generate_migration_plan(
            Some(&Plan::apply_name("test", &Some("f".to_string()))),
            &mut conn,
        )
        .await
        .unwrap();
    assert!(plan_till_f.len() == 4);
    let plan_till_g = migrator
        .generate_migration_plan(
            Some(&Plan::apply_name("test", &Some("g".to_string()))),
            &mut conn,
        )
        .await
        .unwrap();
    assert!(plan_till_g.len() == 5);
}

#[tokio::test]
async fn revert_plan_size_test() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    struct C;
    migration!(C, "c", vec_box!(B), vec_box!(), vec_box!());
    struct D;
    migration!(D, "d", vec_box!(B), vec_box!(), vec_box!());
    struct E;
    migration!(E, "e", vec_box!(C), vec_box!(), vec_box!());
    struct F;
    migration!(F, "f", vec_box!(D), vec_box!(), vec_box!());
    struct G;
    migration!(G, "g", vec_box!(E), vec_box!(), vec_box!());
    let mut migrator = CustomMigrator::default();
    migrator.add_migrations(vec_box!(A, B, C, D, E, F, G));
    migrator.add_applied_migrations(vec_box!(A, B, C, D, E, F, G));
    let sqlite = SqlitePool::connect("sqlite::memory:").await.unwrap();
    let mut conn = sqlite.acquire().await.unwrap();
    let apply_plan = migrator
        .generate_migration_plan(Some(&Plan::apply_all()), &mut conn)
        .await
        .unwrap();
    assert!(apply_plan.is_empty());
    let plan_till_f = migrator
        .generate_migration_plan(
            Some(&Plan::revert_name("test", &Some("f".to_string()))),
            &mut conn,
        )
        .await
        .unwrap();
    assert!(plan_till_f.len() == 1);
    let plan_till_c = migrator
        .generate_migration_plan(
            Some(&Plan::revert_name("test", &Some("c".to_string()))),
            &mut conn,
        )
        .await
        .unwrap();
    assert!(plan_till_c.len() == 3);
    let plan_till_b = migrator
        .generate_migration_plan(
            Some(&Plan::revert_name("test", &Some("b".to_string()))),
            &mut conn,
        )
        .await
        .unwrap();
    assert!(plan_till_b.len() == 6);
}
