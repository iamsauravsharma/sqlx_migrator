//! Scenarios: locks and concurrent runs

use super::*;

/// While the dialect's lock is held by one connection a run on a second
/// connection waits on backends which lock, and never waits on the others.
pub(crate) async fn lock<H>(harness: &H, test: &str)
where
    H: Harness,
    (String, String): Operation<H::Db>,
{
    let mut connection = harness.setup(test).await;
    let migrator = Arc::new(harness.migrator(test));
    let mut second = harness.connect(test).await;
    let Some((acquire, release)) = harness.lock_statements() else {
        return;
    };
    let table_name = migrator.table_name();
    H::Db::execute_bound(&mut connection, acquire, &[&table_name])
        .await
        .unwrap();
    let waiting = {
        let migrator = Arc::clone(&migrator);
        tokio::spawn(async move { migrator.run(&mut second, &Plan::apply_all()).await })
    };
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert!(!waiting.is_finished(), "run should wait for the lock");
    H::Db::execute_bound(&mut connection, release, &[&table_name])
        .await
        .unwrap();
    let report = tokio::time::timeout(Duration::from_secs(10), waiting)
        .await
        .expect("run should proceed once the lock is released")
        .unwrap()
        .unwrap();
    assert_eq!(report.len(), 2);
    harness.teardown(test);
}

/// Two runs racing on separate connections apply every migration exactly
/// once. Backends which lock serialise the runs so both succeed; without a
/// lock one run may fail on what the other already created, but atomic
/// migrations keep the database consistent.
pub(crate) async fn concurrent_runs<H>(harness: &H, test: &str)
where
    H: Harness,
    (String, String): Operation<H::Db>,
{
    let mut connection = harness.setup(test).await;
    let migrator = harness.migrator(test);
    let migrator = Arc::new(migrator);
    let mut second = harness.connect(test).await;
    // both runs must find the migration table, otherwise they race on
    // creating it; fetching creates it
    applied(&mut connection, &migrator).await;
    let a = {
        let migrator = Arc::clone(&migrator);
        tokio::spawn(async move { migrator.run(&mut connection, &Plan::apply_all()).await })
    };
    let b = {
        let migrator = Arc::clone(&migrator);
        tokio::spawn(async move { migrator.run(&mut second, &Plan::apply_all()).await })
    };
    let results = [a.await.unwrap(), b.await.unwrap()];
    if harness.lock_statements().is_some() {
        let lengths = results
            .iter()
            .map(|r| r.as_ref().unwrap().len())
            .collect::<Vec<_>>();
        assert_eq!(lengths.iter().sum::<usize>(), 2, "{lengths:?}");
    } else {
        assert!(results.iter().any(Result::is_ok));
    }

    let mut check = harness.connect(test).await;
    assert_eq!(
        applied(&mut check, &migrator).await,
        vec!["first", "second"]
    );
    assert!(table_exists::<H::Db>(&mut check, &format!("{test}_first")).await);
    assert!(table_exists::<H::Db>(&mut check, &format!("{test}_second")).await);
    harness.teardown(test);
}

/// A lock row left in the lock table makes a run fail until it is deleted, and
/// a run deletes its own row when it is done.
pub(crate) async fn row_lock<H>(harness: &H, test: &str)
where
    H: Harness,
    (String, String): Operation<H::Db>,
{
    let mut connection = harness.setup(test).await;
    let migrator = harness.row_lock_migrator(test);
    let lock_table = migrator.dialect().lock_table_name();
    let hold = format!(
        "INSERT INTO {lock_table}(name) VALUES ('{}')",
        migrator.table_name()
    );
    let release = format!("DELETE FROM {lock_table}");

    // a run creates the lock table and deletes its row, so the row can be
    // inserted by hand afterwards
    migrator
        .run(&mut connection, &Plan::apply_count(1))
        .await
        .unwrap();
    H::Db::execute(&mut connection, &hold).await.unwrap();
    assert!(matches!(
        migrator.run(&mut connection, &Plan::apply_all()).await,
        Err(Error::LockHeld { .. })
    ));
    H::Db::execute(&mut connection, &release).await.unwrap();
    let report = migrator
        .run(&mut connection, &Plan::apply_all())
        .await
        .unwrap();
    assert_eq!(report.len(), 1);
    harness.teardown(test);
}
