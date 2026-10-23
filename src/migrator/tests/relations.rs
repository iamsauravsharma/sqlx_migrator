use super::*;

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
