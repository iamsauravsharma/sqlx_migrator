use super::*;

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
