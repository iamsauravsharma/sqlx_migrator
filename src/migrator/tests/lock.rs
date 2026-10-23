use super::*;

// ── Lock strategies ──────────────────────────────────────────────────────────

/// Every database starts with its natural lock, the others can be chosen and a
/// lock the database does not have is refused.
#[test]
fn lock_strategy_defaults_and_choices() {
    assert_eq!(Dialect::sqlite().lock_strategy(), LockStrategy::None);
    assert_eq!(
        Dialect::sqlite()
            .set_lock_strategy(LockStrategy::Row)
            .unwrap()
            .lock_strategy(),
        LockStrategy::Row
    );
    for unsupported in [
        LockStrategy::Advisory,
        LockStrategy::TransactionAdvisory,
        LockStrategy::TableShareUpdateExclusive,
        LockStrategy::TableAccessExclusive,
    ] {
        assert!(matches!(
            Dialect::sqlite().set_lock_strategy(unsupported),
            Err(Error::UnsupportedLockStrategy { strategy }) if strategy == unsupported
        ));
    }
    #[cfg(feature = "postgres")]
    {
        assert_eq!(Dialect::postgres().lock_strategy(), LockStrategy::Advisory);
        for strategy in [
            LockStrategy::None,
            LockStrategy::Row,
            LockStrategy::TransactionAdvisory,
            LockStrategy::TableShareUpdateExclusive,
            LockStrategy::TableAccessExclusive,
        ] {
            assert_eq!(
                Dialect::postgres()
                    .set_lock_strategy(strategy)
                    .unwrap()
                    .lock_strategy(),
                strategy
            );
        }
    }
    #[cfg(feature = "mysql")]
    {
        assert_eq!(Dialect::mysql().lock_strategy(), LockStrategy::Advisory);
        assert!(
            Dialect::mysql()
                .set_lock_strategy(LockStrategy::TransactionAdvisory)
                .is_err()
        );
    }
}

/// The lock table has the name of the other tables.
#[test]
fn lock_table_follows_prefix_and_schema() {
    assert_eq!(
        Dialect::<Sqlite>::sqlite().lock_table_name(),
        "_sqlx_migrator_lock"
    );
    assert_eq!(
        Dialect::sqlite_with(Some("pfx"), Some("sch"))
            .unwrap()
            .lock_table_name(),
        "sch._pfx_sqlx_migrator_lock"
    );
}

/// The row of the lock is deleted when a run is done, a row which is still
/// there makes a run fail, and deleting it lets the next run proceed.
#[tokio::test]
async fn row_lock_fails_while_held() {
    let dialect = Dialect::sqlite()
        .set_lock_strategy(LockStrategy::Row)
        .unwrap();
    let lock_table = dialect.lock_table_name();
    let mut migrator = Migrator::<Sqlite>::new(dialect);
    migrator
        .add_migrations(vec![
            Box::new(SqlMigration::new(
                "first",
                None,
                "CREATE TABLE first (id INTEGER)",
            )) as Box<dyn Migration<Sqlite>>,
            Box::new(SqlMigration::new(
                "second",
                Some("first"),
                "CREATE TABLE second (id INTEGER)",
            )),
        ])
        .unwrap();
    let mut conn = make_conn().await;
    migrator
        .run(&mut conn, &Plan::apply_count(1))
        .await
        .unwrap();
    // released, so the row can be inserted again
    raw(
        &mut conn,
        format!(
            "INSERT INTO {lock_table}(name) VALUES ('{}')",
            migrator.table_name()
        ),
    )
    .await;

    let result = migrator.run(&mut conn, &Plan::apply_all()).await;
    assert!(
        matches!(&result, Err(Error::LockHeld { table }) if *table == lock_table),
        "{result:?}"
    );

    raw(&mut conn, format!("DELETE FROM {lock_table}")).await;
    let report = migrator.run(&mut conn, &Plan::apply_all()).await.unwrap();
    assert_eq!(report.len(), 1);
}
