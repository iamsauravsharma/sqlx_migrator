use super::*;

/// empty string).
#[test]
fn table_name() {
    for valid in ["abc", "ab1", "abc___123", "_123"] {
        assert!(Dialect::sqlite_with(Some(valid), None).is_ok(), "{valid}");
    }
    for invalid in ["aBc", "1@_2_3", ""] {
        assert!(
            Dialect::sqlite_with(Some(invalid), None).is_err(),
            "{invalid}"
        );
    }
}

/// Verifies that schema names must start with a lowercase letter or
/// underscore and hold only lowercase, digits and underscores.
#[test]
fn schema_name() {
    for valid in ["abc", "ab1", "abc___123", "_123"] {
        assert!(Dialect::sqlite_with(None, Some(valid)).is_ok(), "{valid}");
    }
    for invalid in ["aBc", "1abc", "1@_2_3", "", "!"] {
        assert!(
            Dialect::sqlite_with(None, Some(invalid)).is_err(),
            "{invalid}"
        );
    }
}

/// Verifies that `table_name()` produces the correct string in all four
/// configurations: default, prefix-only, schema-only, and both.
#[test]
fn table_name_format() {
    let name = |prefix, schema| {
        Migrator::<Sqlite>::new(Dialect::sqlite_with(prefix, schema).unwrap()).table_name()
    };
    assert_eq!(name(None, None), "_sqlx_migrator_migrations");
    assert_eq!(
        name(Some("myprefix"), None),
        "_myprefix_sqlx_migrator_migrations"
    );
    assert_eq!(
        name(None, Some("myschema")),
        "myschema._sqlx_migrator_migrations"
    );
    assert_eq!(
        name(Some("pfx"), Some("sch")),
        "sch._pfx_sqlx_migrator_migrations"
    );
}

// ── Migration / Operation unit tests ─────────────────────────────────────────

/// Verifies the `AppliedMigrationSqlRow` accessor methods return the values
/// they were constructed with.
#[test]
fn applied_migration_sql_row_accessors() {
    let row = AppliedMigrationSqlRow::new(42, "myapp", "first_migration");
    assert_eq!(row.id(), 42);
    assert_eq!(row.app(), "myapp");
    assert_eq!(row.name(), "first_migration");
    // `applied_time` is empty when created via the test constructor.
    assert_eq!(row.applied_time(), OffsetDateTime::UNIX_EPOCH);
}

/// Two `dyn Migration` values are equal when their `app` and `name` match,
/// regardless of any other fields.  Equality is defined purely on the identity
/// pair `(app, name)`.
#[test]
fn migration_equality() {
    let m1: Box<dyn Migration<Sqlite>> = Box::new(("app", "name"));
    let m2: Box<dyn Migration<Sqlite>> = Box::new(("app", "name"));
    // Same (app, name) → fields must be identical.
    assert_eq!(m1.app(), m2.app(), "apps must match for equal migrations");
    assert_eq!(
        m1.name(),
        m2.name(),
        "names must match for equal migrations"
    );

    // Different name → name field must differ.
    let m3: Box<dyn Migration<Sqlite>> = Box::new(("app", "other"));
    assert_eq!(m1.app(), m3.app(), "app is shared");
    assert_ne!(m1.name(), m3.name(), "different names must not be equal");

    // Different app → app field must differ.
    let m4: Box<dyn Migration<Sqlite>> = Box::new(("other_app", "name"));
    assert_ne!(m1.app(), m4.app(), "different apps must not be equal");
    assert_eq!(m1.name(), m4.name(), "name is shared");
}

/// A `(A, N)` tuple implements `Migration` as a *virtual* migration with no
/// parents, operations, replaces, or run before entries.
#[test]
fn tuple_migration_is_virtual() {
    // The explicit `&dyn Migration<Sqlite>` annotation disambiguates which
    // `Migration` impl to dispatch through (tuples are generic over the DB).
    let m: &dyn Migration<Sqlite> = &("myapp", "mymig");
    assert!(m.is_virtual(), "tuple migration must be virtual");
    assert!(m.parents().is_empty(), "tuple must have no parents");
    assert!(m.operations().is_empty(), "tuple must have no operations");
    assert!(m.replaces().is_empty(), "tuple must have no replaces");
    assert!(m.run_before().is_empty(), "tuple must have no run_before");
}

/// The default implementation of `Operation::down` returns
/// `Error::IrreversibleOperation`, indicating the operation cannot be reverted
/// unless a custom `down` is provided.
#[tokio::test]
async fn operation_default_down_is_irreversible() {
    struct NoDownOp;

    #[async_trait::async_trait]
    impl Operation<Sqlite> for NoDownOp {
        async fn up(&self, _connection: &mut sqlx::SqliteConnection) -> Result<(), Error> {
            Ok(())
        }
        // `down` is intentionally not overridden — uses the default.
    }

    let op = NoDownOp;
    let mut conn = make_conn().await;
    let result = op.down(&mut conn).await;
    assert!(
        matches!(result, Err(Error::IrreversibleOperation)),
        "expected IrreversibleOperation, got: {result:?}"
    );
}

/// The default implementation of `Operation::is_destructible` returns `false`.
/// Implementors must explicitly opt-in to mark an operation as destructive.
#[test]
fn operation_is_destructible_default() {
    struct DefaultOp;

    #[async_trait::async_trait]
    impl Operation<Sqlite> for DefaultOp {
        async fn up(&self, _connection: &mut sqlx::SqliteConnection) -> Result<(), Error> {
            Ok(())
        }
    }

    assert!(
        !DefaultOp.is_destructible(),
        "default is_destructible must be false"
    );
}

/// Adding the same migration twice with an identical definition is idempotent —
/// it succeeds and results in only one entry in the migration list.
#[test]
fn add_migration_duplicate_consistent_is_ok() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());

    let mut migrator = TestMigrator::default();
    assert!(migrator.add_migration(Box::new(A)).is_ok());
    assert!(
        migrator.add_migration(Box::new(A)).is_ok(),
        "second identical add must succeed"
    );
    assert_eq!(
        migrator.migrations().len(),
        1,
        "duplicate add must not create a second entry"
    );
}

/// Adding a *virtual* migration that has non-empty `parents`, `operations`,
/// `replaces`, or `run_before` returns `Error::InvalidVirtualMigration`.
#[test]
fn add_invalid_virtual_migration_is_error() {
    /// A virtual migration that illegally declares parents.
    struct BadVirtual;

    impl Migration<Sqlite> for BadVirtual {
        fn app(&self) -> &'static str {
            "test"
        }

        fn name(&self) -> &'static str {
            "bad_virtual"
        }

        fn parents(&self) -> Vec<Box<dyn Migration<Sqlite>>> {
            // Non-empty parents make this an invalid virtual migration.
            vec_box!(("test", "some_parent"))
        }

        fn operations(&self) -> Vec<Box<dyn Operation<Sqlite>>> {
            vec![]
        }

        fn is_virtual(&self) -> bool {
            true
        }
    }

    let mut migrator = TestMigrator::default();
    let result = migrator.add_migration(Box::new(BadVirtual));
    assert!(
        matches!(result, Err(Error::InvalidVirtualMigration)),
        "expected InvalidVirtualMigration, got: {result:?}"
    );
}

/// Verifies the `Error` enum's `Display` output matches the documented strings.
#[test]
fn error_display_messages() {
    assert_eq!(
        Error::IrreversibleOperation.to_string(),
        "operation is irreversible"
    );
    assert_eq!(
        Error::InvalidTablePrefix.to_string(),
        "table prefix name can only contain [a-z0-9_]"
    );
    assert_eq!(
        Error::InvalidSchema.to_string(),
        "schema name can only contain [a-z0-9_] and begin with [a-z_]"
    );
    assert_eq!(
        Error::InconsistentMigration {
            app: "myapp".into(),
            name: "mymig".into()
        }
        .to_string(),
        "inconsistent migration found for myapp - mymig"
    );
    assert_eq!(
        Error::PlanError {
            message: "test error".into()
        }
        .to_string(),
        "plan error: test error"
    );
}
