//! Integration test for the tracing spans and events emitted by
//! `Migrate::run`, including the transactional DDL warning. Kept in its own
//! binary as a single test because it installs a global subscriber.
#![cfg(all(test, feature = "sqlite"))]

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;
    use std::sync::{Arc, Mutex};

    use sqlx::{Connection as _, Database, Sqlite, SqliteConnection};
    use sqlx_migrator::error::Error;
    use sqlx_migrator::migration::{AppliedMigrationSqlRow, Migration};
    use sqlx_migrator::migrator::{DatabaseOperation, Info, Migrate, Migrator, Plan};
    use sqlx_migrator::{sqlite_migration, vec_box};
    use tracing::field::{Field, Visit};
    use tracing::span::{Attributes, Id, Record};
    use tracing::subscriber::{self, Subscriber};
    use tracing::{Event, Metadata};

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

    /// Migrator delegating to [`Migrator<Sqlite>`] but reporting that the
    /// database cannot roll back DDL, like MySQL.
    struct NoDdlMigrator(Migrator<Sqlite>);

    impl Info<Sqlite> for NoDdlMigrator {
        fn migrations(&self) -> &[Box<dyn Migration<Sqlite>>] {
            self.0.migrations()
        }

        fn migrations_mut(&mut self) -> &mut Vec<Box<dyn Migration<Sqlite>>> {
            self.0.migrations_mut()
        }
    }

    #[async_trait::async_trait]
    impl DatabaseOperation<Sqlite> for NoDdlMigrator {
        async fn ensure_migration_table_exists(
            &self,
            connection: &mut <Sqlite as Database>::Connection,
        ) -> Result<(), Error> {
            self.0.ensure_migration_table_exists(connection).await
        }

        async fn drop_migration_table_if_exists(
            &self,
            connection: &mut <Sqlite as Database>::Connection,
        ) -> Result<(), Error> {
            self.0.drop_migration_table_if_exists(connection).await
        }

        async fn add_migration_to_db_table(
            &self,
            connection: &mut <Sqlite as Database>::Connection,
            migration: &dyn Migration<Sqlite>,
        ) -> Result<(), Error> {
            self.0
                .add_migration_to_db_table(connection, migration)
                .await
        }

        async fn delete_migration_from_db_table(
            &self,
            connection: &mut <Sqlite as Database>::Connection,
            migration: &dyn Migration<Sqlite>,
        ) -> Result<(), Error> {
            self.0
                .delete_migration_from_db_table(connection, migration)
                .await
        }

        async fn fetch_applied_migration_from_db(
            &self,
            connection: &mut <Sqlite as Database>::Connection,
        ) -> Result<Vec<AppliedMigrationSqlRow>, Error> {
            self.0.fetch_applied_migration_from_db(connection).await
        }

        async fn lock(
            &self,
            connection: &mut <Sqlite as Database>::Connection,
        ) -> Result<(), Error> {
            self.0.lock(connection).await
        }

        async fn unlock(
            &self,
            connection: &mut <Sqlite as Database>::Connection,
        ) -> Result<(), Error> {
            self.0.unlock(connection).await
        }

        fn supports_transactional_ddl(
            &self,
            _connection: &<Sqlite as Database>::Connection,
        ) -> bool {
            false
        }
    }

    impl Migrate<Sqlite> for NoDdlMigrator {}

    async fn connection() -> SqliteConnection {
        SqliteConnection::connect("sqlite::memory:").await.unwrap()
    }

    fn migrator() -> Migrator<Sqlite> {
        let mut migrator = Migrator::<Sqlite>::new();
        migrator.add_migrations(vec_box![First, Second]).unwrap();
        migrator
    }

    /// Minimal subscriber collecting span names and event messages with their
    /// fields, enough to assert what the migrator emits.
    #[derive(Default)]
    struct Collector {
        spans: Mutex<Vec<String>>,
        events: Mutex<Vec<String>>,
    }

    struct FieldVisitor(String);

    impl Visit for FieldVisitor {
        #[expect(
            clippy::use_debug,
            reason = "tracing hands field values over as `dyn Debug` only"
        )]
        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            write!(self.0, " {}={value:?}", field.name()).unwrap();
        }
    }

    impl Subscriber for Collector {
        fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
            true
        }

        fn new_span(&self, attrs: &Attributes<'_>) -> Id {
            let mut visitor = FieldVisitor(format!("{}:", attrs.metadata().name()));
            attrs.record(&mut visitor);
            let mut spans = self.spans.lock().unwrap();
            spans.push(visitor.0);
            Id::from_u64(u64::try_from(spans.len()).unwrap())
        }

        fn record(&self, _span: &Id, _values: &Record<'_>) {}

        fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

        fn event(&self, event: &Event<'_>) {
            let mut visitor = FieldVisitor(format!("{}:", event.metadata().level()));
            event.record(&mut visitor);
            self.events.lock().unwrap().push(visitor.0);
        }

        fn enter(&self, _span: &Id) {}

        fn exit(&self, _span: &Id) {}
    }

    #[tokio::test]
    async fn run_emits_spans_and_info_events() {
        let collector = Arc::new(Collector::default());
        let mut connection = connection().await;
        let migrator = migrator();
        // a global subscriber so callsite interest is not cached as "never" by
        // another thread first; this binary therefore holds only this test
        subscriber::set_global_default(Arc::clone(&collector)).unwrap();
        migrator
            .run(&mut connection, &Plan::apply_all())
            .await
            .unwrap();

        let spans = collector.spans.lock().unwrap().clone();
        assert!(
            spans
                .iter()
                .any(|s| s.starts_with("sqlx_migrator_run:") && s.contains("plan_type=Apply")),
            "{spans:?}"
        );
        assert!(
            spans.iter().any(|s| {
                s.starts_with("sqlx_migrator_migration:")
                    && s.contains("app=\"main\"")
                    && s.contains("name=\"first\"")
            }),
            "{spans:?}"
        );
        assert!(
            spans.iter().any(|s| s.contains("name=\"second\"")),
            "{spans:?}"
        );

        let events = collector.events.lock().unwrap().clone();
        let info = |needle: &str| {
            events
                .iter()
                .filter(|e| e.starts_with("INFO:") && e.contains(needle))
                .count()
        };
        assert_eq!(info("applying migration"), 2, "{events:?}");
        assert_eq!(info("migration finished"), 2, "{events:?}");
        assert!(
            events.iter().any(|e| e.contains("elapsed_ms=")),
            "{events:?}"
        );
        assert_eq!(info("run finished"), 1, "{events:?}");
        assert!(events.iter().any(|e| e.contains("count=2")), "{events:?}");
        assert!(
            !events.iter().any(|e| e.starts_with("WARN:")),
            "no warning expected on a database with transactional DDL: {events:?}"
        );

        // second phase: a backend without transactional DDL warns once per
        // atomic migration, and not for a fake run
        collector.events.lock().unwrap().clear();
        let mut no_ddl_connection = SqliteConnection::connect("sqlite::memory:").await.unwrap();
        let mut no_ddl = NoDdlMigrator(Migrator::<Sqlite>::new());
        no_ddl.add_migrations(vec_box![First, Second]).unwrap();
        no_ddl
            .run(&mut no_ddl_connection, &Plan::apply_all())
            .await
            .unwrap();
        let warnings = collector
            .events
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.starts_with("WARN:") && e.contains("transactional DDL"))
            .count();
        assert_eq!(warnings, 2);

        collector.events.lock().unwrap().clear();
        no_ddl
            .run(&mut no_ddl_connection, &Plan::revert_all().fake(true))
            .await
            .unwrap();
        let warnings = collector
            .events
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.starts_with("WARN:"))
            .count();
        assert_eq!(warnings, 0);
    }
}
