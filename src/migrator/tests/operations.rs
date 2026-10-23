use super::*;
use crate::operation::{from_fn, from_fn_irreversible};

/// Migration of the `main` app running the function operations of the test.
struct FnMigration;

impl Migration<Sqlite> for FnMigration {
    fn app(&self) -> &'static str {
        "main"
    }

    fn name(&self) -> &'static str {
        "functions"
    }

    fn parents(&self) -> Vec<Box<dyn Migration<Sqlite>>> {
        vec![]
    }

    fn operations(&self) -> Vec<Box<dyn Operation<Sqlite>>> {
        vec![
            Box::new(from_fn::<Sqlite>(
                |connection| {
                    Box::pin(async move {
                        sqlx::Executor::execute(connection, "CREATE TABLE fn_sample (id INTEGER)")
                            .await?;
                        Ok(())
                    })
                },
                |connection| {
                    Box::pin(async move {
                        sqlx::Executor::execute(connection, "DROP TABLE fn_sample").await?;
                        Ok(())
                    })
                },
            )),
            Box::new(from_fn_irreversible::<Sqlite>(|connection| {
                Box::pin(async move {
                    sqlx::Executor::execute(connection, "CREATE TABLE fn_seed (id INTEGER)")
                        .await?;
                    Ok(())
                })
            })),
        ]
    }
}

/// Creates and drops a table with functions, and runs them through a migrator.
#[tokio::test]
async fn function_operations_run_up_and_down() {
    let mut migrator = Migrator::<Sqlite>::default();
    migrator.add_migration(Box::new(FnMigration)).unwrap();
    let mut conn = make_conn().await;

    migrator.run(&mut conn, &Plan::apply_all()).await.unwrap();
    assert!(sqlite_table_exists(&mut conn, "fn_sample").await);
    assert!(sqlite_table_exists(&mut conn, "fn_seed").await);

    // operations are reverted in reverse order and the irreversible one stops
    // the revert, which rolls the migration back
    assert!(matches!(
        migrator.run(&mut conn, &Plan::revert_all()).await,
        Err(Error::IrreversibleOperation)
    ));
    assert!(sqlite_table_exists(&mut conn, "fn_sample").await);
}

/// A function operation without a down function is irreversible, and the
/// destructible flag is passed on.
#[tokio::test]
async fn function_operation_flags() {
    let mut conn = make_conn().await;
    let up_only = from_fn_irreversible::<Sqlite>(|connection| {
        Box::pin(async move {
            sqlx::Executor::execute(connection, "SELECT 1").await?;
            Ok(())
        })
    });
    assert!(!up_only.is_destructible());
    up_only.up(&mut conn).await.unwrap();
    assert!(matches!(
        up_only.down(&mut conn).await,
        Err(Error::IrreversibleOperation)
    ));

    let both = from_fn::<Sqlite>(
        |connection| {
            Box::pin(async move {
                sqlx::Executor::execute(connection, "SELECT 1").await?;
                Ok(())
            })
        },
        |_| Box::pin(async { Err(Error::PendingMigrationPresent) }),
    )
    .destructible(true);
    assert!(both.is_destructible());
    assert!(matches!(
        both.down(&mut conn).await,
        Err(Error::PendingMigrationPresent)
    ));
}
