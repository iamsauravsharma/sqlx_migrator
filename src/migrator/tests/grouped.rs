use super::*;

fn failing_chain() -> Migrator<Sqlite> {
    migrator_of(vec![
        SqlMigration::new("first", None, "CREATE TABLE first (id INTEGER)"),
        SqlMigration::new("second", Some("first"), "SELECT * FROM missing"),
    ])
}

/// Without a group the migrations before the failing one stay applied, with a
/// group nothing is.
#[tokio::test]
async fn grouped_plan_is_applied_entirely_or_not_at_all() {
    let migrator = failing_chain();
    let mut conn = make_conn().await;
    assert!(migrator.run(&mut conn, &Plan::apply_all()).await.is_err());
    assert!(sqlite_table_exists(&mut conn, "first").await);

    let migrator = failing_chain();
    let mut conn = make_conn().await;
    let plan = Plan::apply_all().grouped(true);
    assert!(migrator.run(&mut conn, &plan).await.is_err());
    assert!(!sqlite_table_exists(&mut conn, "first").await);
    let rows = migrator
        .fetch_applied_migration_from_db(&mut conn)
        .await
        .unwrap();
    assert!(rows.is_empty());
}

/// A grouped plan which succeeds commits every migration as one batch, and a
/// grouped revert which fails reverts nothing.
#[tokio::test]
async fn grouped_plan_commits_one_batch_and_reverts_together() {
    let migrator = migrator_of(vec![
        SqlMigration::new("first", None, "CREATE TABLE first (id INTEGER)"),
        SqlMigration::new("second", Some("first"), "CREATE TABLE second (id INTEGER)")
            .failing_down(),
    ]);
    let mut conn = make_conn().await;
    let report = migrator
        .run(&mut conn, &Plan::apply_all().grouped(true))
        .await
        .unwrap();
    assert_eq!(report.len(), 2);
    let rows = migrator
        .fetch_applied_migration_from_db(&mut conn)
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].batch(), rows[1].batch());

    // the first revert works, the second fails and takes the first back with it
    let plan = Plan::revert_all().grouped(true);
    assert!(migrator.run(&mut conn, &plan).await.is_err());
    assert!(sqlite_table_exists(&mut conn, "first").await);
    assert!(sqlite_table_exists(&mut conn, "second").await);
    let rows = migrator
        .fetch_applied_migration_from_db(&mut conn)
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
}

/// A plan with a migration which is not atomic cannot be grouped, unless it is
/// fake since nothing runs then.
#[tokio::test]
async fn grouped_plan_needs_atomic_migrations() {
    let migrator = migrator_of(vec![
        SqlMigration::new("first", None, "CREATE TABLE first (id INTEGER)"),
        SqlMigration::new("second", Some("first"), "CREATE TABLE second (id INTEGER)").non_atomic(),
    ]);
    let mut conn = make_conn().await;
    let error = migrator
        .run(&mut conn, &Plan::apply_all().grouped(true))
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "plan error: a grouped plan runs in one transaction so every migration must be atomic"
    );
    assert!(!sqlite_table_exists(&mut conn, "first").await);

    let report = migrator
        .run(&mut conn, &Plan::apply_all().grouped(true).fake(true))
        .await
        .unwrap();
    assert_eq!(report.len(), 2);
    assert!(!sqlite_table_exists(&mut conn, "first").await);
}

/// Hooks of a grouped run work inside the transaction of the group, so a
/// failing last hook takes the whole plan back.
#[tokio::test]
async fn grouped_plan_runs_hooks_inside_the_transaction() {
    let mut migrator = migrator_of(vec![SqlMigration::new(
        "first",
        None,
        "CREATE TABLE first (id INTEGER)",
    )]);
    migrator.add_hook(HookType::AfterRun, |connection, _context| {
        Box::pin(async move {
            sqlx::Executor::execute(connection, "SELECT * FROM missing").await?;
            Ok(())
        })
    });
    let mut conn = make_conn().await;
    let plan = Plan::apply_all().grouped(true);
    assert!(migrator.run(&mut conn, &plan).await.is_err());
    assert!(!sqlite_table_exists(&mut conn, "first").await);
}
