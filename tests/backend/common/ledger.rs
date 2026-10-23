//! Scenarios: the migration table and failed migrations

use super::*;

/// The meta table is created, rejects names this crate does not know,
/// a migration table without a meta table keeps its rows and gets the other
/// columns,
/// and both tables are dropped together.
pub(crate) async fn internal_migrations<H>(harness: &H, test: &str)
where
    H: Harness,
    (String, String): Operation<H::Db>,
{
    let mut connection = harness.setup(test).await;
    let migrator = harness.migrator(test);
    let meta_table = migrator.meta_table_name();

    // fetching creates both tables
    assert_eq!(
        applied(&mut connection, &migrator).await,
        Vec::<String>::new()
    );
    assert!(table_exists::<H::Db>(&mut connection, &meta_table).await);

    // a name from a newer release is rejected until it is removed
    H::Db::execute(
        &mut connection,
        &format!("INSERT INTO {meta_table}(name) VALUES ('9999_from_the_future')"),
    )
    .await
    .unwrap();
    assert!(matches!(
        migrator.run(&mut connection, &Plan::apply_all()).await,
        Err(Error::UnknownInternalMigration { name }) if name == "9999_from_the_future"
    ));

    // a migration table with only the first columns and no meta table keeps
    // its rows and gets the other columns
    H::Db::execute(
        &mut connection,
        &format!(
            "DROP TABLE {}; DROP TABLE {meta_table}",
            migrator.table_name()
        ),
    )
    .await
    .unwrap();
    H::Db::execute(
        &mut connection,
        &harness.legacy_table_sql(&migrator.table_name()),
    )
    .await
    .unwrap();
    insert_row::<H::Db>(&mut connection, &migrator.table_name(), "legacy", "row")
        .await
        .unwrap();
    let rows = migrator
        .fetch_applied_migration_from_db(&mut connection)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name(), "row");
    assert!(rows[0].success());
    assert_eq!(rows[0].batch(), rows[0].id());
    H::Db::execute(
        &mut connection,
        &format!("DELETE FROM {} WHERE app = 'legacy'", migrator.table_name()),
    )
    .await
    .unwrap();

    migrator
        .run(&mut connection, &Plan::apply_all())
        .await
        .unwrap();
    migrator
        .run(&mut connection, &Plan::revert_all())
        .await
        .unwrap();
    migrator
        .drop_migration_table_if_exists(&mut connection)
        .await
        .unwrap();
    assert!(!table_exists::<H::Db>(&mut connection, &meta_table).await);
    harness.teardown(test);
}

/// Applied rows record time and batch. A non atomic migration which
/// fails part way stays recorded as failed and blocks runs until it is
/// repaired. The last batch can be reverted.
pub(crate) async fn failed_migration<H>(harness: &H, test: &str)
where
    H: Harness,
    (String, String): Operation<H::Db>,
{
    let mut connection = harness.setup(test).await;
    let mut migrator = harness.migrator(test);
    migrator
        .add_migration(Box::new(
            SqlMigration::new("main", "third")
                .parent("main", "second")
                .non_atomic()
                .operation(
                    &format!("CREATE TABLE {test}_failed (id INTEGER PRIMARY KEY)"),
                    &format!("DROP TABLE {test}_failed"),
                )
                .operation("THIS IS NOT SQL", "SELECT 1"),
        ))
        .unwrap();

    migrator
        .run(&mut connection, &Plan::apply_count(2))
        .await
        .unwrap();
    let rows = migrator
        .fetch_applied_migration_from_db(&mut connection)
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    for row in &rows {
        assert!(row.success());
        assert_eq!(row.batch(), 1);
        assert!(row.execution_time_ns() >= 0);
    }

    assert!(matches!(
        migrator.run(&mut connection, &Plan::apply_all()).await,
        Err(Error::Sqlx(_))
    ));
    let rows = migrator
        .fetch_applied_migration_from_db(&mut connection)
        .await
        .unwrap();
    let third = rows.iter().find(|row| row.name() == "third").unwrap();
    assert!(!third.success());
    assert_eq!(third.batch(), 2);
    assert!(matches!(
        migrator.run(&mut connection, &Plan::apply_all()).await,
        Err(Error::FailedMigration { name, .. }) if name == "third"
    ));

    assert_eq!(migrator.repair(&mut connection).await.unwrap(), 1);
    assert_eq!(
        applied(&mut connection, &migrator).await,
        vec!["first", "second"]
    );
    assert_eq!(migrator.repair(&mut connection).await.unwrap(), 0);

    let report = migrator
        .run(&mut connection, &Plan::revert_last_batch())
        .await
        .unwrap();
    assert_eq!(report.len(), 2);
    assert_eq!(
        applied(&mut connection, &migrator).await,
        Vec::<String>::new()
    );
    harness.teardown(test);
}
