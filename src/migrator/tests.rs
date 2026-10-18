use std::ops::{Deref, DerefMut};

use sqlx::{Database, Sqlite, SqlitePool};

use super::{Dialect, MigrationVecResult, Migrator};
use crate::error::Error;
use crate::migration::{AppliedMigrationSqlRow, Migration};
use crate::migrator::Plan;
use crate::operation::Operation;
use crate::sync::OldMigrator;
use crate::vec_box;

/// Verifies that the table prefix is accepted when it holds lowercase,
/// digits and underscores and rejected otherwise (uppercase, special chars,
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

/// Migrator with an in memory list of applied rows, so the planner tests
/// run without a database.
#[derive(Default)]
struct TestMigrator {
    migrator: Migrator<Sqlite>,
    applied_migrations: Vec<AppliedMigrationSqlRow>,
}

impl Deref for TestMigrator {
    type Target = Migrator<Sqlite>;

    fn deref(&self) -> &Self::Target {
        &self.migrator
    }
}

impl DerefMut for TestMigrator {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.migrator
    }
}

impl TestMigrator {
    fn add_applied_migrations(&mut self, migrations: Vec<Box<dyn Migration<Sqlite>>>) {
        for migration in migrations {
            self.add_applied_migration(migration.as_ref());
        }
    }

    fn add_applied_migration(&mut self, migration: &dyn Migration<Sqlite>) {
        let id = i32::try_from(self.applied_migrations.len()).unwrap();
        self.applied_migrations.push(AppliedMigrationSqlRow::new(
            id,
            migration.app(),
            migration.name(),
        ));
    }
}

/// The in memory rows double as an old migrator for the sync tests.
#[async_trait::async_trait]
impl OldMigrator<Sqlite> for TestMigrator {
    async fn applied_migrations(
        &self,
        _connection: &mut <Sqlite as Database>::Connection,
    ) -> Result<Vec<Box<dyn Migration<Sqlite>>>, Error> {
        Ok(self
            .applied_migrations
            .iter()
            .map(|row| {
                Box::new((row.app().to_string(), row.name().to_string()))
                    as Box<dyn Migration<Sqlite>>
            })
            .collect())
    }
}

/// Convenience macro for implementing `Migration<Sqlite>` for a test struct.
///
/// Arguments in order: `type`, `name`, `parents`, `replaces`, `run_before`.
/// The `app` is always `"test"` and `operations` is always a single no-op
/// SQL pair so plan-generation tests stay focused on ordering logic.
macro_rules! migration {
    ($op:ty, $name:literal, $parents:expr, $replaces:expr, $run_before:expr) => {
        impl crate::migration::Migration<sqlx::Sqlite> for $op {
            fn app(&self) -> &str {
                "test"
            }

            fn name(&self) -> &str {
                $name
            }

            fn parents(&self) -> Vec<Box<dyn crate::migration::Migration<sqlx::Sqlite>>> {
                $parents
            }

            fn operations(&self) -> Vec<Box<dyn crate::operation::Operation<sqlx::Sqlite>>> {
                vec_box![("", "")]
            }

            fn replaces(&self) -> Vec<Box<dyn crate::migration::Migration<sqlx::Sqlite>>> {
                $replaces
            }

            fn run_before(&self) -> Vec<Box<dyn crate::migration::Migration<sqlx::Sqlite>>> {
                $run_before
            }
        }
    };
}

/// Creates an in-memory `SQLite` connection for use in async tests.
async fn make_conn() -> sqlx::pool::PoolConnection<Sqlite> {
    SqlitePool::connect("sqlite::memory:")
        .await
        .unwrap()
        .acquire()
        .await
        .unwrap()
}

/// Generates a migration plan for the given `plan` against `migrator` and
/// its in memory applied rows.
fn apply_plan<'a>(migrator: &'a TestMigrator, plan: &Plan) -> MigrationVecResult<'a, Sqlite> {
    migrator.generate_migration_plan(Some(plan), &migrator.applied_migrations)
}

/// Adds all `migration_list` entries to `migrator`, then returns the full
/// apply-all plan. This is the most common setup pattern for plan tests.
fn generate_apply_all_plan(
    migrator: &mut TestMigrator,
    migration_list: Vec<Box<dyn Migration<Sqlite>>>,
) -> MigrationVecResult<'_, Sqlite> {
    migrator.add_migrations(migration_list)?;
    apply_plan(migrator, &Plan::apply_all())
}

/// Extracts the `name()` of every migration in a plan into a `Vec<&str>`.
///
/// Using names instead of comparing `Box<dyn Migration<Sqlite>>` values gives
/// much cleaner failure messages (e.g. `["a", "b", "c"]` vs raw pointer
/// comparisons), and is sufficient because every test uses unique names.
#[expect(
    clippy::borrowed_box,
    reason = "plan slices hold &Box<dyn Migration> from generate_migration_plan"
)]
fn plan_names<'a>(plan: &'a [&'a Box<dyn Migration<Sqlite>>]) -> Vec<&'a str> {
    plan.iter().map(|m| m.name()).collect()
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
    assert_eq!(row.applied_time(), "");
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

// ── Plan generation tests: apply ─────────────────────────────────────────────

/// Verifies the happy-path ordering for a simple linear dependency chain
/// A → B → C. The plan must apply A first, then B, then C.
#[test]
fn linear_dependency_chain_applies_in_order() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    struct C;
    migration!(C, "c", vec_box!(B), vec_box!(), vec_box!());
    let mut migrator = TestMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C)).unwrap();
    assert_eq!(plan_names(&plan), vec!["a", "b", "c"]);
}

/// Generating a plan when no migrations have been registered must return a
/// `PlanError` with a clear message rather than silently producing an empty
/// plan.
#[test]
fn apply_all_with_no_migrations_is_error() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    let _: Vec<Box<dyn Migration<Sqlite>>> = vec_box!(A, B);
    let mut migrator = TestMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!());
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: no migrations are added to the migration list".to_string())
    );
}

/// Adding two migrations with the same (app, name) but different definitions
/// (e.g. different parents) must return `Error::InconsistentMigration` so that
/// accidental duplicates are caught at registration time.
#[test]
fn inconsistent_duplicate_migration_name_is_error() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    struct C;
    migration!(C, "c", vec_box!(), vec_box!(), vec_box!());
    // D has the same name "b" as B but different parents → inconsistent.
    struct D;
    migration!(D, "b", vec_box!(C), vec_box!(), vec_box!());
    let mut migrator = TestMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D));
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("inconsistent migration found for test - b".to_string())
    );
}

/// A circular parent chain (A depends on B, B depends on A) must fail with a
/// deadlock error — the planner cannot find a valid ordering.
#[test]
fn circular_parent_dependency_deadlocks() {
    struct A;
    migration!(A, "a", vec_box!(B), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    let mut migrator = TestMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B));
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: reached deadlock stage during plan generation".to_string())
    );
}

/// A mutual `replaces` relationship (A replaces B and B replaces A) must return
/// a "two migrations replace each other" error. (Previously misnamed as
/// `run_before_interrelated_test`.)
#[test]
fn replaces_interrelated_test() {
    // macro arg order: parents, replaces, run_before
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(B), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(), vec_box!(A), vec_box!());
    let mut migrator = TestMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B));
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: two migrations replace each other".to_string())
    );
}

/// A mutual `run_before` relationship (A must run before B and B must run
/// before A) creates an ordering deadlock. (Previously misnamed as
/// `replace_interrelated_test`.)
#[test]
fn run_before_interrelated_test() {
    // macro arg order: parents, replaces, run_before
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!(B));
    struct B;
    migration!(B, "b", vec_box!(), vec_box!(), vec_box!(A));
    let mut migrator = TestMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B));
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: reached deadlock stage during plan generation".to_string())
    );
}

/// A migration that lists itself as its own parent creates an unresolvable
/// dependency and must fail with a deadlock error.
#[test]
fn self_referencing_parent_deadlocks() {
    struct A;
    migration!(A, "a", vec_box!(A), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(B), vec_box!(), vec_box!());
    let mut migrator = TestMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B));
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: reached deadlock stage during plan generation".to_string())
    );
}

/// A migration that lists itself in `replaces` (replacing itself) creates a
/// circular replacement and must fail. (Previously misnamed as
/// `run_before_depend_on_itself`.)
#[test]
fn replaces_depend_on_itself() {
    // macro arg order: parents, replaces, run_before
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(A), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(), vec_box!(B), vec_box!());
    let mut migrator = TestMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B));
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: two migrations replace each other".to_string())
    );
}

/// A migration that lists itself in `run_before` (must run before itself)
/// creates an ordering deadlock and must fail. (Previously misnamed as
/// `replace_depend_on_itself`.)
#[test]
fn run_before_depend_on_itself() {
    // macro arg order: parents, replaces, run_before
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!(A));
    struct B;
    migration!(B, "b", vec_box!(), vec_box!(), vec_box!(B));
    let mut migrator = TestMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B));
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: reached deadlock stage during plan generation".to_string())
    );
}

/// When C replaces B (which depends on A), and D replaces C, the chain B→C is
/// collapsed: the plan must contain only A and D (B and C are skipped).
#[test]
fn replaced_migrations_skipped_in_plan() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    // C replaces B
    struct C;
    migration!(C, "c", vec_box!(), vec_box!(B), vec_box!());
    // D replaces C (and transitively B)
    struct D;
    migration!(D, "d", vec_box!(), vec_box!(C), vec_box!());
    let mut migrator = TestMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D)).unwrap();
    // B and C are superseded by D; only A and D remain.
    assert_eq!(plan_names(&plan), vec!["a", "d"]);
}

/// `apply_name` targeting a replacer must include the parents of every
/// migration in its transitive replace chain.
///
/// D replaces C, C replaces B, B depends on A. The full apply-all plan is
/// [A, D]. A targeted `apply_name("test", "d")` must still include A —
/// even though D itself declares no direct parents — because D inherits the
/// dependency on A through the B→C→D replace chain.
#[test]
fn apply_name_for_replacer_includes_transitive_replace_parents() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    // C replaces B
    struct C;
    migration!(C, "c", vec_box!(), vec_box!(B), vec_box!());
    // D replaces C (and transitively B)
    struct D;
    migration!(D, "d", vec_box!(), vec_box!(C), vec_box!());

    let mut migrator = TestMigrator::default();
    migrator.add_migrations(vec_box!(A, B, C, D)).unwrap();

    let plan = apply_plan(&migrator, &Plan::apply_name("test", Some("d"))).unwrap();
    // A must be included: D needs it through the replace chain (B→C→D).
    assert_eq!(plan_names(&plan), vec!["a", "d"]);
}

/// `revert_name` targeting a migration must include any applied replacer that
/// transitively depends on it through the replace chain.
///
/// D replaces C, C replaces B, B depends on A. All are applied. Reverting A
/// requires reverting D first — because D was ordered after A in the apply
/// plan (inherited from B's dependency). A targeted `revert_name("test",
/// "a")` must therefore include D before A.
#[test]
fn revert_name_for_parent_includes_transitive_replace_dependents() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    // C replaces B
    struct C;
    migration!(C, "c", vec_box!(), vec_box!(B), vec_box!());
    // D replaces C (and transitively B)
    struct D;
    migration!(D, "d", vec_box!(), vec_box!(C), vec_box!());

    let mut migrator = TestMigrator::default();
    migrator.add_migrations(vec_box!(A, B, C, D)).unwrap();
    // A and D are applied (B and C were replaced by D).
    migrator.add_applied_migrations(vec_box!(A, D));

    let plan = apply_plan(&migrator, &Plan::revert_name("test", Some("a"))).unwrap();
    // D must be reverted before A (D depends on A through the replace chain).
    assert_eq!(plan_names(&plan), vec!["d", "a"]);
}

/// When C declares `run_before(B)` the ordering must be:
/// A (B's parent) → D (C's `run_before` provider) → C (must precede B) → B.
///
/// `run_before` reverses intuition: C runs *before* B even though C is listed
/// after B in the migration list.
#[test]
fn run_before_ordering_is_respected() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    // C must run before B
    struct C;
    migration!(C, "c", vec_box!(), vec_box!(), vec_box!(B));
    // D must run before C
    struct D;
    migration!(D, "d", vec_box!(), vec_box!(), vec_box!(C));
    let mut migrator = TestMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D)).unwrap();
    assert_eq!(plan_names(&plan), vec!["a", "d", "c", "b"]);
}

/// Two migrations (C and D) both replacing the same migration (B) is illegal —
/// only one migration may replace a given migration.
#[test]
fn migration_replaced_by_multiple_migrations_is_error() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    // C replaces B
    struct C;
    migration!(C, "c", vec_box!(), vec_box!(B), vec_box!());
    // D also replaces B — conflict!
    struct D;
    migration!(D, "d", vec_box!(), vec_box!(B), vec_box!());
    let mut migrator = TestMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D));
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: migration test:b replaced multiple times".to_string())
    );
}

/// C replaces B; D has `run_before(B)`.  Since B is replaced by C, the
/// `run_before` relationship transfers: D must run before C.  Plan: A, D, C.
#[test]
fn replaced_migration_run_before_transfers_to_replacer() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    // C replaces B
    struct C;
    migration!(C, "c", vec_box!(), vec_box!(B), vec_box!());
    // D must run before B (which is replaced by C)
    struct D;
    migration!(D, "d", vec_box!(), vec_box!(), vec_box!(B));
    let mut migrator = TestMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D)).unwrap();
    assert_eq!(plan_names(&plan), vec!["a", "d", "c"]);
}

/// Chain: C replaces B (B→A parent), D replaces C, E has `run_before(C)`.
/// Since C is replaced by D, E must run before D. Plan: A, E, D.
#[test]
fn chained_replacer_inherits_run_before_constraint() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    struct C;
    migration!(C, "c", vec_box!(), vec_box!(B), vec_box!());
    struct D;
    migration!(D, "d", vec_box!(), vec_box!(C), vec_box!());
    struct E;
    migration!(E, "e", vec_box!(), vec_box!(), vec_box!(C));
    let mut migrator = TestMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D, E)).unwrap();
    assert_eq!(plan_names(&plan), vec!["a", "e", "d"]);
}

/// Similar to `chained_replacer_inherits_run_before_constraint` but E has
/// `run_before(D)` instead of `run_before(C)`.
/// D replaces C which replaces B (B→A). E must precede D. Plan: A, E, D.
#[test]
fn run_before_on_final_replacer_is_respected() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    struct C;
    migration!(C, "c", vec_box!(), vec_box!(B), vec_box!());
    struct D;
    migration!(D, "d", vec_box!(), vec_box!(C), vec_box!());
    struct E;
    migration!(E, "e", vec_box!(), vec_box!(), vec_box!(D));
    let mut migrator = TestMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D, E)).unwrap();
    assert_eq!(plan_names(&plan), vec!["a", "e", "d"]);
}

/// D replaces both C and E; E has `run_before(C)`.  Because D absorbs both
/// replaced migrations (C and E), only A and D appear in the final plan.
#[test]
fn replacer_absorbing_all_run_before_parties_collapses_plan() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    struct C;
    migration!(C, "c", vec_box!(), vec_box!(B), vec_box!());
    // D replaces both C and E
    struct D;
    migration!(D, "d", vec_box!(), vec_box!(C, E), vec_box!());
    struct E;
    migration!(E, "e", vec_box!(), vec_box!(), vec_box!(C));
    let mut migrator = TestMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D, E)).unwrap();
    assert_eq!(plan_names(&plan), vec!["a", "d"]);
}

/// D replaces C and also has `run_before(C)`, which is a self-contradictory
/// circular requirement and must deadlock.
#[test]
fn replacer_with_run_before_on_replaced_deadlocks() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    struct C;
    migration!(C, "c", vec_box!(), vec_box!(B), vec_box!());
    // D replaces C AND must run before C — contradiction!
    struct D;
    migration!(D, "d", vec_box!(), vec_box!(C), vec_box!(C));
    let mut migrator = TestMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D));
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: reached deadlock stage during plan generation".to_string())
    );
}

/// D replaces C and E; E replaces D and has `run_before(C)`. This creates a
/// mutual-replacement cycle between D and E, which must be detected.
#[test]
fn mutual_replacers_with_run_before_is_error() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    struct C;
    migration!(C, "c", vec_box!(), vec_box!(B), vec_box!());
    struct D;
    migration!(D, "d", vec_box!(), vec_box!(C, E), vec_box!());
    // E replaces D AND has run_before(C) — D and E replace each other!
    struct E;
    migration!(E, "e", vec_box!(), vec_box!(D), vec_box!(C));
    let mut migrator = TestMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D, E));
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: two migrations replace each other".to_string())
    );
}

/// C replaces B and has `run_before(D)`; D replaces C. This means C must run
/// before D and D must come after C, but C is replaced by D — deadlock.
#[test]
fn replaced_run_before_its_replacer_deadlocks() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    // C replaces B and declares `run_before(D)`
    struct C;
    migration!(C, "c", vec_box!(), vec_box!(B), vec_box!(D));
    // D replaces C — creates the deadlock
    struct D;
    migration!(D, "d", vec_box!(), vec_box!(C), vec_box!());
    let mut migrator = TestMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D));
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: reached deadlock stage during plan generation".to_string())
    );
}

/// B depends on A and has `run_before(A)`: B must come *after* A (parent) but
/// *before* A (`run_before`) — an impossible ordering that must deadlock.
#[test]
fn parent_and_run_before_same_migration_deadlocks() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    // B depends on A as a parent, but also declares `run_before(A)`.
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!(A));
    let mut migrator = TestMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B));
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: reached deadlock stage during plan generation".to_string())
    );
}

/// If the database records B as applied but A (B's parent) is not applied, the
/// plan generator must report an integrity error rather than silently producing
/// an incorrect plan.
#[test]
fn child_applied_without_parent_is_error() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    let mut migrator = TestMigrator::default();
    // Mark B as applied without A — simulates a corrupted migration state.
    migrator.add_applied_migrations(vec_box!(B));
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B));
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some(
            "plan error: child migration test:b applied before its parent migration test:a"
                .to_string()
        )
    );
}

/// When the grandchild squash migration D (which transitively replaces B via
/// C) is already applied together with A, `apply_all` must produce an empty
/// plan — everything is considered up-to-date.
#[test]
fn transitive_replacer_fully_applied_gives_empty_plan() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    // C replaces B
    struct C;
    migration!(C, "c", vec_box!(), vec_box!(B), vec_box!());
    // D replaces C (and therefore transitively replaces B)
    struct D;
    migration!(D, "d", vec_box!(), vec_box!(C), vec_box!());
    let mut migrator = TestMigrator::default();
    // A and D are already applied — no remaining work.
    migrator.add_applied_migrations(vec_box!(A, D));
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(D, C, B, A)).unwrap();
    assert!(plan.is_empty(), "all migrations already applied via squash");
}

/// When parents and replaces are expressed as virtual (tuple) migrations, the
/// planner must resolve them to the concrete registered migrations.
/// C replaces B (B depends on A); D replaces C. Only A and D remain.
#[test]
fn virtual_references_in_replace_chain_resolves() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(
        B,
        "b",
        vec_box!((A.app(), A.name())),
        vec_box!(),
        vec_box!()
    );
    struct C;
    migration!(
        C,
        "c",
        vec_box!(),
        vec_box!((B.app(), B.name())),
        vec_box!()
    );
    struct D;
    migration!(
        D,
        "d",
        vec_box!(),
        vec_box!((C.app(), C.name())),
        vec_box!()
    );
    let mut migrator = TestMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(B, C, D, A)).unwrap();
    assert_eq!(plan_names(&plan), vec!["a", "d"]);
}

/// A virtual (tuple) migration that is never replaced by a concrete migration
/// must cause a `PlanError` — it has no real implementation.
#[test]
fn unreplaced_virtual_migration_is_error() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    let mut migrator = TestMigrator::default();
    // ("test", "b") is a virtual placeholder with no concrete replacement.
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, ("test", "b")));
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: virtual migration which is not replaced is present".to_string())
    );
}

/// A virtual placeholder may appear in the migration list multiple times; it is
/// deduplicated automatically. Once the concrete migration B is also
/// registered, the plan must resolve correctly: A then B.
#[test]
fn virtual_migration_with_concrete_replacement_resolves() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    let mut migrator = TestMigrator::default();
    // The virtual ("test", "b") appears twice; B is its concrete replacement.
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, ("test", "b"), B, ("test", "b")))
        .unwrap();
    assert_eq!(plan_names(&plan), vec!["a", "b"]);
}

/// When B's parent is expressed as a virtual (tuple) reference instead of a
/// direct struct, the dependency must still be resolved correctly — A before B.
#[test]
fn virtual_parent_reference_resolves_dependency_order() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    // B references A via a virtual (tuple) parent.
    struct B;
    migration!(B, "b", vec_box!(("test", "a")), vec_box!(), vec_box!());
    let mut migrator = TestMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(B, A)).unwrap();
    // A must precede B regardless of registration order.
    assert_eq!(plan.len(), 2, "plan must contain both migrations");
    let names = plan_names(&plan);
    let pos_a = names.iter().position(|&n| n == "a").unwrap();
    let pos_b = names.iter().position(|&n| n == "b").unwrap();
    assert!(pos_a < pos_b, "A must precede B: {names:?}");
}

/// Validates apply and targeted apply plans for a diamond-tree topology:
///
/// ```text
///        A
///        |
///        B
///       / \
///      C   D
///      |   |
///      E   F
///      |
///      G
/// ```
///
/// All parents are expressed as virtual tuple references.
/// - `apply_all` must traverse in dependency order: A B C D E F G.
/// - `apply_name("test", Some("f"))` must include only the F-branch: A B D F.
/// - `apply_name("test", Some("g"))` must include only the G-branch: A B C E G.
#[test]
fn apply_plan_diamond_topology_respects_branches() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(
        B,
        "b",
        vec_box!((A.app(), A.name())),
        vec_box!(),
        vec_box!()
    );
    struct C;
    migration!(
        C,
        "c",
        vec_box!((B.app(), B.name())),
        vec_box!(),
        vec_box!()
    );
    struct D;
    migration!(
        D,
        "d",
        vec_box!((B.app(), B.name())),
        vec_box!(),
        vec_box!()
    );
    struct E;
    migration!(
        E,
        "e",
        vec_box!((C.app(), C.name())),
        vec_box!(),
        vec_box!()
    );
    struct F;
    migration!(
        F,
        "f",
        vec_box!((D.app(), D.name())),
        vec_box!(),
        vec_box!()
    );
    struct G;
    migration!(
        G,
        "g",
        vec_box!((E.app(), E.name())),
        vec_box!(),
        vec_box!()
    );
    let mut migrator = TestMigrator::default();
    migrator
        .add_migrations(vec_box!(A, B, C, D, E, F, G))
        .unwrap();
    // Full apply plan: all seven migrations in topological order.
    let full_plan = apply_plan(&migrator, &Plan::apply_all()).unwrap();
    assert_eq!(
        plan_names(&full_plan),
        vec!["a", "b", "c", "d", "e", "f", "g"]
    );

    // Targeted apply up to F: only the F-branch (A→B→D→F).
    let plan_till_f = apply_plan(&migrator, &Plan::apply_name("test", Some("f"))).unwrap();
    assert_eq!(plan_names(&plan_till_f), vec!["a", "b", "d", "f"]);

    // Targeted apply up to G: only the G-branch (A→B→C→E→G).
    let plan_till_g = apply_plan(&migrator, &Plan::apply_name("test", Some("g"))).unwrap();
    assert_eq!(plan_names(&plan_till_g), vec!["a", "b", "c", "e", "g"]);
}

// ── Plan generation tests: revert ────────────────────────────────────────────

/// Using the same diamond topology as
/// `apply_plan_diamond_topology_respects_branches`, verifies:
/// - `revert_all` reverses every migration in reverse dependency order.
/// - `revert_name("test", Some("f"))` reverts only F (leaf with no dependents).
/// - `revert_name("test", Some("b"))` reverts all descendants of B before B.
#[test]
fn revert_plan_diamond_topology_reverses_correctly() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(
        B,
        "b",
        vec_box!((A.app(), A.name())),
        vec_box!(),
        vec_box!()
    );
    struct C;
    migration!(
        C,
        "c",
        vec_box!((B.app(), B.name())),
        vec_box!(),
        vec_box!()
    );
    struct D;
    migration!(
        D,
        "d",
        vec_box!((B.app(), B.name())),
        vec_box!(),
        vec_box!()
    );
    struct E;
    migration!(
        E,
        "e",
        vec_box!((C.app(), C.name())),
        vec_box!(),
        vec_box!()
    );
    struct F;
    migration!(
        F,
        "f",
        vec_box!((D.app(), D.name())),
        vec_box!(),
        vec_box!()
    );
    struct G;
    migration!(
        G,
        "g",
        vec_box!((E.app(), E.name())),
        vec_box!(),
        vec_box!()
    );
    let mut migrator = TestMigrator::default();
    migrator
        .add_migrations(vec_box!(A, B, C, D, E, F, G))
        .unwrap();
    migrator.add_applied_migrations(vec_box!(A, B, C, D, E, F, G));
    // Full revert: reverse of apply order.
    let revert_plan = apply_plan(&migrator, &Plan::revert_all()).unwrap();
    assert_eq!(
        plan_names(&revert_plan),
        vec!["g", "f", "e", "d", "c", "b", "a"]
    );

    // Targeted revert of F only (leaf — no dependents to pull in).
    let plan_till_f = apply_plan(&migrator, &Plan::revert_name("test", Some("f"))).unwrap();
    assert_eq!(plan_names(&plan_till_f), vec!["f"]);

    // Targeted revert down to B: must revert all B-descendants first.
    let plan_till_b = apply_plan(&migrator, &Plan::revert_name("test", Some("b"))).unwrap();
    assert_eq!(plan_names(&plan_till_b), vec!["g", "f", "e", "d", "c", "b"]);
}

/// When no migrations are applied, `revert_all` must produce an empty plan
/// rather than erroring.
#[test]
fn revert_all_no_applied_gives_empty_plan() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());

    let mut migrator = TestMigrator::default();
    migrator.add_migrations(vec_box!(A, B)).unwrap();

    let plan = apply_plan(&migrator, &Plan::revert_all()).unwrap();
    assert!(plan.is_empty(), "no applied migrations → empty revert plan");
}

/// `revert_name` with no migration name reverts all applied migrations for the
/// given app in reverse dependency order.
#[test]
fn revert_app_name_reverts_all_for_app() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());

    let mut migrator = TestMigrator::default();
    migrator.add_migrations(vec_box!(A, B)).unwrap();
    migrator.add_applied_migrations(vec_box!(A, B));

    // revert_name with no specific migration → revert entire app
    let plan = apply_plan(&migrator, &Plan::revert_name("test", None)).unwrap();
    let names = plan_names(&plan);
    // Both A and B must be reverted; B must come before A.
    assert!(names.contains(&"a"), "A missing: {names:?}");
    assert!(names.contains(&"b"), "B missing: {names:?}");
    let pos_a = names.iter().position(|&n| n == "a").unwrap();
    let pos_b = names.iter().position(|&n| n == "b").unwrap();
    assert!(pos_b < pos_a, "B must be reverted before A: {names:?}");
}

// ── Count plan tests
// ──────────────────────────────────────────────────────────

/// `apply_count(2)` on three pending migrations must return exactly the first
/// two in dependency order.
#[test]
fn apply_count_limits_to_n_pending() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(
        B,
        "b",
        vec_box!((A.app(), A.name())),
        vec_box!(),
        vec_box!()
    );
    struct C;
    migration!(
        C,
        "c",
        vec_box!((B.app(), B.name())),
        vec_box!(),
        vec_box!()
    );

    let mut migrator = TestMigrator::default();
    migrator.add_migrations(vec_box!(A, B, C)).unwrap();

    let plan = apply_plan(&migrator, &Plan::apply_count(2)).unwrap();
    assert_eq!(plan_names(&plan), vec!["a", "b"]);
}

/// `revert_count(1)` on three applied migrations must revert only the most
/// recently applied one (the last in apply order).
#[test]
fn revert_count_reverts_last_n_applied() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(
        B,
        "b",
        vec_box!((A.app(), A.name())),
        vec_box!(),
        vec_box!()
    );
    struct C;
    migration!(
        C,
        "c",
        vec_box!((B.app(), B.name())),
        vec_box!(),
        vec_box!()
    );

    let mut migrator = TestMigrator::default();
    migrator.add_migrations(vec_box!(A, B, C)).unwrap();
    migrator.add_applied_migrations(vec_box!(A, B, C));

    let plan = apply_plan(&migrator, &Plan::revert_count(1)).unwrap();
    // revert_count(1) should revert C only (the last applied)
    assert_eq!(plan_names(&plan), vec!["c"]);
}

/// `apply_count(0)` is invalid and must return an error with a clear message.
#[test]
fn apply_count_zero_is_error() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());

    let mut migrator = TestMigrator::default();
    migrator.add_migrations(vec_box!(A)).unwrap();

    let err = apply_plan(&migrator, &Plan::apply_count(0))
        .err()
        .expect("apply_count(0) must return an error");
    assert_eq!(err.to_string(), "plan error: count must be greater than 0");
}

/// `revert_count(0)` is invalid and must return the same error as
/// `apply_count(0)`.
#[test]
fn revert_count_zero_is_error() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());

    let mut migrator = TestMigrator::default();
    migrator.add_migrations(vec_box!(A)).unwrap();
    migrator.add_applied_migrations(vec_box!(A));

    let err = apply_plan(&migrator, &Plan::revert_count(0))
        .err()
        .expect("revert_count(0) must return an error");
    assert_eq!(err.to_string(), "plan error: count must be greater than 0");
}

/// Requesting more migrations than are pending must fail with a descriptive
/// error that includes the actual number of pending migrations available.
#[test]
fn apply_count_exceeds_pending_is_error() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());

    let mut migrator = TestMigrator::default();
    migrator.add_migrations(vec_box!(A)).unwrap();

    // Only 1 migration pending, but we request 5.
    let err = apply_plan(&migrator, &Plan::apply_count(5))
        .err()
        .expect("apply_count exceeding pending must return an error");
    assert_eq!(
        err.to_string(),
        "plan error: passed count value is larger than migration length: 1"
    );
}

/// Requesting to revert more migrations than are applied must fail with a
/// descriptive error that includes the actual number applied.
#[test]
fn revert_count_exceeds_applied_is_error() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());

    let mut migrator = TestMigrator::default();
    migrator.add_migrations(vec_box!(A)).unwrap();
    migrator.add_applied_migrations(vec_box!(A));

    // Only 1 migration applied, but we request to revert 5.
    let err = apply_plan(&migrator, &Plan::revert_count(5))
        .err()
        .expect("revert_count exceeding applied must return an error");
    assert_eq!(
        err.to_string(),
        "plan error: passed count value is larger than migration length: 1"
    );
}

// ── Named plan tests
// ──────────────────────────────────────────────────────────

/// When all migrations are already applied, `apply_all` must return an empty
/// plan (no-op) rather than re-applying anything.
#[test]
fn apply_all_already_applied_gives_empty_plan() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(
        B,
        "b",
        vec_box!((A.app(), A.name())),
        vec_box!(),
        vec_box!()
    );

    let mut migrator = TestMigrator::default();
    migrator.add_migrations(vec_box!(A, B)).unwrap();
    migrator.add_applied_migrations(vec_box!(A, B));

    let plan = apply_plan(&migrator, &Plan::apply_all()).unwrap();
    assert!(plan.is_empty(), "all already applied → plan must be empty");
}

/// When `apply_name` targets a specific migration by name the planner pulls in
/// the full set of required predecessors.
///
/// Topology: N depends on M (parent); N has `run_before(T)`.
/// Requesting T must include M and N (because N must precede T and M must
/// precede N).
#[test]
fn apply_name_run_before_pulls_in_required_parents() {
    struct M;
    migration!(M, "m", vec_box!(), vec_box!(), vec_box!());
    struct N;
    migration!(
        N,
        "n",
        vec_box!((M.app(), M.name())),
        vec_box!(),
        vec_box!((T.app(), T.name()))
    );
    struct T;
    migration!(T, "t", vec_box!(), vec_box!(), vec_box!());

    let mut migrator = TestMigrator::default();
    migrator.add_migrations(vec_box!(M, N, T)).unwrap();

    let plan = apply_plan(&migrator, &Plan::apply_name("test", Some("t"))).unwrap();

    let names = plan_names(&plan);
    // M must come before N; N must come before T
    assert!(names.contains(&"m"), "M missing: {names:?}");
    assert!(names.contains(&"n"), "N missing: {names:?}");
    assert!(names.contains(&"t"), "T missing: {names:?}");
    let pos = |n: &str| names.iter().position(|&x| x == n).unwrap();
    assert!(pos("m") < pos("n"), "M must precede N: {names:?}");
    assert!(pos("n") < pos("t"), "N must precede T: {names:?}");
}

/// Same topology as `apply_name_run_before_pulls_in_required_parents`
/// (M←N→`run_before`→T), but now all migrations are applied. Reverting M must
/// also revert N first (since N's parent is M). T is unrelated to the M-branch
/// on revert.
#[test]
fn revert_name_includes_all_dependent_children() {
    struct M;
    migration!(M, "m", vec_box!(), vec_box!(), vec_box!());
    struct N;
    migration!(
        N,
        "n",
        vec_box!((M.app(), M.name())),
        vec_box!(),
        vec_box!((T.app(), T.name()))
    );
    struct T;
    migration!(T, "t", vec_box!(), vec_box!(), vec_box!());

    let mut migrator = TestMigrator::default();
    migrator.add_migrations(vec_box!(M, N, T)).unwrap();
    migrator.add_applied_migrations(vec_box!(M, N, T));

    let plan = apply_plan(&migrator, &Plan::revert_name("test", Some("m"))).unwrap();

    let names = plan_names(&plan);
    // N must be reverted before M (N depends on M as parent).
    assert!(names.contains(&"m"), "M missing: {names:?}");
    assert!(names.contains(&"n"), "N missing: {names:?}");
    let pos = |n: &str| names.iter().position(|&x| x == n).unwrap();
    assert!(
        pos("n") < pos("m"),
        "N must be reverted before M: {names:?}"
    );
}

/// A diamond dependency (A←B, A←C, B+C←D) must resolve without duplicating A.
/// All four migrations appear exactly once, in a valid topological order.
#[test]
fn diamond_dependency_resolves() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(
        B,
        "b",
        vec_box!((A.app(), A.name())),
        vec_box!(),
        vec_box!()
    );
    struct C;
    migration!(
        C,
        "c",
        vec_box!((A.app(), A.name())),
        vec_box!(),
        vec_box!()
    );
    // D requires both B and C.
    struct D;
    migration!(
        D,
        "d",
        vec_box!((B.app(), B.name()), (C.app(), C.name())),
        vec_box!(),
        vec_box!()
    );

    let mut migrator = TestMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D)).unwrap();
    let names = plan_names(&plan);
    assert_eq!(
        names.len(),
        4,
        "all four migrations must appear exactly once"
    );
    let pos = |n: &str| names.iter().position(|&x| x == n).unwrap();
    assert!(pos("a") < pos("b"), "A before B: {names:?}");
    assert!(pos("a") < pos("c"), "A before C: {names:?}");
    assert!(pos("b") < pos("d"), "B before D: {names:?}");
    assert!(pos("c") < pos("d"), "C before D: {names:?}");
}

/// `apply_name` with no specific migration name applies all pending migrations
/// for the named app.
#[test]
fn apply_by_app_name_only() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(
        B,
        "b",
        vec_box!((A.app(), A.name())),
        vec_box!(),
        vec_box!()
    );

    let mut migrator = TestMigrator::default();
    migrator.add_migrations(vec_box!(A, B)).unwrap();

    // apply_name with no specific migration name → apply all for that app
    let plan = apply_plan(&migrator, &Plan::apply_name("test", None)).unwrap();
    let names = plan_names(&plan);
    assert!(names.contains(&"a"), "A missing: {names:?}");
    assert!(names.contains(&"b"), "B missing: {names:?}");
}

/// `apply_name` referencing a non-existent app must return a `PlanError`
/// describing that the app does not exist.
#[test]
fn apply_name_nonexistent_app_is_error() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());

    let mut migrator = TestMigrator::default();
    migrator.add_migrations(vec_box!(A)).unwrap();

    let err = apply_plan(&migrator, &Plan::apply_name("nonexistent_app", None))
        .err()
        .expect("nonexistent app must return an error");
    assert_eq!(
        err.to_string(),
        "plan error: app nonexistent_app doesn't exist"
    );
}

/// `apply_name` referencing an app that exists but a migration that does not
/// must return a `PlanError` naming the missing migration.
#[test]
fn apply_name_nonexistent_migration_is_error() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());

    let mut migrator = TestMigrator::default();
    migrator.add_migrations(vec_box!(A)).unwrap();

    let err = apply_plan(
        &migrator,
        &Plan::apply_name("test", Some("no_such_migration")),
    )
    .err()
    .expect("nonexistent migration name must return an error");
    assert_eq!(
        err.to_string(),
        "plan error: migration no_such_migration doesn't exist for app test"
    );
}

/// `revert_name` referencing a non-existent app must return a `PlanError`
/// describing that the app does not exist.
#[test]
fn revert_name_nonexistent_app_is_error() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());

    let mut migrator = TestMigrator::default();
    migrator.add_migrations(vec_box!(A)).unwrap();
    migrator.add_applied_migrations(vec_box!(A));

    let err = apply_plan(&migrator, &Plan::revert_name("nonexistent_app", None))
        .err()
        .expect("nonexistent app must return an error");
    assert_eq!(
        err.to_string(),
        "plan error: app nonexistent_app doesn't exist"
    );
}

/// Targeting an app whose migrations are all already applied must produce an
/// empty plan (idempotent apply) instead of an error.
#[test]
fn apply_name_fully_applied_app_returns_empty_plan() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());

    let mut migrator = TestMigrator::default();
    migrator.add_migrations(vec_box!(A)).unwrap();
    migrator.add_applied_migrations(vec_box!(A));

    let plan = apply_plan(&migrator, &Plan::apply_name("test", None)).unwrap();
    assert!(
        plan.is_empty(),
        "expected empty plan: {:?}",
        plan_names(&plan)
    );
}

/// Targeting an already applied migration by name must produce an empty plan
/// (idempotent apply) instead of an error.
#[test]
fn apply_name_already_applied_migration_returns_empty_plan() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());

    let mut migrator = TestMigrator::default();
    migrator.add_migrations(vec_box!(A)).unwrap();
    migrator.add_applied_migrations(vec_box!(A));

    let plan = apply_plan(&migrator, &Plan::apply_name("test", Some("a"))).unwrap();
    assert!(
        plan.is_empty(),
        "expected empty plan: {:?}",
        plan_names(&plan)
    );
}

/// Reverting a migration that was never applied must produce an empty plan
/// (idempotent revert) instead of an error.
#[test]
fn revert_name_nothing_applied_returns_empty_plan() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());

    let mut migrator = TestMigrator::default();
    migrator.add_migrations(vec_box!(A)).unwrap();

    let plan = apply_plan(&migrator, &Plan::revert_name("test", Some("a"))).unwrap();
    assert!(
        plan.is_empty(),
        "expected empty plan: {:?}",
        plan_names(&plan)
    );
}

/// `check_pending` must error while migrations are pending and pass once
/// every migration is applied.
#[tokio::test]
async fn check_pending_errors_only_when_pending_exists() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());

    let mut migrator = Migrator::<Sqlite>::default();
    migrator.add_migrations(vec_box!(A, B)).unwrap();
    let mut conn = make_conn().await;
    migrator
        .run(&mut conn, &Plan::apply_count(1).fake(true))
        .await
        .unwrap();

    let err = migrator
        .check_pending(&mut conn)
        .await
        .expect_err("pending migration must return an error");
    assert!(matches!(err, Error::PendingMigrationPresent));

    migrator
        .run(&mut conn, &Plan::apply_all().fake(true))
        .await
        .unwrap();
    assert!(migrator.check_pending(&mut conn).await.is_ok());
}

/// `status` must return every migration in apply order along with its applied
/// row when applied, and an empty list for a migrator with no migrations.
#[test]
fn status_returns_applied_state_in_apply_order() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());

    let mut migrator = TestMigrator::default();
    migrator.add_migrations(vec_box!(B, A)).unwrap();
    migrator.add_applied_migrations(vec_box!(A));

    let rows = &migrator.applied_migrations;
    let status = migrator.status(rows).unwrap();
    assert_eq!(status.len(), 2);
    assert_eq!(status[0].0.name(), "a");
    assert!(status[0].1.is_some(), "a must be applied");
    assert_eq!(status[1].0.name(), "b");
    assert!(status[1].1.is_none(), "b must be pending");

    let empty_migrator = TestMigrator::default();
    assert!(empty_migrator.status(rows).unwrap().is_empty());
}
/// Both the squash migration (replacer) and one of its replaced predecessors
/// being applied simultaneously is an inconsistent state. The planner must
/// report a `PlanError` rather than silently producing a broken plan.
#[test]
fn replace_and_replacer_both_applied_is_error() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    // C replaces B — they must not both appear in applied migrations.
    struct C;
    migration!(C, "c", vec_box!(), vec_box!(B), vec_box!());

    let mut migrator = TestMigrator::default();
    migrator.add_migrations(vec_box!(A, B, C)).unwrap();
    // Mark both B and C as applied — this is the illegal state.
    migrator.add_applied_migrations(vec_box!(A, B, C));

    let err = apply_plan(&migrator, &Plan::apply_all())
        .err()
        .expect("simultaneously applied replacer and replaced must return an error");
    assert_eq!(
        err.to_string(),
        "plan error: migration test:c and its replaces are applied together"
    );
}

/// `generate_migration_plan` is the synchronous core of the planner.
/// Called with an empty `applied_migration_sql_rows` slice and an `apply_all`
/// plan it must return every migration in apply order.
#[test]
fn generate_plan_with_empty_rows_returns_apply_order() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());

    let mut migrator = TestMigrator::default();
    migrator.add_migrations(vec_box!(A, B)).unwrap();

    // Call the sync variant directly with no applied rows.
    let plan = migrator
        .generate_migration_plan(Some(&Plan::apply_all()), &[])
        .unwrap();
    assert_eq!(plan_names(&plan), vec!["a", "b"]);
}

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
    assert_eq!(rows[0].execution_time_ms(), 0);
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
    migrator.lock(&mut conn).await.unwrap();
    migrator.unlock(&mut conn).await.unwrap();
}

/// Creating the schema is on for Postgres and can be switched off.
#[cfg(feature = "postgres")]
#[test]
fn create_schema_can_be_disabled() {
    let dialect = Dialect::postgres_with(None, Some("sch")).unwrap();
    assert!(dialect.create_schema());
    assert!(!dialect.set_create_schema(false).create_schema());
}

// ── Internal migrations of the migration table ───────────────────────────────

/// Runs a raw statement on the connection, for setting up tampered
/// states the public API cannot produce.
async fn raw(conn: &mut <Sqlite as Database>::Connection, sql: String) {
    sqlx::query(sqlx::AssertSqlSafe(sql))
        .execute(conn)
        .await
        .unwrap();
}

async fn sqlite_table_exists(conn: &mut <Sqlite as Database>::Connection, table: &str) -> bool {
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT 1 FROM {table} WHERE 1 = 0"
    )))
    .execute(conn)
    .await
    .is_ok()
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
    assert_eq!(rows[0].execution_time_ms(), 0);
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

// ── Failed migrations and ledger columns ─────────────────────────────────────

/// Migration of the `main` app running plain SQL, optionally outside of a
/// transaction.
struct SqlMigration {
    name: &'static str,
    parent: Option<&'static str>,
    operations: Vec<(&'static str, &'static str)>,
    atomic: bool,
}

impl SqlMigration {
    fn new(name: &'static str, parent: Option<&'static str>, up: &'static str) -> Self {
        Self {
            name,
            parent,
            operations: vec![(up, "SELECT 1")],
            atomic: true,
        }
    }

    fn non_atomic(mut self) -> Self {
        self.atomic = false;
        self
    }

    fn operation(mut self, up: &'static str) -> Self {
        self.operations.push((up, "SELECT 1"));
        self
    }
}

impl Migration<Sqlite> for SqlMigration {
    fn app(&self) -> &'static str {
        "main"
    }

    fn name(&self) -> &str {
        self.name
    }

    fn parents(&self) -> Vec<Box<dyn Migration<Sqlite>>> {
        self.parent
            .map(|parent| Box::new(("main", parent)) as Box<dyn Migration<Sqlite>>)
            .into_iter()
            .collect()
    }

    fn operations(&self) -> Vec<Box<dyn Operation<Sqlite>>> {
        self.operations
            .iter()
            .map(|(up, down)| {
                Box::new((up.to_string(), down.to_string())) as Box<dyn Operation<Sqlite>>
            })
            .collect()
    }

    fn is_atomic(&self) -> bool {
        self.atomic
    }
}

fn migrator_of(migrations: Vec<SqlMigration>) -> Migrator<Sqlite> {
    let mut migrator = Migrator::<Sqlite>::default();
    migrator
        .add_migrations(
            migrations
                .into_iter()
                .map(|migration| Box::new(migration) as Box<dyn Migration<Sqlite>>)
                .collect(),
        )
        .unwrap();
    migrator
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
    assert_eq!(rows[3].execution_time_ms(), 0);
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
