//! Integration tests for the `RunReport` returned by `Migrator::run`.
#![cfg(all(test, feature = "sqlite"))]

#[cfg(test)]
mod tests {
    use sqlx::{Connection as _, Sqlite, SqliteConnection};
    use sqlx_migrator::error::Error;
    use sqlx_migrator::migrator::{Migrator, Plan, RunRecord, RunReport};
    use sqlx_migrator::{sqlite_migration, vec_box};

    struct First;
    sqlite_migration!(
        First,
        "main",
        "first",
        vec_box![],
        vec_box![("CREATE TABLE first (id INTEGER)", "DROP TABLE first")]
    );

    struct Second;
    sqlite_migration!(
        Second,
        "main",
        "second",
        vec_box![First],
        vec_box![("CREATE TABLE second (id INTEGER)", "DROP TABLE second")]
    );

    struct Failing;
    sqlite_migration!(
        Failing,
        "main",
        "failing",
        vec_box![Second],
        vec_box![("THIS IS NOT SQL", "DROP TABLE nothing")]
    );

    async fn connection() -> SqliteConnection {
        SqliteConnection::connect("sqlite::memory:").await.unwrap()
    }

    fn migrator() -> Migrator<Sqlite> {
        let mut migrator = Migrator::<Sqlite>::default();
        migrator.add_migrations(vec_box![First, Second]).unwrap();
        migrator
    }

    fn names(report: &RunReport) -> Vec<String> {
        report
            .records()
            .iter()
            .map(|record| format!("{}:{}", record.app(), record.name()))
            .collect()
    }

    #[tokio::test]
    async fn apply_report_lists_migrations_in_order() {
        let mut connection = connection().await;
        let report = migrator()
            .run(&mut connection, &Plan::apply_all())
            .await
            .unwrap();
        assert!(report.is_apply());
        assert!(!report.is_fake());
        assert_eq!(report.len(), 2);
        assert!(!report.is_empty());
        assert_eq!(names(&report), vec!["main:first", "main:second"]);
        let summed: std::time::Duration = report.records().iter().map(RunRecord::elapsed).sum();
        assert_eq!(report.total_elapsed(), summed);
    }

    #[tokio::test]
    async fn revert_report_lists_migrations_in_reverse_order() {
        let mut connection = connection().await;
        let migrator = migrator();
        migrator
            .run(&mut connection, &Plan::apply_all())
            .await
            .unwrap();
        let report = migrator
            .run(&mut connection, &Plan::revert_all())
            .await
            .unwrap();
        assert!(!report.is_apply());
        assert_eq!(names(&report), vec!["main:second", "main:first"]);
    }

    #[tokio::test]
    async fn fake_and_empty_reports() {
        let mut connection = connection().await;
        let migrator = migrator();
        let report = migrator
            .run(&mut connection, &Plan::apply_count(1).fake(true))
            .await
            .unwrap();
        assert!(report.is_fake());
        assert_eq!(names(&report), vec!["main:first"]);

        // nothing left for a count of one after faking? second is still pending
        let report = migrator
            .run(&mut connection, &Plan::apply_all())
            .await
            .unwrap();
        assert_eq!(names(&report), vec!["main:second"]);

        let report = migrator
            .run(&mut connection, &Plan::apply_all())
            .await
            .unwrap();
        assert!(report.is_empty());
        assert_eq!(report.len(), 0);
        assert_eq!(report.total_elapsed(), std::time::Duration::ZERO);
    }

    #[tokio::test]
    async fn failed_run_returns_error_and_keeps_earlier_migrations() {
        let mut connection = connection().await;
        let mut migrator = Migrator::<Sqlite>::default();
        migrator.add_migration(Box::new(Failing)).unwrap();
        let result = migrator.run(&mut connection, &Plan::apply_all()).await;
        assert!(matches!(result, Err(Error::Sqlx(_))));
        // first and second were applied before failing ran
        let applied = migrator
            .generate_migration_plan(Some(&Plan::revert_all()), &[])
            .map(|plan| plan.len());
        assert_eq!(applied.unwrap(), 0);
        let report = migrator
            .run(&mut connection, &Plan::revert_all())
            .await
            .unwrap();
        assert_eq!(names(&report), vec!["main:second", "main:first"]);
    }
}
