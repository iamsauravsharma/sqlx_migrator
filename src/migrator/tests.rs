use sqlx::{Database, Sqlite, SqlitePool};

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

    #[expect(clippy::borrowed_box)]
    fn add_applied_migration(&mut self, migration: &Box<dyn Migration<Sqlite>>) {
        let current_length = self.migrations.len();
        self.applied_migrations.push(AppliedMigrationSqlRow::new(
            i32::try_from(current_length).unwrap(),
            migration.app(),
            migration.name(),
        ));
    }
}

impl Info<Sqlite> for CustomMigrator {
    fn migrations(&self) -> &Vec<Box<dyn Migration<Sqlite>>> {
        &self.migrations
    }

    fn migrations_mut(&mut self) -> &mut Vec<Box<dyn Migration<Sqlite>>> {
        &mut self.migrations
    }
}

#[async_trait::async_trait]
impl DatabaseOperation<Sqlite> for CustomMigrator {
    async fn ensure_migration_table_exists(
        &self,
        _connection: &mut <Sqlite as Database>::Connection,
    ) -> Result<(), Error> {
        Ok(())
    }

    async fn drop_migration_table_if_exists(
        &self,
        _connection: &mut <Sqlite as Database>::Connection,
    ) -> Result<(), Error> {
        Ok(())
    }

    async fn add_migration_to_db_table(
        &self,
        _connection: &mut <Sqlite as Database>::Connection,
        _migration: &Box<dyn Migration<Sqlite>>,
    ) -> Result<(), Error> {
        Ok(())
    }

    async fn delete_migration_from_db_table(
        &self,
        _connection: &mut <Sqlite as Database>::Connection,
        _migration: &Box<dyn Migration<Sqlite>>,
    ) -> Result<(), Error> {
        Ok(())
    }

    async fn fetch_applied_migration_from_db(
        &self,
        _connection: &mut <Sqlite as Database>::Connection,
    ) -> Result<Vec<AppliedMigrationSqlRow>, Error> {
        Ok(self.applied_migrations.clone())
    }

    async fn lock(&self, _connection: &mut <Sqlite as Database>::Connection) -> Result<(), Error> {
        Ok(())
    }

    async fn unlock(
        &self,
        _connection: &mut <Sqlite as Database>::Connection,
    ) -> Result<(), Error> {
        Ok(())
    }
}

impl Migrate<Sqlite> for CustomMigrator {}

macro_rules! migration {
    ($op:ty, $name:expr, $parents:expr, $replaces:expr, $run_before:expr) => {
        #[async_trait::async_trait]
        impl crate::migration::Migration<sqlx::Sqlite> for $op {
            fn app(&self) -> &str {
                "test"
            }

            fn name(&self) -> &str {
                $name
            }

            fn parents(&self) -> Vec<Box<dyn crate::migration::Migration<sqlx::Sqlite>>> {
                $parents
            }

            fn operations(&self) -> Vec<Box<dyn crate::operation::Operation<sqlx::Sqlite>>> {
                vec![]
            }

            fn replaces(&self) -> Vec<Box<dyn crate::migration::Migration<sqlx::Sqlite>>> {
                $replaces
            }

            fn run_before(&self) -> Vec<Box<dyn crate::migration::Migration<sqlx::Sqlite>>> {
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
        .generate_migration_plan(&mut conn, Some(&Plan::apply_all()))
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
    let mut plan_iter = plan.iter();
    assert!(plan_iter.next() == Some(&&(Box::new(A) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_iter.next() == Some(&&(Box::new(B) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_iter.next() == Some(&&(Box::new(C) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_iter.next().is_none());
}

#[tokio::test]
async fn no_migration() {
    struct _A;
    migration!(_A, "a", vec_box!(), vec_box!(), vec_box!());
    struct _B;
    migration!(_B, "b", vec_box!(_A), vec_box!(), vec_box!());
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!()).await;
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: no migration are added to migration list".to_string())
    );
}

#[tokio::test]
async fn interrelated_test() {
    struct A;
    migration!(A, "a", vec_box!(B), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B)).await;
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: reached deadlock stage during plan generation".to_string())
    );
}

#[tokio::test]
async fn run_before_interrelated_test() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(B), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(), vec_box!(A), vec_box!());
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B)).await;
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: two migrations replaces each other".to_string())
    );
}

#[tokio::test]
async fn replace_interrelated_test() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!(B));
    struct B;
    migration!(B, "b", vec_box!(), vec_box!(), vec_box!(A));
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B)).await;
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: reached deadlock stage during plan generation".to_string())
    );
}

#[tokio::test]
async fn depend_on_itself() {
    struct A;
    migration!(A, "a", vec_box!(A), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(B), vec_box!(), vec_box!());
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B)).await;
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: reached deadlock stage during plan generation".to_string())
    );
}

#[tokio::test]
async fn run_before_depend_on_itself() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(A), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(), vec_box!(B), vec_box!());
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B)).await;
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: two migrations replaces each other".to_string())
    );
}

#[tokio::test]
async fn replace_depend_on_itself() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!(A));
    struct B;
    migration!(B, "b", vec_box!(), vec_box!(), vec_box!(B));
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B)).await;
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: reached deadlock stage during plan generation".to_string())
    );
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
    let mut plan_iter = plan.iter();
    assert!(plan_iter.next() == Some(&&(Box::new(A) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_iter.next() == Some(&&(Box::new(D) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_iter.next().is_none());
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
    let mut plan_iter = plan.iter();
    assert!(plan_iter.next() == Some(&&(Box::new(A) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_iter.next() == Some(&&(Box::new(D) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_iter.next() == Some(&&(Box::new(C) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_iter.next() == Some(&&(Box::new(B) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_iter.next().is_none());
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
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: migration test:b replaced multiple times".to_string())
    );
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
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D))
        .await
        .unwrap();
    let mut plan_iter = plan.iter();
    assert!(plan_iter.next() == Some(&&(Box::new(A) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_iter.next() == Some(&&(Box::new(D) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_iter.next() == Some(&&(Box::new(C) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_iter.next().is_none());
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
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D, E))
        .await
        .unwrap();
    let mut plan_iter = plan.iter();
    assert!(plan_iter.next() == Some(&&(Box::new(A) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_iter.next() == Some(&&(Box::new(E) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_iter.next() == Some(&&(Box::new(D) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_iter.next().is_none());
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
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D, E))
        .await
        .unwrap();
    let mut plan_iter = plan.iter();
    assert!(plan_iter.next() == Some(&&(Box::new(A) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_iter.next() == Some(&&(Box::new(E) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_iter.next() == Some(&&(Box::new(D) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_iter.next().is_none());
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
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D, E))
        .await
        .unwrap();
    let mut plan_iter = plan.iter();
    assert!(plan_iter.next() == Some(&&(Box::new(A) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_iter.next() == Some(&&(Box::new(D) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_iter.next().is_none());
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
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: reached deadlock stage during plan generation".to_string())
    );
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
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: two migrations replaces each other".to_string())
    );
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
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: reached deadlock stage during plan generation".to_string())
    );
}

#[tokio::test]
async fn loop_error() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!(A));
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B)).await;
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: reached deadlock stage during plan generation".to_string())
    );
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
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some(
            "plan error: children migration test:b applied before its parent migration test:a"
                .to_string()
        )
    );
}

#[tokio::test]
async fn replace_grand_child_applied() {
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
async fn replace_detailed_virtual() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(
        B,
        "b",
        vec_box!((A.app(), A.name())),
        vec_box!(),
        vec_box!()
    );
    struct C;
    migration!(
        C,
        "c",
        vec_box!(),
        vec_box!((B.app(), B.name())),
        vec_box!()
    );
    struct D;
    migration!(
        D,
        "d",
        vec_box!(),
        vec_box!((C.app(), C.name())),
        vec_box!()
    );
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(B, C, D, A))
        .await
        .unwrap();
    let mut plan_iter = plan.iter();
    assert!(plan_iter.next() == Some(&&(Box::new(A) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_iter.next() == Some(&&(Box::new(D) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_iter.next().is_none());
}

#[tokio::test]
async fn virtual_not_added() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, ("test", "b"))).await;
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: virtual migrations which is not replaced is present".to_string())
    );
}

#[tokio::test]
async fn virtual_replaced() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, ("test", "b"), B, ("test", "b")))
        .await
        .unwrap();
    let mut plan_iter = plan.iter();
    assert!(plan_iter.next() == Some(&&(Box::new(A) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_iter.next() == Some(&&(Box::new(B) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_iter.next().is_none());
}

#[tokio::test]
async fn virtual_reference() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(("test", "a")), vec_box!(), vec_box!());
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(B, A))
        .await
        .unwrap();
    assert_eq!(plan.len(), 2);
}

#[tokio::test]
async fn apply_virtual_plan_size() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(
        B,
        "b",
        vec_box!((A.app(), A.name())),
        vec_box!(),
        vec_box!()
    );
    struct C;
    migration!(
        C,
        "c",
        vec_box!((B.app(), B.name())),
        vec_box!(),
        vec_box!()
    );
    struct D;
    migration!(
        D,
        "d",
        vec_box!((B.app(), B.name())),
        vec_box!(),
        vec_box!()
    );
    struct E;
    migration!(
        E,
        "e",
        vec_box!((C.app(), C.name())),
        vec_box!(),
        vec_box!()
    );
    struct F;
    migration!(
        F,
        "f",
        vec_box!((D.app(), D.name())),
        vec_box!(),
        vec_box!()
    );
    struct G;
    migration!(
        G,
        "g",
        vec_box!((E.app(), E.name())),
        vec_box!(),
        vec_box!()
    );
    let mut migrator = CustomMigrator::default();
    migrator.add_migrations(vec_box!(A, B, C, D, E, F, G));
    let sqlite = SqlitePool::connect("sqlite::memory:").await.unwrap();
    let mut conn = sqlite.acquire().await.unwrap();
    let full_plan = migrator
        .generate_migration_plan(&mut conn, Some(&Plan::apply_all()))
        .await
        .unwrap();
    let mut full_plan_iter = full_plan.iter();
    assert!(full_plan_iter.next() == Some(&&(Box::new(A) as Box<dyn Migration<Sqlite>>)));
    assert!(full_plan_iter.next() == Some(&&(Box::new(B) as Box<dyn Migration<Sqlite>>)));
    assert!(full_plan_iter.next() == Some(&&(Box::new(C) as Box<dyn Migration<Sqlite>>)));
    assert!(full_plan_iter.next() == Some(&&(Box::new(D) as Box<dyn Migration<Sqlite>>)));
    assert!(full_plan_iter.next() == Some(&&(Box::new(E) as Box<dyn Migration<Sqlite>>)));
    assert!(full_plan_iter.next() == Some(&&(Box::new(F) as Box<dyn Migration<Sqlite>>)));
    assert!(full_plan_iter.next() == Some(&&(Box::new(G) as Box<dyn Migration<Sqlite>>)));
    assert!(full_plan_iter.next().is_none());
    let plan_till_f = migrator
        .generate_migration_plan(
            &mut conn,
            Some(&Plan::apply_name("test", &Some("f".to_string()))),
        )
        .await
        .unwrap();
    let mut plan_till_f_iter = plan_till_f.iter();
    assert!(plan_till_f_iter.next() == Some(&&(Box::new(A) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_till_f_iter.next() == Some(&&(Box::new(B) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_till_f_iter.next() == Some(&&(Box::new(D) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_till_f_iter.next() == Some(&&(Box::new(F) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_till_f_iter.next().is_none());
    let plan_till_g = migrator
        .generate_migration_plan(
            &mut conn,
            Some(&Plan::apply_name("test", &Some("g".to_string()))),
        )
        .await
        .unwrap();
    let mut plan_till_g_iter = plan_till_g.iter();
    assert!(plan_till_g_iter.next() == Some(&&(Box::new(A) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_till_g_iter.next() == Some(&&(Box::new(B) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_till_g_iter.next() == Some(&&(Box::new(C) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_till_g_iter.next() == Some(&&(Box::new(E) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_till_g_iter.next() == Some(&&(Box::new(G) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_till_g_iter.next().is_none());
}

#[tokio::test]
async fn revert_virtual_plan_size() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(
        B,
        "b",
        vec_box!((A.app(), A.name())),
        vec_box!(),
        vec_box!()
    );
    struct C;
    migration!(
        C,
        "c",
        vec_box!((B.app(), B.name())),
        vec_box!(),
        vec_box!()
    );
    struct D;
    migration!(
        D,
        "d",
        vec_box!((B.app(), B.name())),
        vec_box!(),
        vec_box!()
    );
    struct E;
    migration!(
        E,
        "e",
        vec_box!((C.app(), C.name())),
        vec_box!(),
        vec_box!()
    );
    struct F;
    migration!(
        F,
        "f",
        vec_box!((D.app(), D.name())),
        vec_box!(),
        vec_box!()
    );
    struct G;
    migration!(
        G,
        "g",
        vec_box!((E.app(), E.name())),
        vec_box!(),
        vec_box!()
    );
    let mut migrator = CustomMigrator::default();
    migrator.add_migrations(vec_box!(A, B, C, D, E, F, G));
    migrator.add_applied_migrations(vec_box!(A, B, C, D, E, F, G));
    let sqlite = SqlitePool::connect("sqlite::memory:").await.unwrap();
    let mut conn = sqlite.acquire().await.unwrap();
    let revert_plan = migrator
        .generate_migration_plan(&mut conn, Some(&Plan::revert_all()))
        .await
        .unwrap();
    let mut revert_plan_iter = revert_plan.iter();
    assert!(revert_plan_iter.next() == Some(&&(Box::new(G) as Box<dyn Migration<Sqlite>>)));
    assert!(revert_plan_iter.next() == Some(&&(Box::new(F) as Box<dyn Migration<Sqlite>>)));
    assert!(revert_plan_iter.next() == Some(&&(Box::new(E) as Box<dyn Migration<Sqlite>>)));
    assert!(revert_plan_iter.next() == Some(&&(Box::new(D) as Box<dyn Migration<Sqlite>>)));
    assert!(revert_plan_iter.next() == Some(&&(Box::new(C) as Box<dyn Migration<Sqlite>>)));
    assert!(revert_plan_iter.next() == Some(&&(Box::new(B) as Box<dyn Migration<Sqlite>>)));
    assert!(revert_plan_iter.next() == Some(&&(Box::new(A) as Box<dyn Migration<Sqlite>>)));
    assert!(revert_plan_iter.next().is_none());
    let revert_till_f = Plan::revert_name("test", &Some("f".to_string()));
    let plan_till_f = migrator
        .generate_migration_plan(&mut conn, Some(&revert_till_f))
        .await
        .unwrap();
    let mut plan_till_f_iter = plan_till_f.iter();
    assert!(plan_till_f_iter.next() == Some(&&(Box::new(F) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_till_f_iter.next().is_none());
    let revert_till_c = Plan::revert_name("test", &Some("c".to_string()));
    let plan_till_c = migrator
        .generate_migration_plan(&mut conn, Some(&revert_till_c))
        .await
        .unwrap();
    let mut plan_till_c_iter = plan_till_c.iter();
    assert!(plan_till_c_iter.next() == Some(&&(Box::new(G) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_till_c_iter.next() == Some(&&(Box::new(E) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_till_c_iter.next() == Some(&&(Box::new(C) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_till_c_iter.next().is_none());
    let revert_till_b = Plan::revert_name("test", &Some("b".to_string()));
    let plan_till_b = migrator
        .generate_migration_plan(&mut conn, Some(&revert_till_b))
        .await
        .unwrap();
    let mut plan_till_b_iter = plan_till_b.iter();
    assert!(plan_till_b_iter.next() == Some(&&(Box::new(G) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_till_b_iter.next() == Some(&&(Box::new(F) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_till_b_iter.next() == Some(&&(Box::new(E) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_till_b_iter.next() == Some(&&(Box::new(D) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_till_b_iter.next() == Some(&&(Box::new(C) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_till_b_iter.next() == Some(&&(Box::new(B) as Box<dyn Migration<Sqlite>>)));
    assert!(plan_till_b_iter.next().is_none());
}
