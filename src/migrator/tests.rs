use sqlx::{Database, Sqlite, SqlitePool};

use super::{DatabaseOperation, Info, Migrate, Migrator};
use crate::error::Error;
use crate::migration::{AppliedMigrationSqlRow, Migration};
use crate::migrator::Plan;
use crate::operation::Operation;
use crate::sync::Synchronize;
use crate::vec_box;

/// Verifies that `set_table_prefix` accepts valid identifiers (lowercase,
/// digits, underscores) and rejects invalid ones (uppercase, special chars,
/// empty string).
#[test]
fn table_name() {
    assert!(Migrator::<Sqlite>::new().set_table_prefix("abc").is_ok());
    assert!(Migrator::<Sqlite>::new().set_table_prefix("aBc").is_err());
    assert!(Migrator::<Sqlite>::new().set_table_prefix("ab1").is_ok());
    assert!(
        Migrator::<Sqlite>::new()
            .set_table_prefix("abc___123")
            .is_ok()
    );
    assert!(Migrator::<Sqlite>::new().set_table_prefix("_123").is_ok());
    assert!(
        Migrator::<Sqlite>::new()
            .set_table_prefix("1@_2_3")
            .is_err()
    );
    assert!(Migrator::<Sqlite>::new().set_table_prefix("").is_err());
}

/// Verifies that `set_schema` accepts valid schema names (must start with a
/// lowercase letter or underscore) and rejects invalid ones.
#[test]
fn schema_name() {
    assert!(Migrator::<Sqlite>::new().set_schema("abc").is_ok());
    assert!(Migrator::<Sqlite>::new().set_schema("aBc").is_err());
    assert!(Migrator::<Sqlite>::new().set_schema("1abc").is_err());
    assert!(Migrator::<Sqlite>::new().set_schema("ab1").is_ok());
    assert!(Migrator::<Sqlite>::new().set_schema("abc___123").is_ok());
    assert!(Migrator::<Sqlite>::new().set_schema("_123").is_ok());
    assert!(Migrator::<Sqlite>::new().set_schema("1@_2_3").is_err());
    assert!(Migrator::<Sqlite>::new().set_schema("").is_err());
    assert!(Migrator::<Sqlite>::new().set_schema("!").is_err());
}

/// Verifies that `table_name()` produces the correct string in all four
/// configurations: default, prefix-only, schema-only, and both.
#[test]
fn table_name_format() {
    // Default: no prefix or schema
    assert_eq!(
        Migrator::<Sqlite>::new().table_name(),
        "_sqlx_migrator_migrations"
    );

    // With prefix only
    let m = Migrator::<Sqlite>::new()
        .set_table_prefix("myprefix")
        .unwrap();
    assert_eq!(m.table_name(), "_myprefix_sqlx_migrator_migrations");

    // With schema only
    let m = Migrator::<Sqlite>::new().set_schema("myschema").unwrap();
    assert_eq!(m.table_name(), "myschema._sqlx_migrator_migrations");

    // With both prefix and schema
    let m = Migrator::<Sqlite>::new()
        .set_table_prefix("pfx")
        .unwrap()
        .set_schema("sch")
        .unwrap();
    assert_eq!(m.table_name(), "sch._pfx_sqlx_migrator_migrations");
}

#[derive(Default)]
struct CustomMigrator {
    internal_migrator: Migrator<Sqlite>,
    migrations: Vec<Box<dyn Migration<Sqlite>>>,
    applied_migrations: Vec<AppliedMigrationSqlRow>,
}

impl CustomMigrator {
    fn add_applied_migrations(
        &mut self,
        migrations: Vec<Box<dyn Migration<Sqlite>>>,
    ) -> Result<(), Error> {
        for migration in migrations {
            self.add_applied_migration(migration)?;
        }
        Ok(())
    }

    fn add_applied_migration(
        &mut self,
        migration: Box<dyn Migration<Sqlite>>,
    ) -> Result<(), Error> {
        let current_length = self.migrations.len();
        self.applied_migrations.push(AppliedMigrationSqlRow::new(
            i32::try_from(current_length).unwrap(),
            migration.app(),
            migration.name(),
        ));
        self.internal_migrator.add_migration(migration)
    }
}

impl Info<Sqlite> for CustomMigrator {
    fn migrations(&self) -> &[Box<dyn Migration<Sqlite>>] {
        &self.migrations
    }

    fn migrations_mut(&mut self) -> &mut Vec<Box<dyn Migration<Sqlite>>> {
        &mut self.migrations
    }
}

#[async_trait::async_trait]
impl DatabaseOperation<Sqlite> for CustomMigrator {
    async fn ensure_migration_table_exists(
        &self,
        _connection: &mut <Sqlite as Database>::Connection,
    ) -> Result<(), Error> {
        Ok(())
    }

    async fn drop_migration_table_if_exists(
        &self,
        _connection: &mut <Sqlite as Database>::Connection,
    ) -> Result<(), Error> {
        Ok(())
    }

    async fn add_migration_to_db_table(
        &self,
        _connection: &mut <Sqlite as Database>::Connection,
        _migration: &dyn Migration<Sqlite>,
    ) -> Result<(), Error> {
        Ok(())
    }

    async fn delete_migration_from_db_table(
        &self,
        _connection: &mut <Sqlite as Database>::Connection,
        _migration: &dyn Migration<Sqlite>,
    ) -> Result<(), Error> {
        Ok(())
    }

    async fn fetch_applied_migration_from_db(
        &self,
        _connection: &mut <Sqlite as Database>::Connection,
    ) -> Result<Vec<AppliedMigrationSqlRow>, Error> {
        Ok(self.applied_migrations.clone())
    }

    async fn lock(&self, _connection: &mut <Sqlite as Database>::Connection) -> Result<(), Error> {
        Ok(())
    }

    async fn unlock(
        &self,
        _connection: &mut <Sqlite as Database>::Connection,
    ) -> Result<(), Error> {
        Ok(())
    }
}

impl Migrate<Sqlite> for CustomMigrator {}

/// Allow custom migrator to be used as a sync source in sync tests.
impl Synchronize<Sqlite> for CustomMigrator {}

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

/// Runs `generate_migration_plan` with the given `plan` against `migrator`.
async fn apply_plan<'a>(
    migrator: &'a CustomMigrator,
    conn: &mut <Sqlite as Database>::Connection,
    plan: Plan,
) -> Result<Vec<&'a Box<dyn Migration<Sqlite>>>, Error> {
    migrator.generate_migration_plan(conn, Some(&plan)).await
}

/// Adds all `migration_list` entries to `migrator`, then returns the full
/// apply-all plan. This is the most common setup pattern for plan tests.
async fn generate_apply_all_plan(
    migrator: &mut CustomMigrator,
    migration_list: Vec<Box<dyn Migration<Sqlite>>>,
) -> Result<Vec<&Box<dyn Migration<Sqlite>>>, Error> {
    migrator.add_migrations(migration_list)?;
    let mut conn = make_conn().await;
    apply_plan(migrator, &mut conn, Plan::apply_all()).await
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

    let mut migrator = CustomMigrator::default();
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

    let mut migrator = CustomMigrator::default();
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
#[tokio::test]
async fn linear_dependency_chain_applies_in_order() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    struct C;
    migration!(C, "c", vec_box!(B), vec_box!(), vec_box!());
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C))
        .await
        .unwrap();
    assert_eq!(plan_names(&plan), vec!["a", "b", "c"]);
}

/// Generating a plan when no migrations have been registered must return a
/// `PlanError` with a clear message rather than silently producing an empty
/// plan.
#[tokio::test]
async fn apply_all_with_no_migrations_is_error() {
    struct _A;
    migration!(_A, "a", vec_box!(), vec_box!(), vec_box!());
    struct _B;
    migration!(_B, "b", vec_box!(_A), vec_box!(), vec_box!());
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!()).await;
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: no migrations are added to the migration list".to_string())
    );
}

/// Adding two migrations with the same (app, name) but different definitions
/// (e.g. different parents) must return `Error::InconsistentMigration` so that
/// accidental duplicates are caught at registration time.
#[tokio::test]
async fn inconsistent_duplicate_migration_name_is_error() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    struct C;
    migration!(C, "c", vec_box!(), vec_box!(), vec_box!());
    // D has the same name "b" as B but different parents → inconsistent.
    struct D;
    migration!(D, "b", vec_box!(C), vec_box!(), vec_box!());
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D)).await;
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("inconsistent migration found for test - b".to_string())
    );
}

/// A circular parent chain (A depends on B, B depends on A) must fail with a
/// deadlock error — the planner cannot find a valid ordering.
#[tokio::test]
async fn circular_parent_dependency_deadlocks() {
    struct A;
    migration!(A, "a", vec_box!(B), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B)).await;
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: reached deadlock stage during plan generation".to_string())
    );
}

/// A mutual `replaces` relationship (A replaces B and B replaces A) must return
/// a "two migrations replace each other" error. (Previously misnamed as
/// `run_before_interrelated_test`.)
#[tokio::test]
async fn replaces_interrelated_test() {
    // macro arg order: parents, replaces, run_before
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(B), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(), vec_box!(A), vec_box!());
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B)).await;
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: two migrations replace each other".to_string())
    );
}

/// A mutual `run_before` relationship (A must run before B and B must run
/// before A) creates an ordering deadlock. (Previously misnamed as
/// `replace_interrelated_test`.)
#[tokio::test]
async fn run_before_interrelated_test() {
    // macro arg order: parents, replaces, run_before
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!(B));
    struct B;
    migration!(B, "b", vec_box!(), vec_box!(), vec_box!(A));
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B)).await;
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: reached deadlock stage during plan generation".to_string())
    );
}

/// A migration that lists itself as its own parent creates an unresolvable
/// dependency and must fail with a deadlock error.
#[tokio::test]
async fn self_referencing_parent_deadlocks() {
    struct A;
    migration!(A, "a", vec_box!(A), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(B), vec_box!(), vec_box!());
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B)).await;
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: reached deadlock stage during plan generation".to_string())
    );
}

/// A migration that lists itself in `replaces` (replacing itself) creates a
/// circular replacement and must fail. (Previously misnamed as
/// `run_before_depend_on_itself`.)
#[tokio::test]
async fn replaces_depend_on_itself() {
    // macro arg order: parents, replaces, run_before
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(A), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(), vec_box!(B), vec_box!());
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B)).await;
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: two migrations replace each other".to_string())
    );
}

/// A migration that lists itself in `run_before` (must run before itself)
/// creates an ordering deadlock and must fail. (Previously misnamed as
/// `replace_depend_on_itself`.)
#[tokio::test]
async fn run_before_depend_on_itself() {
    // macro arg order: parents, replaces, run_before
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!(A));
    struct B;
    migration!(B, "b", vec_box!(), vec_box!(), vec_box!(B));
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B)).await;
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: reached deadlock stage during plan generation".to_string())
    );
}

/// When C replaces B (which depends on A), and D replaces C, the chain B→C is
/// collapsed: the plan must contain only A and D (B and C are skipped).
#[tokio::test]
async fn replaced_migrations_skipped_in_plan() {
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
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D))
        .await
        .unwrap();
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
#[tokio::test]
async fn apply_name_for_replacer_includes_transitive_replace_parents() {
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

    let mut migrator = CustomMigrator::default();
    migrator.add_migrations(vec_box!(A, B, C, D)).unwrap();
    let mut conn = make_conn().await;

    let plan = apply_plan(
        &migrator,
        &mut conn,
        Plan::apply_name("test", &Some("d".to_string())),
    )
    .await
    .unwrap();
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
#[tokio::test]
async fn revert_name_for_parent_includes_transitive_replace_dependents() {
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

    let mut migrator = CustomMigrator::default();
    migrator.add_migrations(vec_box!(A, B, C, D)).unwrap();
    // A and D are applied (B and C were replaced by D).
    migrator.add_applied_migrations(vec_box!(A, D)).unwrap();
    let mut conn = make_conn().await;

    let plan = apply_plan(
        &migrator,
        &mut conn,
        Plan::revert_name("test", &Some("a".to_string())),
    )
    .await
    .unwrap();
    // D must be reverted before A (D depends on A through the replace chain).
    assert_eq!(plan_names(&plan), vec!["d", "a"]);
}

/// When C declares `run_before(B)` the ordering must be:
/// A (B's parent) → D (C's `run_before` provider) → C (must precede B) → B.
///
/// `run_before` reverses intuition: C runs *before* B even though C is listed
/// after B in the migration list.
#[tokio::test]
async fn run_before_ordering_is_respected() {
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
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D))
        .await
        .unwrap();
    assert_eq!(plan_names(&plan), vec!["a", "d", "c", "b"]);
}

/// Two migrations (C and D) both replacing the same migration (B) is illegal —
/// only one migration may replace a given migration.
#[tokio::test]
async fn migration_replaced_by_multiple_migrations_is_error() {
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
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D)).await;
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: migration test:b replaced multiple times".to_string())
    );
}

/// C replaces B; D has `run_before(B)`.  Since B is replaced by C, the
/// `run_before` relationship transfers: D must run before C.  Plan: A, D, C.
#[tokio::test]
async fn replaced_migration_run_before_transfers_to_replacer() {
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
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D))
        .await
        .unwrap();
    assert_eq!(plan_names(&plan), vec!["a", "d", "c"]);
}

/// Chain: C replaces B (B→A parent), D replaces C, E has `run_before(C)`.
/// Since C is replaced by D, E must run before D. Plan: A, E, D.
#[tokio::test]
async fn chained_replacer_inherits_run_before_constraint() {
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
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D, E))
        .await
        .unwrap();
    assert_eq!(plan_names(&plan), vec!["a", "e", "d"]);
}

/// Similar to `chained_replacer_inherits_run_before_constraint` but E has
/// `run_before(D)` instead of `run_before(C)`.
/// D replaces C which replaces B (B→A). E must precede D. Plan: A, E, D.
#[tokio::test]
async fn run_before_on_final_replacer_is_respected() {
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
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D, E))
        .await
        .unwrap();
    assert_eq!(plan_names(&plan), vec!["a", "e", "d"]);
}

/// D replaces both C and E; E has `run_before(C)`.  Because D absorbs both
/// replaced migrations (C and E), only A and D appear in the final plan.
#[tokio::test]
async fn replacer_absorbing_all_run_before_parties_collapses_plan() {
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
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D, E))
        .await
        .unwrap();
    assert_eq!(plan_names(&plan), vec!["a", "d"]);
}

/// D replaces C and also has `run_before(C)`, which is a self-contradictory
/// circular requirement and must deadlock.
#[tokio::test]
async fn replacer_with_run_before_on_replaced_deadlocks() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    struct C;
    migration!(C, "c", vec_box!(), vec_box!(B), vec_box!());
    // D replaces C AND must run before C — contradiction!
    struct D;
    migration!(D, "d", vec_box!(), vec_box!(C), vec_box!(C));
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D)).await;
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: reached deadlock stage during plan generation".to_string())
    );
}

/// D replaces C and E; E replaces D and has `run_before(C)`. This creates a
/// mutual-replacement cycle between D and E, which must be detected.
#[tokio::test]
async fn mutual_replacers_with_run_before_is_error() {
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
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D, E)).await;
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: two migrations replace each other".to_string())
    );
}

/// C replaces B and has `run_before(D)`; D replaces C. This means C must run
/// before D and D must come after C, but C is replaced by D — deadlock.
#[tokio::test]
async fn replaced_run_before_its_replacer_deadlocks() {
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
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D)).await;
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: reached deadlock stage during plan generation".to_string())
    );
}

/// B depends on A and has `run_before(A)`: B must come *after* A (parent) but
/// *before* A (`run_before`) — an impossible ordering that must deadlock.
#[tokio::test]
async fn parent_and_run_before_same_migration_deadlocks() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    // B depends on A as a parent, but also declares `run_before(A)`.
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!(A));
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B)).await;
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: reached deadlock stage during plan generation".to_string())
    );
}

/// If the database records B as applied but A (B's parent) is not applied, the
/// plan generator must report an integrity error rather than silently producing
/// an incorrect plan.
#[tokio::test]
async fn child_applied_without_parent_is_error() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    let mut migrator = CustomMigrator::default();
    // Mark B as applied without A — simulates a corrupted migration state.
    migrator.add_applied_migrations(vec_box!(B)).unwrap();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B)).await;
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
#[tokio::test]
async fn transitive_replacer_fully_applied_gives_empty_plan() {
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
    let mut migrator = CustomMigrator::default();
    // A and D are already applied — no remaining work.
    migrator.add_applied_migrations(vec_box!(A, D)).unwrap();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(D, C, B, A))
        .await
        .unwrap();
    assert!(plan.is_empty(), "all migrations already applied via squash");
}

/// When parents and replaces are expressed as virtual (tuple) migrations, the
/// planner must resolve them to the concrete registered migrations.
/// C replaces B (B depends on A); D replaces C. Only A and D remain.
#[tokio::test]
async fn virtual_references_in_replace_chain_resolves() {
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
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(B, C, D, A))
        .await
        .unwrap();
    assert_eq!(plan_names(&plan), vec!["a", "d"]);
}

/// A virtual (tuple) migration that is never replaced by a concrete migration
/// must cause a `PlanError` — it has no real implementation.
#[tokio::test]
async fn unreplaced_virtual_migration_is_error() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    let mut migrator = CustomMigrator::default();
    // ("test", "b") is a virtual placeholder with no concrete replacement.
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, ("test", "b"))).await;
    assert_eq!(
        plan.err().map(|e| e.to_string()),
        Some("plan error: virtual migration which is not replaced is present".to_string())
    );
}

/// A virtual placeholder may appear in the migration list multiple times; it is
/// deduplicated automatically. Once the concrete migration B is also
/// registered, the plan must resolve correctly: A then B.
#[tokio::test]
async fn virtual_migration_with_concrete_replacement_resolves() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    let mut migrator = CustomMigrator::default();
    // The virtual ("test", "b") appears twice; B is its concrete replacement.
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, ("test", "b"), B, ("test", "b")))
        .await
        .unwrap();
    assert_eq!(plan_names(&plan), vec!["a", "b"]);
}

/// When B's parent is expressed as a virtual (tuple) reference instead of a
/// direct struct, the dependency must still be resolved correctly — A before B.
#[tokio::test]
async fn virtual_parent_reference_resolves_dependency_order() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    // B references A via a virtual (tuple) parent.
    struct B;
    migration!(B, "b", vec_box!(("test", "a")), vec_box!(), vec_box!());
    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(B, A))
        .await
        .unwrap();
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
#[tokio::test]
async fn apply_plan_diamond_topology_respects_branches() {
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
    let mut migrator = CustomMigrator::default();
    migrator
        .add_migrations(vec_box!(A, B, C, D, E, F, G))
        .unwrap();
    let sqlite = SqlitePool::connect("sqlite::memory:").await.unwrap();
    let mut conn = sqlite.acquire().await.unwrap();

    // Full apply plan: all seven migrations in topological order.
    let full_plan = migrator
        .generate_migration_plan(&mut conn, Some(&Plan::apply_all()))
        .await
        .unwrap();
    assert_eq!(
        plan_names(&full_plan),
        vec!["a", "b", "c", "d", "e", "f", "g"]
    );

    // Targeted apply up to F: only the F-branch (A→B→D→F).
    let plan_till_f = migrator
        .generate_migration_plan(
            &mut conn,
            Some(&Plan::apply_name("test", &Some("f".to_string()))),
        )
        .await
        .unwrap();
    assert_eq!(plan_names(&plan_till_f), vec!["a", "b", "d", "f"]);

    // Targeted apply up to G: only the G-branch (A→B→C→E→G).
    let plan_till_g = migrator
        .generate_migration_plan(
            &mut conn,
            Some(&Plan::apply_name("test", &Some("g".to_string()))),
        )
        .await
        .unwrap();
    assert_eq!(plan_names(&plan_till_g), vec!["a", "b", "c", "e", "g"]);
}

// ── Plan generation tests: revert ────────────────────────────────────────────

/// Using the same diamond topology as
/// `apply_plan_diamond_topology_respects_branches`, verifies:
/// - `revert_all` reverses every migration in reverse dependency order.
/// - `revert_name("test", Some("f"))` reverts only F (leaf with no dependents).
/// - `revert_name("test", Some("b"))` reverts all descendants of B before B.
#[tokio::test]
async fn revert_plan_diamond_topology_reverses_correctly() {
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
    let mut migrator = CustomMigrator::default();
    migrator
        .add_migrations(vec_box!(A, B, C, D, E, F, G))
        .unwrap();
    migrator
        .add_applied_migrations(vec_box!(A, B, C, D, E, F, G))
        .unwrap();
    let sqlite = SqlitePool::connect("sqlite::memory:").await.unwrap();
    let mut conn = sqlite.acquire().await.unwrap();

    // Full revert: reverse of apply order.
    let revert_plan = migrator
        .generate_migration_plan(&mut conn, Some(&Plan::revert_all()))
        .await
        .unwrap();
    assert_eq!(
        plan_names(&revert_plan),
        vec!["g", "f", "e", "d", "c", "b", "a"]
    );

    // Targeted revert of F only (leaf — no dependents to pull in).
    let plan_till_f = migrator
        .generate_migration_plan(
            &mut conn,
            Some(&Plan::revert_name("test", &Some("f".to_string()))),
        )
        .await
        .unwrap();
    assert_eq!(plan_names(&plan_till_f), vec!["f"]);

    // Targeted revert down to B: must revert all B-descendants first.
    let plan_till_b = migrator
        .generate_migration_plan(
            &mut conn,
            Some(&Plan::revert_name("test", &Some("b".to_string()))),
        )
        .await
        .unwrap();
    assert_eq!(plan_names(&plan_till_b), vec!["g", "f", "e", "d", "c", "b"]);
}

/// When no migrations are applied, `revert_all` must produce an empty plan
/// rather than erroring.
#[tokio::test]
async fn revert_all_no_applied_gives_empty_plan() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());

    let mut migrator = CustomMigrator::default();
    migrator.add_migrations(vec_box!(A, B)).unwrap();
    let mut conn = make_conn().await;

    let plan = apply_plan(&migrator, &mut conn, Plan::revert_all())
        .await
        .unwrap();
    assert!(plan.is_empty(), "no applied migrations → empty revert plan");
}

/// `revert_name` with no migration name reverts all applied migrations for the
/// given app in reverse dependency order.
#[tokio::test]
async fn revert_app_name_reverts_all_for_app() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());

    let mut migrator = CustomMigrator::default();
    migrator.add_migrations(vec_box!(A, B)).unwrap();
    migrator.add_applied_migrations(vec_box!(A, B)).unwrap();
    let mut conn = make_conn().await;

    // revert_name with no specific migration → revert entire app
    let plan = apply_plan(&migrator, &mut conn, Plan::revert_name("test", &None))
        .await
        .unwrap();
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
#[tokio::test]
async fn apply_count_limits_to_n_pending() {
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

    let mut migrator = CustomMigrator::default();
    migrator.add_migrations(vec_box!(A, B, C)).unwrap();
    let mut conn = make_conn().await;

    let plan = apply_plan(&migrator, &mut conn, Plan::apply_count(2))
        .await
        .unwrap();
    assert_eq!(plan_names(&plan), vec!["a", "b"]);
}

/// `revert_count(1)` on three applied migrations must revert only the most
/// recently applied one (the last in apply order).
#[tokio::test]
async fn revert_count_reverts_last_n_applied() {
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

    let mut migrator = CustomMigrator::default();
    migrator.add_migrations(vec_box!(A, B, C)).unwrap();
    migrator.add_applied_migrations(vec_box!(A, B, C)).unwrap();
    let mut conn = make_conn().await;

    let plan = apply_plan(&migrator, &mut conn, Plan::revert_count(1))
        .await
        .unwrap();
    // revert_count(1) should revert C only (the last applied)
    assert_eq!(plan_names(&plan), vec!["c"]);
}

/// `apply_count(0)` is invalid and must return an error with a clear message.
#[tokio::test]
async fn apply_count_zero_is_error() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());

    let mut migrator = CustomMigrator::default();
    migrator.add_migrations(vec_box!(A)).unwrap();
    let mut conn = make_conn().await;

    let err = apply_plan(&migrator, &mut conn, Plan::apply_count(0))
        .await
        .err()
        .expect("apply_count(0) must return an error");
    assert_eq!(err.to_string(), "plan error: count must be greater than 0");
}

/// `revert_count(0)` is invalid and must return the same error as
/// `apply_count(0)`.
#[tokio::test]
async fn revert_count_zero_is_error() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());

    let mut migrator = CustomMigrator::default();
    migrator.add_migrations(vec_box!(A)).unwrap();
    migrator.add_applied_migrations(vec_box!(A)).unwrap();
    let mut conn = make_conn().await;

    let err = apply_plan(&migrator, &mut conn, Plan::revert_count(0))
        .await
        .err()
        .expect("revert_count(0) must return an error");
    assert_eq!(err.to_string(), "plan error: count must be greater than 0");
}

/// Requesting more migrations than are pending must fail with a descriptive
/// error that includes the actual number of pending migrations available.
#[tokio::test]
async fn apply_count_exceeds_pending_is_error() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());

    let mut migrator = CustomMigrator::default();
    migrator.add_migrations(vec_box!(A)).unwrap();
    let mut conn = make_conn().await;

    // Only 1 migration pending, but we request 5.
    let err = apply_plan(&migrator, &mut conn, Plan::apply_count(5))
        .await
        .err()
        .expect("apply_count exceeding pending must return an error");
    assert_eq!(
        err.to_string(),
        "plan error: passed count value is larger than migration length: 1"
    );
}

/// Requesting to revert more migrations than are applied must fail with a
/// descriptive error that includes the actual number applied.
#[tokio::test]
async fn revert_count_exceeds_applied_is_error() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());

    let mut migrator = CustomMigrator::default();
    migrator.add_migrations(vec_box!(A)).unwrap();
    migrator.add_applied_migrations(vec_box!(A)).unwrap();
    let mut conn = make_conn().await;

    // Only 1 migration applied, but we request to revert 5.
    let err = apply_plan(&migrator, &mut conn, Plan::revert_count(5))
        .await
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
#[tokio::test]
async fn apply_all_already_applied_gives_empty_plan() {
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

    let mut migrator = CustomMigrator::default();
    migrator.add_migrations(vec_box!(A, B)).unwrap();
    migrator.add_applied_migrations(vec_box!(A, B)).unwrap();
    let mut conn = make_conn().await;

    let plan = apply_plan(&migrator, &mut conn, Plan::apply_all())
        .await
        .unwrap();
    assert!(plan.is_empty(), "all already applied → plan must be empty");
}

/// When `apply_name` targets a specific migration by name the planner pulls in
/// the full set of required predecessors.
///
/// Topology: N depends on M (parent); N has `run_before(T)`.
/// Requesting T must include M and N (because N must precede T and M must
/// precede N).
#[tokio::test]
async fn apply_name_run_before_pulls_in_required_parents() {
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

    let mut migrator = CustomMigrator::default();
    migrator.add_migrations(vec_box!(M, N, T)).unwrap();
    let mut conn = make_conn().await;

    let plan = apply_plan(
        &migrator,
        &mut conn,
        Plan::apply_name("test", &Some("t".to_string())),
    )
    .await
    .unwrap();

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
#[tokio::test]
async fn revert_name_includes_all_dependent_children() {
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

    let mut migrator = CustomMigrator::default();
    migrator.add_migrations(vec_box!(M, N, T)).unwrap();
    migrator.add_applied_migrations(vec_box!(M, N, T)).unwrap();
    let mut conn = make_conn().await;

    let plan = apply_plan(
        &migrator,
        &mut conn,
        Plan::revert_name("test", &Some("m".to_string())),
    )
    .await
    .unwrap();

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
#[tokio::test]
async fn diamond_dependency_resolves() {
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

    let mut migrator = CustomMigrator::default();
    let plan = generate_apply_all_plan(&mut migrator, vec_box!(A, B, C, D))
        .await
        .unwrap();
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
#[tokio::test]
async fn apply_by_app_name_only() {
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

    let mut migrator = CustomMigrator::default();
    migrator.add_migrations(vec_box!(A, B)).unwrap();
    let mut conn = make_conn().await;

    // apply_name with no specific migration name → apply all for that app
    let plan = apply_plan(&migrator, &mut conn, Plan::apply_name("test", &None))
        .await
        .unwrap();
    let names = plan_names(&plan);
    assert!(names.contains(&"a"), "A missing: {names:?}");
    assert!(names.contains(&"b"), "B missing: {names:?}");
}

/// `apply_name` referencing a non-existent app must return a `PlanError`
/// describing that the app does not exist.
#[tokio::test]
async fn apply_name_nonexistent_app_is_error() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());

    let mut migrator = CustomMigrator::default();
    migrator.add_migrations(vec_box!(A)).unwrap();
    let mut conn = make_conn().await;

    let err = apply_plan(
        &migrator,
        &mut conn,
        Plan::apply_name("nonexistent_app", &None),
    )
    .await
    .err()
    .expect("nonexistent app must return an error");
    assert_eq!(
        err.to_string(),
        "plan error: app nonexistent_app doesn't exist"
    );
}

/// `apply_name` referencing an app that exists but a migration that does not
/// must return a `PlanError` naming the missing migration.
#[tokio::test]
async fn apply_name_nonexistent_migration_is_error() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());

    let mut migrator = CustomMigrator::default();
    migrator.add_migrations(vec_box!(A)).unwrap();
    let mut conn = make_conn().await;

    let err = apply_plan(
        &migrator,
        &mut conn,
        Plan::apply_name("test", &Some("no_such_migration".to_string())),
    )
    .await
    .err()
    .expect("nonexistent migration name must return an error");
    assert_eq!(
        err.to_string(),
        "plan error: migration test:no_such_migration doesn't exist for app"
    );
}

/// `revert_name` referencing a non-existent app must return a `PlanError`
/// describing that the app does not exist.
#[tokio::test]
async fn revert_name_nonexistent_app_is_error() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());

    let mut migrator = CustomMigrator::default();
    migrator.add_migrations(vec_box!(A)).unwrap();
    migrator.add_applied_migrations(vec_box!(A)).unwrap();
    let mut conn = make_conn().await;

    let err = apply_plan(
        &migrator,
        &mut conn,
        Plan::revert_name("nonexistent_app", &None),
    )
    .await
    .err()
    .expect("nonexistent app must return an error");
    assert_eq!(
        err.to_string(),
        "plan error: app nonexistent_app doesn't exist"
    );
}

/// Both the squash migration (replacer) and one of its replaced predecessors
/// being applied simultaneously is an inconsistent state. The planner must
/// report a `PlanError` rather than silently producing a broken plan.
#[tokio::test]
async fn replace_and_replacer_both_applied_is_error() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());
    // C replaces B — they must not both appear in applied migrations.
    struct C;
    migration!(C, "c", vec_box!(), vec_box!(B), vec_box!());

    let mut migrator = CustomMigrator::default();
    migrator.add_migrations(vec_box!(A, B, C)).unwrap();
    // Mark both B and C as applied — this is the illegal state.
    migrator.add_applied_migrations(vec_box!(A, B, C)).unwrap();
    let mut conn = make_conn().await;

    let err = apply_plan(&migrator, &mut conn, Plan::apply_all())
        .await
        .err()
        .expect("simultaneously applied replacer and replaced must return an error");
    assert_eq!(
        err.to_string(),
        "plan error: migration test:c and its replaces are applied together"
    );
}

/// `generate_migration_plan_with_rows` is the synchronous core of the planner.
/// It must produce the same result as `generate_migration_plan` when called
/// with an empty `applied_migration_sql_rows` slice and an `apply_all` plan.
#[tokio::test]
async fn generate_plan_with_rows_matches_apply_all() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());

    let mut migrator = CustomMigrator::default();
    migrator.add_migrations(vec_box!(A, B)).unwrap();

    // Call the sync variant directly with no applied rows.
    let plan = migrator
        .generate_migration_plan_with_rows(Some(&Plan::apply_all()), &[])
        .unwrap();
    assert_eq!(plan_names(&plan), vec!["a", "b"]);
}

// ── Sync tests
// ────────────────────────────────────────────────────────────────

/// `Synchronize::sync` must complete without error when the old migrator has
/// migrations that are a subset of those registered in the new migrator.
#[tokio::test]
async fn sync_completes_without_error() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());

    // Old migrator: A is applied.
    let mut old_migrator = CustomMigrator::default();
    old_migrator.add_migration(Box::new(A)).unwrap();
    old_migrator.add_applied_migration(Box::new(A)).unwrap();

    // New migrator knows about both A and B.
    let mut new_migrator = CustomMigrator::default();
    new_migrator.add_migrations(vec_box!(A, B)).unwrap();

    let mut conn = make_conn().await;
    new_migrator
        .sync(&mut conn, &old_migrator)
        .await
        .expect("sync must complete without error");
}

/// `Synchronize::sync` must silently skip migrations from the old migrator that
/// are not present in the new migrator's migration list.
#[tokio::test]
async fn sync_skips_unknown_migrations() {
    struct A;
    migration!(A, "a", vec_box!(), vec_box!(), vec_box!());
    struct B;
    migration!(B, "b", vec_box!(A), vec_box!(), vec_box!());

    // Old migrator: A and B are applied.
    let mut old_migrator = CustomMigrator::default();
    old_migrator.add_migrations(vec_box!(A, B)).unwrap();
    old_migrator.add_applied_migrations(vec_box!(A, B)).unwrap();

    // New migrator only knows about A (not B).
    let mut new_migrator = CustomMigrator::default();
    new_migrator.add_migration(Box::new(A)).unwrap();

    // Sync must not fail even though B is unknown to the new migrator.
    let mut conn = make_conn().await;
    new_migrator
        .sync(&mut conn, &old_migrator)
        .await
        .expect("sync must skip unknown migrations without error");
}
