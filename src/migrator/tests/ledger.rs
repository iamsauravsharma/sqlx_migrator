use super::*;

// ── Sync tests
// ────────────────────────────────────────────────────────────────

/// `Migrator::sync` must complete without error when the old migrator has
/// migrations that are a subset of those registered in the new migrator.
#[tokio::test]
async fn sync_completes_without_error() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());

    // Old migrator: A is applied.
    let mut old_migrator = TestMigrator::default();
    old_migrator.add_migration(Box::new(A)).unwrap();
    old_migrator.add_applied_migration(&A);

    // New migrator knows about both A and B.
    let mut new_migrator = TestMigrator::default();
    new_migrator.add_migrations(vec_box!(A, B)).unwrap();

    let mut conn = make_conn().await;
    new_migrator
        .sync(&mut conn, &old_migrator)
        .await
        .expect("sync must complete without error");
    let rows = new_migrator
        .fetch_applied_migration_from_db(&mut conn)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name(), "a");
    // synced migrations have no execution time and form a batch
    assert_eq!(rows[0].execution_time_ns(), 0);
    assert_eq!(rows[0].batch(), 1);
}

/// `Migrator::sync` must silently skip migrations from the old migrator that
/// are not present in the new migrator's migration list.
#[tokio::test]
async fn sync_skips_unknown_migrations() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());

    // Old migrator: A and B are applied.
    let mut old_migrator = TestMigrator::default();
    old_migrator.add_migrations(vec_box!(A, B)).unwrap();
    old_migrator.add_applied_migrations(vec_box!(A, B));

    // New migrator only knows about A (not B).
    let mut new_migrator = TestMigrator::default();
    new_migrator.add_migration(Box::new(A)).unwrap();

    // Sync must not fail even though B is unknown to the new migrator.
    let mut conn = make_conn().await;
    new_migrator
        .sync(&mut conn, &old_migrator)
        .await
        .expect("sync must skip unknown migrations without error");
    let rows = new_migrator
        .fetch_applied_migration_from_db(&mut conn)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name(), "a");
}

/// A dialect carries the table prefix and schema into the migrator.
#[tokio::test]
async fn new_with_dialect_adjusts_built_in_sql() {
    let dialect = Dialect::sqlite_with(Some("pfx"), Some("sch")).unwrap();
    let migrator = Migrator::<Sqlite>::new(dialect);
    assert_eq!(&migrator.table_name(), "sch._pfx_sqlx_migrator_migrations");
    let mut conn = make_conn().await;
    let locked = migrator.lock(&mut conn).await.unwrap();
    migrator.unlock(locked, true).await.unwrap();
}

/// Creating the schema is on for Postgres and can be switched off.
#[cfg(feature = "postgres")]
#[test]
fn create_schema_can_be_disabled() {
    let dialect = Dialect::postgres_with(None, Some("sch")).unwrap();
    assert!(dialect.create_schema());
    assert!(!dialect.set_create_schema(false).create_schema());
}

#[test]
fn meta_table_name_follows_prefix_and_schema() {
    let migrator = Migrator::<Sqlite>::default();
    assert_eq!(&migrator.meta_table_name(), "_sqlx_migrator_meta");
    let migrator = Migrator::<Sqlite>::new(Dialect::sqlite_with(Some("pfx"), Some("sch")).unwrap());
    assert_eq!(&migrator.meta_table_name(), "sch._pfx_sqlx_migrator_meta");
    assert_eq!(&migrator.table_name(), "sch._pfx_sqlx_migrator_migrations");
}

/// A fresh install creates both tables and records the internal migration;
/// ensuring again is a no-op.
#[tokio::test]
async fn fresh_install_creates_both_tables() {
    let migrator = Migrator::<Sqlite>::default();
    let mut conn = make_conn().await;
    migrator
        .ensure_migration_table_exists(&mut conn)
        .await
        .unwrap();
    assert!(sqlite_table_exists(&mut conn, &migrator.table_name()).await);
    assert!(sqlite_table_exists(&mut conn, &migrator.meta_table_name()).await);
    let (count,): (i64,) = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT COUNT(*) FROM {}",
        migrator.meta_table_name()
    )))
    .fetch_one(&mut *conn)
    .await
    .unwrap();
    assert_eq!(
        usize::try_from(count).unwrap(),
        migrator.dialect().sql().internal.steps().len()
    );
    migrator
        .ensure_migration_table_exists(&mut conn)
        .await
        .unwrap();
}

/// A migration table without a meta table keeps its rows when the internal
/// migrations run.
#[tokio::test]
async fn legacy_install_keeps_existing_rows() {
    let migrator = Migrator::<Sqlite>::default();
    let mut conn = make_conn().await;
    let first_layout = migrator.dialect().sql().internal.steps()[0].1.clone();
    raw(&mut conn, first_layout).await;
    raw(
        &mut conn,
        format!(
            "INSERT INTO {}(app, name) VALUES ('legacy', 'row'), ('legacy', 'next')",
            migrator.table_name()
        ),
    )
    .await;

    migrator
        .ensure_migration_table_exists(&mut conn)
        .await
        .unwrap();
    let rows = migrator
        .fetch_applied_migration_from_db(&mut conn)
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].app(), "legacy");
    // columns added by internal migrations get their defaults
    assert!(rows[0].success());
    assert_eq!(rows[0].execution_time_ns(), 0);
    // existing rows get a batch each, in apply order
    assert_eq!(rows[0].batch(), 1);
    assert_eq!(rows[1].batch(), 2);
}

/// A migration table with only the first internal migration recorded gets the
/// other columns and keeps its rows.
#[tokio::test]
async fn first_layout_with_meta_table_gets_new_columns() {
    let migrator = Migrator::<Sqlite>::default();
    let mut conn = make_conn().await;
    let first_layout = migrator.dialect().sql().internal.steps()[0].1.clone();
    raw(&mut conn, first_layout).await;
    raw(
        &mut conn,
        format!(
            "CREATE TABLE {} (name TEXT PRIMARY KEY NOT NULL, applied_time TIMESTAMP NOT NULL \
             DEFAULT CURRENT_TIMESTAMP)",
            migrator.meta_table_name()
        ),
    )
    .await;
    raw(
        &mut conn,
        format!(
            "INSERT INTO {}(name) VALUES ('{}')",
            migrator.meta_table_name(),
            migrator.dialect().sql().internal.steps()[0].0
        ),
    )
    .await;
    raw(
        &mut conn,
        format!(
            "INSERT INTO {}(app, name) VALUES ('old', 'row')",
            migrator.table_name()
        ),
    )
    .await;

    let rows = migrator
        .fetch_applied_migration_from_db(&mut conn)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].success());
    assert_eq!(rows[0].batch(), rows[0].id());
}

/// An internal migration recorded by a newer release is rejected rather than
/// ignored.
#[tokio::test]
async fn unknown_internal_migration_is_rejected() {
    let migrator = Migrator::<Sqlite>::default();
    let mut conn = make_conn().await;
    migrator
        .ensure_migration_table_exists(&mut conn)
        .await
        .unwrap();
    raw(
        &mut conn,
        format!(
            "INSERT INTO {}(name) VALUES ('9999_from_the_future')",
            migrator.meta_table_name()
        ),
    )
    .await;
    for result in [
        migrator.ensure_migration_table_exists(&mut conn).await,
        migrator
            .run(&mut conn, &Plan::apply_all())
            .await
            .map(|_| ()),
    ] {
        assert!(
            matches!(
                &result,
                Err(Error::UnknownInternalMigration { name }) if name == "9999_from_the_future"
            ),
            "{result:?}"
        );
    }
    let error = migrator
        .ensure_migration_table_exists(&mut conn)
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "migration table was changed by internal migration 9999_from_the_future which this \
         version of sqlx_migrator does not know, upgrade sqlx_migrator"
    );
}

/// Dropping the migration table drops the meta table as well.
#[tokio::test]
async fn drop_removes_meta_table() {
    let migrator = Migrator::<Sqlite>::default();
    let mut conn = make_conn().await;
    migrator
        .ensure_migration_table_exists(&mut conn)
        .await
        .unwrap();
    migrator
        .drop_migration_table_if_exists(&mut conn)
        .await
        .unwrap();
    assert!(!sqlite_table_exists(&mut conn, &migrator.table_name()).await);
    assert!(!sqlite_table_exists(&mut conn, &migrator.meta_table_name()).await);
}

/// A non atomic migration which fails part way stays recorded as failed, no
/// run touches the database until it is repaired, and repairing only removes
/// the failed entry.
#[tokio::test]
async fn failed_non_atomic_migration_blocks_runs_until_repaired() {
    let broken = || {
        vec![
            SqlMigration::new("first", None, "CREATE TABLE first (id INTEGER)"),
            SqlMigration::new("second", Some("first"), "CREATE TABLE second (id INTEGER)")
                .non_atomic()
                .operation("THIS IS NOT SQL"),
        ]
    };
    let migrator = migrator_of(broken());
    let mut conn = make_conn().await;
    assert!(matches!(
        migrator.run(&mut conn, &Plan::apply_all()).await,
        Err(Error::Sqlx(_))
    ));
    let rows = migrator
        .fetch_applied_migration_from_db(&mut conn)
        .await
        .unwrap();
    let success = |name: &str| {
        rows.iter()
            .find(|row| row.name() == name)
            .map(AppliedMigrationSqlRow::success)
    };
    assert_eq!(success("first"), Some(true));
    assert_eq!(success("second"), Some(false));

    for plan in [
        Plan::apply_all(),
        Plan::revert_all(),
        Plan::apply_all().fake(true),
    ] {
        assert!(matches!(
            migrator.run(&mut conn, &plan).await,
            Err(Error::FailedMigration { name, .. }) if name == "second"
        ));
    }

    assert_eq!(migrator.repair(&mut conn).await.unwrap(), 1);
    assert_eq!(migrator.repair(&mut conn).await.unwrap(), 0);
    let rows = migrator
        .fetch_applied_migration_from_db(&mut conn)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name(), "first");

    // the partly applied table has to be fixed by hand before running again
    raw(&mut conn, "DROP TABLE second".to_string()).await;
    let fixed = migrator_of(vec![
        SqlMigration::new("first", None, "CREATE TABLE first (id INTEGER)"),
        SqlMigration::new("second", Some("first"), "CREATE TABLE second (id INTEGER)").non_atomic(),
    ]);
    let report = fixed.run(&mut conn, &Plan::apply_all()).await.unwrap();
    assert_eq!(report.len(), 1);
    let rows = fixed
        .fetch_applied_migration_from_db(&mut conn)
        .await
        .unwrap();
    assert!(rows.iter().all(AppliedMigrationSqlRow::success));
}

/// Rows of one run share a batch and a fake run records no execution time.
#[tokio::test]
async fn applied_rows_record_time_and_batch() {
    let migrator = migrator_of(vec![
        SqlMigration::new("first", None, "CREATE TABLE first (id INTEGER)"),
        SqlMigration::new("second", Some("first"), "CREATE TABLE second (id INTEGER)").non_atomic(),
        SqlMigration::new("third", Some("second"), "CREATE TABLE third (id INTEGER)"),
        SqlMigration::new("fourth", Some("third"), "CREATE TABLE fourth (id INTEGER)"),
    ]);
    let mut conn = make_conn().await;
    migrator
        .run(&mut conn, &Plan::apply_count(1))
        .await
        .unwrap();
    migrator
        .run(&mut conn, &Plan::apply_count(2))
        .await
        .unwrap();
    migrator
        .run(&mut conn, &Plan::apply_all().fake(true))
        .await
        .unwrap();
    let rows = migrator
        .fetch_applied_migration_from_db(&mut conn)
        .await
        .unwrap();
    let batches = rows
        .iter()
        .map(AppliedMigrationSqlRow::batch)
        .collect::<Vec<_>>();
    assert_eq!(batches, [1, 2, 2, 3]);
    // a fake run executes nothing
    assert_eq!(rows[3].execution_time_ns(), 0);
}

/// Reverting the last batch reverts the migrations of the last run only and
/// has nothing to do once nothing is applied.
#[tokio::test]
async fn revert_last_batch_reverts_one_run() {
    let migrator = migrator_of(vec![
        SqlMigration::new("first", None, "CREATE TABLE first (id INTEGER)"),
        SqlMigration::new("second", Some("first"), "CREATE TABLE second (id INTEGER)"),
        SqlMigration::new("third", Some("second"), "CREATE TABLE third (id INTEGER)"),
    ]);
    let mut conn = make_conn().await;
    migrator
        .run(&mut conn, &Plan::apply_count(1))
        .await
        .unwrap();
    migrator.run(&mut conn, &Plan::apply_all()).await.unwrap();
    let applied = |rows: Vec<AppliedMigrationSqlRow>| {
        rows.iter()
            .map(|row| row.name().to_string())
            .collect::<Vec<_>>()
    };

    let report = migrator
        .run(&mut conn, &Plan::revert_last_batch())
        .await
        .unwrap();
    assert_eq!(report.len(), 2);
    let rows = migrator
        .fetch_applied_migration_from_db(&mut conn)
        .await
        .unwrap();
    assert_eq!(applied(rows), ["first"]);

    let report = migrator
        .run(&mut conn, &Plan::revert_last_batch())
        .await
        .unwrap();
    assert_eq!(report.len(), 1);
    let report = migrator
        .run(&mut conn, &Plan::revert_last_batch())
        .await
        .unwrap();
    assert!(report.is_empty());
}

/// Applied time is decoded as a time value.
#[tokio::test]
async fn applied_time_is_decoded_as_time() {
    let migrator = migrator_of(vec![SqlMigration::new(
        "first",
        None,
        "CREATE TABLE first (id INTEGER)",
    )]);
    let mut conn = make_conn().await;
    migrator.run(&mut conn, &Plan::apply_all()).await.unwrap();
    let rows = migrator
        .fetch_applied_migration_from_db(&mut conn)
        .await
        .unwrap();
    let age = OffsetDateTime::now_utc() - rows[0].applied_time();
    assert!(age.whole_seconds().abs() < 60);
}
