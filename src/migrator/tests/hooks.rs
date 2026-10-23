use super::*;
use crate::migrator::RunReport;

/// Hooks run around the run and each migration, for apply and revert, atomic
/// or not, and migration hooks are skipped by a fake plan.
#[tokio::test]
async fn hooks_run_around_run_and_migrations() {
    let mut migrator = migrator_of(vec![
        SqlMigration::new("first", None, "CREATE TABLE first (id INTEGER)"),
        SqlMigration::new("second", Some("first"), "CREATE TABLE second (id INTEGER)").non_atomic(),
    ]);
    let calls = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    for hook_type in [
        HookType::BeforeRun,
        HookType::AfterRun,
        HookType::BeforeMigration,
        HookType::AfterMigration,
    ] {
        let calls = calls.clone();
        migrator.add_hook(hook_type, move |_connection, context| {
            let calls = calls.clone();
            let call = format!(
                "{hook_type:?} {} {}",
                context
                    .migration()
                    .map_or("-", |migration| migration.name()),
                context.is_apply()
            );
            Box::pin(async move {
                calls.lock().unwrap().push(call);
                Ok(())
            })
        });
    }
    let mut conn = make_conn().await;

    migrator.run(&mut conn, &Plan::apply_all()).await.unwrap();
    assert_eq!(
        std::mem::take(&mut *calls.lock().unwrap()),
        [
            "BeforeRun - true",
            "BeforeMigration first true",
            "AfterMigration first true",
            "BeforeMigration second true",
            "AfterMigration second true",
            "AfterRun - true",
        ]
    );

    migrator.run(&mut conn, &Plan::revert_all()).await.unwrap();
    assert_eq!(
        std::mem::take(&mut *calls.lock().unwrap()),
        [
            "BeforeRun - false",
            "BeforeMigration second false",
            "AfterMigration second false",
            "BeforeMigration first false",
            "AfterMigration first false",
            "AfterRun - false",
        ]
    );

    migrator
        .run(&mut conn, &Plan::apply_all().fake(true))
        .await
        .unwrap();
    assert_eq!(
        std::mem::take(&mut *calls.lock().unwrap()),
        ["BeforeRun - true", "AfterRun - true"]
    );
}

/// A failing hook stops the run and rolls back the atomic migration it ran in.
#[tokio::test]
async fn failing_hook_rolls_back_migration() {
    let mut migrator = migrator_of(vec![SqlMigration::new(
        "first",
        None,
        "CREATE TABLE first (id INTEGER)",
    )]);
    migrator.add_hook(HookType::AfterMigration, |connection, _context| {
        Box::pin(async move {
            sqlx::Executor::execute(connection, "SELECT * FROM missing").await?;
            Ok(())
        })
    });
    let mut conn = make_conn().await;
    assert!(migrator.run(&mut conn, &Plan::apply_all()).await.is_err());
    assert!(!sqlite_table_exists(&mut conn, "first").await);
}

/// A plain function can be a hook and use the connection.
#[tokio::test]
async fn function_hook_uses_connection() {
    fn create_table<'a>(
        connection: &'a mut <Sqlite as Database>::Connection,
        _context: &'a HookContext<'a, Sqlite>,
    ) -> HookFuture<'a> {
        Box::pin(async move {
            sqlx::Executor::execute(connection, "CREATE TABLE hooked (id INTEGER)").await?;
            Ok(())
        })
    }
    let mut migrator = migrator_of(vec![SqlMigration::new(
        "first",
        None,
        "INSERT INTO hooked VALUES (1)",
    )]);
    migrator.add_hook(HookType::BeforeRun, create_table);
    migrator.add_hook(HookType::AfterRun, |connection, context| {
        let count = context.report().map_or(0, RunReport::len);
        Box::pin(async move {
            sqlx::Executor::execute(
                connection,
                sqlx::AssertSqlSafe(format!("INSERT INTO hooked VALUES ({})", count + 1)),
            )
            .await?;
            Ok(())
        })
    });
    let mut conn = make_conn().await;
    migrator.run(&mut conn, &Plan::apply_all()).await.unwrap();
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM hooked")
        .fetch_one(&mut *conn)
        .await
        .unwrap();
    assert_eq!(count, 2);
}
