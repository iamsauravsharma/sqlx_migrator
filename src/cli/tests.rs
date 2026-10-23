//! Tests for the cli module
#![expect(
    clippy::unreachable,
    reason = "tests destructure a subcommand they just parsed"
)]
use super::list::{ListStatus, list_entries, milliseconds, render_list};
use super::output::{OutputFormat, render_plan, render_table};
use super::*;
use crate::migration::Migration;
use crate::migrator::Plan;

fn parse(args: &[&str]) -> Result<MigrationCommand, clap::Error> {
    MigrationCommand::try_parse_from(std::iter::once("migrator").chain(args.iter().copied()))
}

#[test]
fn every_subcommand_parses() {
    assert!(matches!(
        parse(&["apply"]).unwrap().sub_command,
        SubCommand::Apply(_)
    ));
    assert!(matches!(
        parse(&["drop"]).unwrap().sub_command,
        SubCommand::Drop
    ));
    assert!(matches!(
        parse(&["heads"]).unwrap().sub_command,
        SubCommand::Heads(_)
    ));
    assert!(matches!(
        parse(&["list"]).unwrap().sub_command,
        SubCommand::List(_)
    ));
    assert!(matches!(
        parse(&["prune"]).unwrap().sub_command,
        SubCommand::Prune(Prune { force: false })
    ));
    assert!(matches!(
        parse(&["prune", "--force"]).unwrap().sub_command,
        SubCommand::Prune(Prune { force: true })
    ));
    assert!(matches!(
        parse(&["repair"]).unwrap().sub_command,
        SubCommand::Repair(Repair { force: false })
    ));
    assert!(matches!(
        parse(&["repair", "--force"]).unwrap().sub_command,
        SubCommand::Repair(Repair { force: true })
    ));
    assert!(matches!(
        parse(&["redo", "--force"]).unwrap().sub_command,
        SubCommand::Redo(Redo { force: true })
    ));
    assert!(matches!(
        parse(&["revert"]).unwrap().sub_command,
        SubCommand::Revert(_)
    ));
}

#[test]
fn apply_argument_rules() {
    #[cfg(feature = "json")]
    {
        let SubCommand::Apply(apply) = parse(&["apply", "--plan", "--format", "json"])
            .unwrap()
            .sub_command
        else {
            unreachable!()
        };
        assert!(apply.plan);
        assert_eq!(apply.format, OutputFormat::Json);
    }

    let SubCommand::Apply(apply) = parse(&["apply"]).unwrap().sub_command else {
        unreachable!()
    };
    assert_eq!(apply.format, OutputFormat::Table);

    // format requires plan (and json is rejected entirely without the feature)
    assert!(parse(&["apply", "--format", "json"]).is_err());
    // check conflicts with count, fake, force and plan
    for flag in [
        ["--count", "1"],
        ["--fake", ""],
        ["--force", ""],
        ["--plan", ""],
    ] {
        let mut args = vec!["apply", "--check", flag[0]];
        if !flag[1].is_empty() {
            args.push(flag[1]);
        }
        assert!(parse(&args).is_err(), "{args:?}");
    }
    // count conflicts with app, migration requires app
    assert!(parse(&["apply", "--count", "1", "--app", "main"]).is_err());
    assert!(parse(&["apply", "--migration", "first"]).is_err());
    assert!(parse(&["apply", "--app", "main", "--migration", "first"]).is_ok());
    // plan conflicts with fake and force
    assert!(parse(&["apply", "--plan", "--fake"]).is_err());
    assert!(parse(&["apply", "--plan", "--force"]).is_err());
    // prune is no longer an apply flag
    assert!(parse(&["apply", "--prune"]).is_err());
}

#[test]
fn revert_argument_rules() {
    assert!(parse(&["revert", "--all", "--app", "main"]).is_err());
    assert!(parse(&["revert", "--count", "1", "--all"]).is_err());
    assert!(parse(&["revert", "--count", "1", "--app", "main"]).is_err());
    assert!(parse(&["revert", "--migration", "first"]).is_err());
    assert!(parse(&["revert", "--plan", "--fake"]).is_err());
    assert!(parse(&["revert", "--format", "json"]).is_err());
    #[cfg(feature = "json")]
    assert!(parse(&["revert", "--plan", "--format", "json"]).is_ok());
    assert!(parse(&["revert", "--all", "--force"]).is_ok());
    assert!(parse(&["revert", "--last-batch", "--force"]).is_ok());
    assert!(parse(&["revert", "--last-batch", "--all"]).is_err());
    assert!(parse(&["revert", "--last-batch", "--app", "main"]).is_err());
    assert!(parse(&["revert", "--last-batch", "--count", "1"]).is_err());
}

#[test]
fn list_format_argument() {
    let SubCommand::List(list) = parse(&["list"]).unwrap().sub_command else {
        unreachable!()
    };
    assert_eq!(list.format, OutputFormat::Table);
    #[cfg(feature = "json")]
    {
        let SubCommand::List(list) = parse(&["list", "--format", "json"]).unwrap().sub_command
        else {
            unreachable!()
        };
        assert_eq!(list.format, OutputFormat::Json);
    }
    assert!(parse(&["list", "--format", "yaml"]).is_err());
}

#[test]
fn time_is_shown_in_milliseconds_with_a_fraction() {
    assert_eq!(milliseconds(0), "0.000");
    assert_eq!(milliseconds(999_999), "0.999");
    assert_eq!(milliseconds(1_234_567), "1.234");
    assert_eq!(milliseconds(2_000_000_000), "2000.000");
}

#[test]
fn table_widths_fit_content() {
    let rows = vec![
        vec!["1".to_string(), "a_rather_long_migration_name".to_string()],
        vec!["22".to_string(), "b".to_string()],
    ];
    let table = render_table(&["ID", "Name"], &rows);
    let lines = table.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 4);
    // id column is as wide as "ID" and "22", name column as wide as the
    // long name; separator is " | "
    let expected_width = 2 + 3 + "a_rather_long_migration_name".len();
    assert_eq!(lines[1], "-".repeat(expected_width));
    assert!(
        lines
            .iter()
            .all(|line| line.chars().count() == expected_width)
    );
    assert!(lines[0].starts_with("ID | "));
    assert_eq!(
        lines[0].trim_end().trim_start_matches("ID | ").trim(),
        "Name"
    );
    assert!(lines[2].starts_with("1  | a_rather_long_migration_name"));
    assert!(lines[3].starts_with("22 |"));
}

#[test]
fn table_width_grows_with_header_when_cells_are_short() {
    let table = render_table(&["Applied time"], &[vec!["x".to_string()]]);
    let lines = table.lines().collect::<Vec<_>>();
    assert_eq!(lines[1].len(), "Applied time".len());
    assert_eq!(lines[2].trim(), "x");
}

#[cfg(feature = "sqlite")]
mod with_sqlite {
    use sqlx::{Connection as _, Sqlite, SqliteConnection};

    use super::*;
    use crate::{sqlite_migration, vec_box};

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

    struct Replacer;
    impl Migration<Sqlite> for Replacer {
        fn app(&self) -> &'static str {
            "main"
        }

        fn name(&self) -> &'static str {
            "replacer"
        }

        fn parents(&self) -> Vec<Box<dyn Migration<Sqlite>>> {
            vec_box![]
        }

        fn operations(&self) -> Vec<Box<dyn crate::operation::Operation<Sqlite>>> {
            vec_box![("CREATE TABLE replacer (id INTEGER)", "DROP TABLE replacer")]
        }

        fn replaces(&self) -> Vec<Box<dyn Migration<Sqlite>>> {
            vec_box![Second]
        }
    }

    async fn connection() -> SqliteConnection {
        SqliteConnection::connect("sqlite::memory:").await.unwrap()
    }

    fn migrator() -> Migrator<Sqlite> {
        let mut migrator = Migrator::<Sqlite>::default();
        migrator.add_migrations(vec_box![First, Second]).unwrap();
        migrator
    }

    async fn run(
        args: &[&str],
        connection: &mut SqliteConnection,
        migrator: &Migrator<Sqlite>,
    ) -> Result<(), Error> {
        parse(args).unwrap().run(connection, migrator).await
    }

    async fn table_exists(connection: &mut SqliteConnection, table: &str) -> bool {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT 1 FROM {table} WHERE 1 = 0"
        )))
        .execute(connection)
        .await
        .is_ok()
    }

    async fn applied(
        connection: &mut SqliteConnection,
        migrator: &Migrator<Sqlite>,
    ) -> Vec<String> {
        migrator
            .fetch_applied_migration_from_db(connection)
            .await
            .unwrap()
            .iter()
            .map(|row| row.name().to_string())
            .collect()
    }

    #[tokio::test]
    async fn apply_check_revert_and_drop_through_cli() {
        let mut connection = connection().await;
        let migrator = migrator();
        assert!(matches!(
            run(&["apply", "--check"], &mut connection, &migrator).await,
            Err(Error::PendingMigrationPresent)
        ));
        run(&["apply", "--plan"], &mut connection, &migrator)
            .await
            .unwrap();
        run(&["apply"], &mut connection, &migrator).await.unwrap();
        assert_eq!(
            applied(&mut connection, &migrator).await,
            vec!["first", "second"]
        );
        run(&["apply", "--check"], &mut connection, &migrator)
            .await
            .unwrap();
        run(&["list"], &mut connection, &migrator).await.unwrap();
        #[cfg(feature = "json")]
        run(&["list", "--format", "json"], &mut connection, &migrator)
            .await
            .unwrap();

        // confirmation cannot be read in tests, so revert needs --force
        assert!(matches!(
            run(&["revert"], &mut connection, &migrator).await,
            Err(Error::ConfirmationRequired)
        ));
        assert!(matches!(
            run(&["drop"], &mut connection, &migrator).await,
            Err(Error::AppliedMigrationExists)
        ));
        run(&["revert", "--force"], &mut connection, &migrator)
            .await
            .unwrap();
        assert_eq!(applied(&mut connection, &migrator).await, vec!["first"]);
        run(&["revert", "--all", "--force"], &mut connection, &migrator)
            .await
            .unwrap();
        assert_eq!(
            applied(&mut connection, &migrator).await,
            Vec::<String>::new()
        );
        run(&["drop"], &mut connection, &migrator).await.unwrap();
    }

    #[tokio::test]
    async fn redo_reverts_and_reapplies_last_migration() {
        let mut connection = connection().await;
        let migrator = migrator();
        run(&["redo", "--force"], &mut connection, &migrator)
            .await
            .unwrap();
        assert_eq!(
            applied(&mut connection, &migrator).await,
            Vec::<String>::new()
        );

        run(&["apply"], &mut connection, &migrator).await.unwrap();
        let before = migrator
            .fetch_applied_migration_from_db(&mut connection)
            .await
            .unwrap();
        assert!(matches!(
            run(&["redo"], &mut connection, &migrator).await,
            Err(Error::ConfirmationRequired)
        ));
        run(&["redo", "--force"], &mut connection, &migrator)
            .await
            .unwrap();
        let after = migrator
            .fetch_applied_migration_from_db(&mut connection)
            .await
            .unwrap();
        assert_eq!(
            applied(&mut connection, &migrator).await,
            vec!["first", "second"]
        );
        // first keeps its row, second was deleted and inserted again
        assert_eq!(before[0].id(), after[0].id());
        assert!(after[1].id() > before[1].id());
    }

    #[tokio::test]
    async fn prune_removes_only_unknown_rows() {
        let mut connection = connection().await;
        let migrator = migrator();
        run(&["apply"], &mut connection, &migrator).await.unwrap();
        // nothing unknown yet, no confirmation needed
        run(&["prune"], &mut connection, &migrator).await.unwrap();

        migrator
            .add_migration_to_db_table(
                &mut connection,
                &("ghost", "removed"),
                std::time::Duration::ZERO,
                1,
            )
            .await
            .unwrap();
        let rows = migrator
            .fetch_applied_migration_from_db(&mut connection)
            .await
            .unwrap();
        let entries = list_entries(&migrator, &rows).unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[2].status, ListStatus::Unknown);
        assert_eq!(entries[2].name, "removed");

        assert!(matches!(
            run(&["prune"], &mut connection, &migrator).await,
            Err(Error::ConfirmationRequired)
        ));
        run(&["prune", "--force"], &mut connection, &migrator)
            .await
            .unwrap();
        assert_eq!(
            applied(&mut connection, &migrator).await,
            vec!["first", "second"]
        );
    }

    #[tokio::test]
    async fn drop_removes_the_lock_table_of_the_row_lock() {
        let mut connection = connection().await;
        let dialect = crate::Dialect::sqlite()
            .set_lock_strategy(crate::migrator::LockStrategy::Row)
            .unwrap();
        let lock_table = dialect.lock_table_name();
        let mut migrator = Migrator::<Sqlite>::new(dialect);
        migrator.add_migrations(vec_box![First, Second]).unwrap();
        run(&["apply"], &mut connection, &migrator).await.unwrap();
        assert!(table_exists(&mut connection, &lock_table).await);
        run(&["revert", "--all", "--force"], &mut connection, &migrator)
            .await
            .unwrap();
        run(&["drop"], &mut connection, &migrator).await.unwrap();
        assert!(!table_exists(&mut connection, &lock_table).await);
    }

    #[tokio::test]
    async fn repair_removes_failed_rows() {
        let mut connection = connection().await;
        let migrator = migrator();
        // nothing failed yet, no confirmation needed
        run(&["repair"], &mut connection, &migrator).await.unwrap();

        migrator
            .start_migration_in_db_table(&mut connection, &First, 1)
            .await
            .unwrap();
        let rows = migrator
            .fetch_applied_migration_from_db(&mut connection)
            .await
            .unwrap();
        let entries = list_entries(&migrator, &rows).unwrap();
        assert_eq!(entries[0].status, ListStatus::Failed);
        assert!(
            render_list(&entries, OutputFormat::Table)
                .unwrap()
                .contains('!')
        );
        assert!(matches!(
            run(&["apply"], &mut connection, &migrator).await,
            Err(Error::FailedMigration { name, .. }) if name == "first"
        ));

        assert!(matches!(
            run(&["repair"], &mut connection, &migrator).await,
            Err(Error::ConfirmationRequired)
        ));
        run(&["repair", "--force"], &mut connection, &migrator)
            .await
            .unwrap();
        run(&["apply"], &mut connection, &migrator).await.unwrap();
        assert_eq!(
            applied(&mut connection, &migrator).await,
            vec!["first", "second"]
        );
    }

    #[tokio::test]
    async fn revert_last_batch_through_cli() {
        let mut connection = connection().await;
        let migrator = migrator();
        // nothing applied, nothing to revert
        run(
            &["revert", "--last-batch", "--force"],
            &mut connection,
            &migrator,
        )
        .await
        .unwrap();
        run(&["apply", "--count", "1"], &mut connection, &migrator)
            .await
            .unwrap();
        run(&["apply"], &mut connection, &migrator).await.unwrap();
        run(
            &["revert", "--last-batch", "--force"],
            &mut connection,
            &migrator,
        )
        .await
        .unwrap();
        assert_eq!(applied(&mut connection, &migrator).await, vec!["first"]);
    }

    #[tokio::test]
    async fn list_entries_and_rendering() {
        let mut connection = connection().await;
        let mut migrator = migrator();
        migrator.add_migration(Box::new(Replacer)).unwrap();
        run(
            &["apply", "--app", "main", "--migration", "first"],
            &mut connection,
            &migrator,
        )
        .await
        .unwrap();
        let rows = migrator
            .fetch_applied_migration_from_db(&mut connection)
            .await
            .unwrap();
        let entries = list_entries(&migrator, &rows).unwrap();
        let status_of = |name: &str| {
            entries
                .iter()
                .find(|entry| entry.name == name)
                .map(|entry| entry.status)
                .unwrap()
        };
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].name, "first");
        assert_eq!(status_of("first"), ListStatus::Applied);
        assert_eq!(status_of("second"), ListStatus::Replaced);
        assert_eq!(status_of("replacer"), ListStatus::Pending);

        #[cfg(feature = "json")]
        {
            let json: serde_json::Value =
                serde_json::from_str(&render_list(&entries, OutputFormat::Json).unwrap()).unwrap();
            let items = json.as_array().unwrap();
            assert_eq!(items.len(), 3);
            let item = |name: &str| items.iter().find(|item| item["name"] == name).unwrap();
            assert_eq!(item("first")["status"], "applied");
            assert_eq!(item("first")["app"], "main");
            assert!(item("first")["id"].is_number());
            assert!(item("first")["applied_time"].is_string());
            assert_eq!(item("second")["status"], "replaced");
            assert!(item("second")["id"].is_null());
            assert!(item("second")["applied_time"].is_null());
            assert_eq!(item("replacer")["status"], "pending");
        }

        let table = render_list(&entries, OutputFormat::Table).unwrap();
        assert!(table.contains("\u{2713}"));
        assert!(table.contains("\u{2194}"));
        assert!(table.contains("\u{2717}"));
        assert!(table.lines().next().unwrap().contains("Applied time"));
    }

    #[tokio::test]
    async fn plan_rendering() {
        let mut connection = connection().await;
        let migrator = migrator();
        // fetching creates the table, so a fresh database has no rows
        let rows = migrator
            .fetch_applied_migration_from_db(&mut connection)
            .await
            .unwrap();
        assert_eq!(rows.len(), 0);
        let plan = migrator
            .generate_migration_plan(Some(&Plan::apply_all()), &[])
            .unwrap();

        let as_dyn = || plan.iter().map(AsRef::as_ref);
        #[cfg(feature = "json")]
        {
            let json: serde_json::Value =
                serde_json::from_str(&render_plan(as_dyn(), OutputFormat::Json, "empty").unwrap())
                    .unwrap();
            assert_eq!(json[0]["name"], "first");
            assert_eq!(json[1]["name"], "second");
        }

        let table = render_plan(as_dyn(), OutputFormat::Table, "empty").unwrap();
        let header = table.lines().next().unwrap();
        assert!(header.contains("App") && header.contains("| ") && header.contains("Name"));
        assert_eq!(table.lines().count(), 4);
        assert!(table.contains("second"));

        let empty = std::iter::empty::<&dyn Migration<Sqlite>>;
        assert_eq!(
            render_plan(empty(), OutputFormat::Table, "empty").unwrap(),
            "empty"
        );
        #[cfg(feature = "json")]
        {
            assert_eq!(
                render_plan(empty(), OutputFormat::Json, "empty").unwrap(),
                "[]"
            );
            run(
                &["apply", "--plan", "--format", "json"],
                &mut connection,
                &migrator,
            )
            .await
            .unwrap();
            run(
                &["revert", "--plan", "--format", "json"],
                &mut connection,
                &migrator,
            )
            .await
            .unwrap();
        }
        // reverting the last migration with nothing applied is a no-op,
        // an explicit count is still an error
        run(&["revert", "--force"], &mut connection, &migrator)
            .await
            .unwrap();
        assert!(matches!(
            run(
                &["revert", "--count", "1", "--force"],
                &mut connection,
                &migrator
            )
            .await,
            Err(Error::PlanError { .. })
        ));
    }
}
