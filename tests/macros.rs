//! Integration tests for the public macros and the string based operations.
//!
//! These live outside the crate so that `$crate` paths and macro hygiene are
//! exercised the same way a downstream crate would use them.
#![cfg(all(test, feature = "sqlite"))]

#[cfg(test)]
mod tests {
    use sqlx::{Connection as _, Sqlite, SqliteConnection};
    use sqlx_migrator::error::Error;
    use sqlx_migrator::migration::{AppliedMigrationSqlRow, Migration};
    use sqlx_migrator::migrator::{Migrator, Plan};
    use sqlx_migrator::operation::Operation;
    use sqlx_migrator::{include_sql, migration, sqlite_migration, vec_box};

    struct Positional;
    migration!(
        Sqlite,
        Positional,
        "main",
        "positional",
        vec_box![],
        vec_box![(
            "CREATE TABLE positional (id INTEGER)",
            "DROP TABLE positional"
        )],
    );

    // hand written implementation for the methods the macro does not cover
    struct Manual;
    impl Migration<Sqlite> for Manual {
        fn app(&self) -> &'static str {
            "main"
        }

        fn name(&self) -> &'static str {
            "manual"
        }

        fn parents(&self) -> Vec<Box<dyn Migration<Sqlite>>> {
            vec_box![Positional]
        }

        fn operations(&self) -> Vec<Box<dyn Operation<Sqlite>>> {
            vec_box![("CREATE TABLE manual (id INTEGER)", "DROP TABLE manual")]
        }

        fn replaces(&self) -> Vec<Box<dyn Migration<Sqlite>>> {
            vec_box![("main", "old_manual")]
        }

        fn run_before(&self) -> Vec<Box<dyn Migration<Sqlite>>> {
            vec_box![("other", "later")]
        }

        fn is_atomic(&self) -> bool {
            false
        }
    }

    // database specific macro without a trailing comma
    struct DbPositional;
    sqlite_migration!(
        DbPositional,
        "main",
        "db_positional",
        vec_box![],
        vec_box![]
    );

    struct SqlFiles;
    sqlite_migration!(
        SqlFiles,
        "files",
        "sql_files",
        vec_box![],
        vec_box![include_sql!("sql/0001")],
    );

    struct SqlFileIrreversible;
    sqlite_migration!(
        SqlFileIrreversible,
        "files",
        "sql_file_irreversible",
        vec_box![SqlFiles],
        vec_box![include_sql!(up: "sql/0002_seed.up.sql")]
    );

    struct StringOperations;
    sqlite_migration!(
        StringOperations,
        "strings",
        "string_operations",
        vec_box![],
        vec_box![
            ("CREATE TABLE strings (id INTEGER)", "DROP TABLE strings"),
            "INSERT INTO strings (id) VALUES (1)",
            String::from("INSERT INTO strings (id) VALUES (2)"),
        ],
    );

    // Concrete migrations standing in for the virtual ones referenced by
    // `Manual`
    struct OldManual;
    sqlite_migration!(OldManual, "main", "old_manual", vec_box![], vec_box![]);
    struct Later;
    sqlite_migration!(Later, "other", "later", vec_box![], vec_box![]);

    async fn connection() -> SqliteConnection {
        SqliteConnection::connect("sqlite::memory:").await.unwrap()
    }

    async fn count(connection: &mut SqliteConnection, table: &str) -> i64 {
        let (count,): (i64,) =
            sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT COUNT(*) FROM {table}")))
                .fetch_one(connection)
                .await
                .unwrap();
        count
    }

    #[test]
    fn positional_form_uses_defaults() {
        let migration: Box<dyn Migration<Sqlite>> = Box::new(Positional);
        assert_eq!(migration.app(), "main");
        assert_eq!(migration.name(), "positional");
        assert!(migration.parents().is_empty());
        assert_eq!(migration.operations().len(), 1);
        assert!(migration.replaces().is_empty());
        assert!(migration.run_before().is_empty());
        assert!(migration.is_atomic());
        assert!(!migration.is_virtual());
    }

    #[test]
    fn db_specific_macro_uses_defaults() {
        let migration: Box<dyn Migration<Sqlite>> = Box::new(DbPositional);
        assert_eq!(migration.app(), "main");
        assert_eq!(migration.name(), "db_positional");
        assert!(migration.parents().is_empty());
        assert!(migration.operations().is_empty());
        assert!(migration.replaces().is_empty());
        assert!(migration.run_before().is_empty());
        assert!(migration.is_atomic());
    }

    #[test]
    fn manual_implementation_sets_optional_methods() {
        let migration: Box<dyn Migration<Sqlite>> = Box::new(Manual);
        assert_eq!(migration.parents()[0].name(), "positional");
        assert_eq!(migration.replaces()[0].name(), "old_manual");
        assert_eq!(migration.run_before()[0].app(), "other");
        assert!(!migration.is_atomic());
    }

    #[test]
    fn include_sql_embeds_file_contents() {
        let expected_up = include_str!("sql/0001.up.sql");
        let expected_down = include_str!("sql/0001.down.sql");

        let (up, down) = include_sql!("sql/0001");
        assert_eq!((up, down), (expected_up, expected_down));

        let (up, down) = include_sql!("sql/0001.up.sql", "sql/0001.down.sql");
        assert_eq!((up, down), (expected_up, expected_down));

        let (up, down) = include_sql!(up: "sql/0001.up.sql", down: "sql/0001.down.sql");
        assert_eq!((up, down), (expected_up, expected_down));

        let single = include_sql!(up: "sql/0002_seed.up.sql");
        assert_eq!(single, include_str!("sql/0002_seed.up.sql"));
    }

    #[tokio::test]
    async fn string_operation_is_irreversible() {
        let mut connection = connection().await;
        let operation: Box<dyn Operation<Sqlite>> =
            Box::new("CREATE TABLE irreversible (id INTEGER)");
        operation.up(&mut connection).await.unwrap();
        assert_eq!(count(&mut connection, "irreversible").await, 0);
        assert!(matches!(
            operation.down(&mut connection).await,
            Err(Error::IrreversibleOperation)
        ));
        assert!(!operation.is_destructible());

        let owned: Box<dyn Operation<Sqlite>> =
            Box::new(String::from("INSERT INTO irreversible (id) VALUES (1)"));
        owned.up(&mut connection).await.unwrap();
        assert_eq!(count(&mut connection, "irreversible").await, 1);
        assert!(matches!(
            owned.down(&mut connection).await,
            Err(Error::IrreversibleOperation)
        ));
    }

    #[tokio::test]
    async fn mixed_string_operations_apply_and_fail_to_revert() {
        let mut connection = connection().await;
        let mut migrator = Migrator::<Sqlite>::default();
        migrator.add_migration(Box::new(StringOperations)).unwrap();

        migrator
            .run(&mut connection, &Plan::apply_all())
            .await
            .unwrap();
        assert_eq!(count(&mut connection, "strings").await, 2);

        // the irreversible string operations are reverted first and fail, the
        // atomic migration rolls back so the table and its rows remain
        assert!(matches!(
            migrator.run(&mut connection, &Plan::revert_all()).await,
            Err(Error::IrreversibleOperation)
        ));
        assert_eq!(count(&mut connection, "strings").await, 2);
    }

    #[tokio::test]
    async fn sql_file_migrations_apply_and_revert() {
        let mut connection = connection().await;
        let mut migrator = Migrator::<Sqlite>::default();
        migrator
            .add_migrations(vec_box![SqlFiles, SqlFileIrreversible])
            .unwrap();

        migrator
            .run(&mut connection, &Plan::apply_all())
            .await
            .unwrap();
        assert_eq!(count(&mut connection, "sample").await, 1);

        // reverting only the reversible migration works once the irreversible
        // one is faked out of the way
        migrator
            .run(
                &mut connection,
                &Plan::revert_name("files", Some("sql_file_irreversible")).fake(true),
            )
            .await
            .unwrap();
        migrator
            .run(&mut connection, &Plan::revert_all())
            .await
            .unwrap();
        let (exists,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'sample'",
        )
        .fetch_one(&mut connection)
        .await
        .unwrap();
        assert_eq!(exists, 0);
    }

    #[tokio::test]
    async fn manual_migration_with_replaces_and_run_before_applies() {
        let mut connection = connection().await;
        let mut migrator = Migrator::<Sqlite>::default();
        // Manual replaces a virtual migration and runs before another virtual
        // one; register the concrete stand-ins so the plan can be
        // generated
        migrator
            .add_migrations(vec_box![Manual, OldManual, Later])
            .unwrap();

        let applied = migrator
            .fetch_applied_migration_from_db(&mut connection)
            .await
            .unwrap();
        assert_eq!(applied.len(), 0, "nothing is applied before the first run");

        migrator
            .run(&mut connection, &Plan::apply_all())
            .await
            .unwrap();
        let applied = migrator
            .fetch_applied_migration_from_db(&mut connection)
            .await
            .unwrap();
        let names = applied
            .iter()
            .map(AppliedMigrationSqlRow::name)
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["positional", "manual", "later"]);
    }
}
