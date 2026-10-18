//! Integration test for the tracing spans and events emitted by
//! `Migrator::run`. Kept in its own binary as a single test because it installs
//! a global subscriber.
#![cfg(all(test, feature = "sqlite"))]

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;
    use std::sync::{Arc, Mutex};

    use sqlx::{Connection as _, Sqlite, SqliteConnection};
    use sqlx_migrator::migrator::Plan;
    use sqlx_migrator::{Migrator, sqlite_migration, vec_box};
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

    async fn connection() -> SqliteConnection {
        SqliteConnection::connect("sqlite::memory:").await.unwrap()
    }

    fn migrator() -> Migrator<Sqlite> {
        let mut migrator = Migrator::<Sqlite>::default();
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
            events.iter().any(|e| e.contains("elapsed_ns=")),
            "{events:?}"
        );
        assert_eq!(info("run finished"), 1, "{events:?}");
        assert!(events.iter().any(|e| e.contains("count=2")), "{events:?}");
        assert!(
            !events.iter().any(|e| e.starts_with("WARN:")),
            "no warning expected: {events:?}"
        );
    }
}
