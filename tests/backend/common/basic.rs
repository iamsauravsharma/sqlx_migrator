//! Scenarios: applying, reverting, syncing and pruning

use super::*;

/// Applies the chain, checks the migration table from a second connection,
/// reverts everything and drops the migration table.
pub(crate) async fn apply_and_revert<H>(harness: &H, test: &str)
where
    H: Harness,
    (String, String): Operation<H::Db>,
{
    let mut connection = harness.setup(test).await;
    let migrator = harness.migrator(test);
    assert_eq!(migrator.table_name(), harness.table_name(test));

    let report = migrator
        .run(&mut connection, &Plan::apply_all())
        .await
        .unwrap();
    assert_eq!(report.len(), 2);
    assert!(table_exists::<H::Db>(&mut connection, &format!("{test}_second")).await);
    assert_eq!(
        applied(&mut connection, &migrator).await,
        vec!["first", "second"]
    );
    let rows = migrator
        .fetch_applied_migration_from_db(&mut connection)
        .await
        .unwrap();
    assert!(rows[0].id() < rows[1].id());
    assert_ne!(rows[0].applied_time(), OffsetDateTime::UNIX_EPOCH);
    migrator.check_pending(&mut connection).await.unwrap();

    let mut other = harness.connect(test).await;
    assert_eq!(
        applied(&mut other, &migrator).await,
        vec!["first", "second"]
    );
    let report = migrator.run(&mut other, &Plan::revert_all()).await.unwrap();
    assert_eq!(report.len(), 2);
    assert!(!table_exists::<H::Db>(&mut other, &format!("{test}_first")).await);
    assert_eq!(applied(&mut other, &migrator).await, Vec::<String>::new());
    assert!(matches!(
        migrator.check_pending(&mut other).await,
        Err(Error::PendingMigrationPresent)
    ));

    migrator
        .drop_migration_table_if_exists(&mut other)
        .await
        .unwrap();
    assert!(!table_exists::<H::Db>(&mut other, &migrator.table_name()).await);
    harness.teardown(test);
}

/// A fake run records rows without executing operations.
pub(crate) async fn fake_apply<H>(harness: &H, test: &str)
where
    H: Harness,
    (String, String): Operation<H::Db>,
{
    let mut connection = harness.setup(test).await;
    let migrator = harness.migrator(test);
    let report = migrator
        .run(&mut connection, &Plan::apply_all().fake(true))
        .await
        .unwrap();
    assert!(report.is_fake());
    assert_eq!(
        applied(&mut connection, &migrator).await,
        vec!["first", "second"]
    );
    assert!(!table_exists::<H::Db>(&mut connection, &format!("{test}_first")).await);
    harness.teardown(test);
}

/// A failing atomic migration never records a migration table row. Whether its
/// DDL survives depends on the backend.
pub(crate) async fn failed_atomic_migration<H>(harness: &H, test: &str)
where
    H: Harness,
    (String, String): Operation<H::Db>,
{
    let mut connection = harness.setup(test).await;
    let migrator = harness.migrator(test);
    let mut migrator = migrator;
    migrator
        .add_migration(Box::new(
            SqlMigration::new("main", "failing")
                .parent("main", "second")
                .operation(
                    &format!("CREATE TABLE {test}_failed (id INTEGER PRIMARY KEY)"),
                    &format!("DROP TABLE {test}_failed"),
                )
                .operation("THIS IS NOT SQL", "SELECT 1"),
        ))
        .unwrap();
    assert!(matches!(
        migrator.run(&mut connection, &Plan::apply_all()).await,
        Err(Error::Sqlx(_))
    ));
    assert_eq!(
        table_exists::<H::Db>(&mut connection, &format!("{test}_failed")).await,
        !harness.transactional_ddl()
    );
    assert_eq!(
        applied(&mut connection, &migrator).await,
        vec!["first", "second"]
    );
    harness.teardown(test);
}

/// Syncing from an old migrator records only migrations the new migrator
/// knows and is idempotent.
pub(crate) async fn sync<H>(harness: &H, test: &str)
where
    H: Harness,
    (String, String): Operation<H::Db>,
{
    let mut connection = harness.setup(test).await;
    let migrator = harness.migrator(test);
    let old = OldMigratorStub(vec![
        ("main".to_string(), "first".to_string()),
        ("main".to_string(), "unknown".to_string()),
    ]);
    migrator.sync(&mut connection, &old).await.unwrap();
    migrator.sync(&mut connection, &old).await.unwrap();
    assert_eq!(applied(&mut connection, &migrator).await, vec!["first"]);
    let report = migrator
        .run(&mut connection, &Plan::apply_all().fake(true))
        .await
        .unwrap();
    assert_eq!(report.len(), 1);
    assert_eq!(report.records()[0].name(), "second");
    harness.teardown(test);
}

/// Prune removes migration table rows without a matching migration and
/// nothing else, and the table rejects duplicate rows.
pub(crate) async fn prune<H>(harness: &H, test: &str)
where
    H: Harness,
    (String, String): Operation<H::Db>,
{
    let mut connection = harness.setup(test).await;
    let migrator = harness.migrator(test);
    migrator
        .run(&mut connection, &Plan::apply_all().fake(true))
        .await
        .unwrap();
    insert_row::<H::Db>(&mut connection, &migrator.table_name(), "ghost", "removed")
        .await
        .unwrap();
    assert_eq!(migrator.prune(&mut connection).await.unwrap(), 1);
    assert_eq!(migrator.prune(&mut connection).await.unwrap(), 0);
    assert_eq!(
        applied(&mut connection, &migrator).await,
        vec!["first", "second"]
    );
    assert!(matches!(
        insert_row::<H::Db>(&mut connection, &migrator.table_name(), "main", "first").await,
        Err(Error::Sqlx(_))
    ));
    harness.teardown(test);
}

/// A grouped plan which fails leaves nothing behind where DDL is rolled back,
/// and one which succeeds applies everything. A backend which commits DDL
/// implicitly keeps what was committed before the failure.
pub(crate) async fn grouped_plan<H>(harness: &H, test: &str)
where
    H: Harness,
    (String, String): Operation<H::Db>,
{
    let mut connection = harness.setup(test).await;
    let mut migrator = harness.migrator(test);
    migrator
        .add_migration(Box::new(
            SqlMigration::new("main", "failing")
                .parent("main", "second")
                .operation("THIS IS NOT SQL", "SELECT 1"),
        ))
        .unwrap();
    let plan = Plan::apply_all().grouped(true);
    assert!(matches!(
        migrator.run(&mut connection, &plan).await,
        Err(Error::Sqlx(_))
    ));
    if !harness.transactional_ddl() {
        harness.teardown(test);
        return;
    }
    assert_eq!(
        applied(&mut connection, &migrator).await,
        Vec::<String>::new()
    );
    assert!(!table_exists::<H::Db>(&mut connection, &format!("{test}_first")).await);

    let migrator = harness.migrator(test);
    let report = migrator.run(&mut connection, &plan).await.unwrap();
    assert_eq!(report.len(), 2);
    assert_eq!(
        applied(&mut connection, &migrator).await,
        vec!["first", "second"]
    );
    harness.teardown(test);
}
