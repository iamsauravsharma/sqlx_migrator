//! Integration tests for `LockStrategy` and `supports_transactional_ddl`.
#![cfg(all(test, feature = "sqlite"))]

#[cfg(test)]
mod tests {
    use sqlx::{Connection as _, Sqlite, SqliteConnection};
    use sqlx_migrator::migrator::{
        DatabaseOperation as _, Info as _, LockStrategy, Migrate as _, Migrator, Plan,
    };
    use sqlx_migrator::{sqlite_migration, vec_box};

    struct First;
    sqlite_migration!(
        First,
        "main",
        "first",
        vec_box![],
        vec_box![("CREATE TABLE first (id INTEGER)", "DROP TABLE first")]
    );

    async fn connection() -> SqliteConnection {
        SqliteConnection::connect("sqlite::memory:").await.unwrap()
    }

    #[test]
    fn default_strategy_is_database() {
        let migrator = Migrator::<Sqlite>::new();
        assert_eq!(migrator.lock_strategy(), LockStrategy::Database);
        assert_eq!(LockStrategy::default(), LockStrategy::Database);
    }

    #[test]
    fn strategy_can_be_changed() {
        let migrator = Migrator::<Sqlite>::new().set_lock_strategy(LockStrategy::None);
        assert_eq!(migrator.lock_strategy(), LockStrategy::None);
        let migrator = migrator.set_lock_strategy(LockStrategy::Database);
        assert_eq!(migrator.lock_strategy(), LockStrategy::Database);
    }

    #[test]
    fn strategy_combines_with_other_options() {
        let migrator = Migrator::<Sqlite>::new()
            .set_table_prefix("pfx")
            .unwrap()
            .set_lock_strategy(LockStrategy::None)
            .set_schema("sch")
            .unwrap();
        assert_eq!(migrator.lock_strategy(), LockStrategy::None);
        assert_eq!(migrator.table_name(), "sch._pfx_sqlx_migrator_migrations");
    }

    #[tokio::test]
    async fn run_works_with_every_strategy() {
        for strategy in [LockStrategy::Database, LockStrategy::None] {
            let mut connection = connection().await;
            let mut migrator = Migrator::<Sqlite>::new().set_lock_strategy(strategy);
            migrator.add_migration(Box::new(First)).unwrap();
            // explicit lock and unlock round trip
            migrator.lock(&mut connection).await.unwrap();
            migrator.unlock(&mut connection).await.unwrap();
            let report = migrator
                .run(&mut connection, &Plan::apply_all())
                .await
                .unwrap();
            assert_eq!(report.len(), 1, "{strategy:?}");
            let report = migrator
                .run(&mut connection, &Plan::revert_all())
                .await
                .unwrap();
            assert_eq!(report.len(), 1, "{strategy:?}");
        }
    }

    #[tokio::test]
    async fn sqlite_supports_transactional_ddl() {
        let connection = connection().await;
        let migrator = Migrator::<Sqlite>::new();
        assert!(migrator.supports_transactional_ddl(&connection));
    }
}
