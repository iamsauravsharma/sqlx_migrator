use super::*;

// ── Plan order ───────────────────────────────────────────────────────────────

/// Migration of the `main` app with parents and run before migrations given by
/// name.
struct Node {
    name: String,
    parents: Vec<String>,
    run_before: Vec<String>,
    replaces: Vec<String>,
}

impl Node {
    fn new(name: usize, parents: &[usize], run_before: &[usize]) -> Self {
        let names = |indexes: &[usize]| indexes.iter().map(|index| format!("m{index}")).collect();
        Self {
            name: format!("m{name}"),
            parents: names(parents),
            run_before: names(run_before),
            replaces: Vec::new(),
        }
    }

    fn replacing(mut self, replaces: &[usize]) -> Self {
        self.replaces = replaces.iter().map(|index| format!("m{index}")).collect();
        self
    }
}

fn nodes_to_migrations(nodes: Vec<Node>) -> Vec<Box<dyn Migration<Sqlite>>> {
    nodes
        .into_iter()
        .map(|node| Box::new(node) as Box<dyn Migration<Sqlite>>)
        .collect()
}

impl Migration<Sqlite> for Node {
    fn app(&self) -> &'static str {
        "main"
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn parents(&self) -> Vec<Box<dyn Migration<Sqlite>>> {
        self.parents
            .iter()
            .map(|parent| Box::new(("main", parent.clone())) as Box<dyn Migration<Sqlite>>)
            .collect()
    }

    fn operations(&self) -> Vec<Box<dyn Operation<Sqlite>>> {
        vec![]
    }

    fn replaces(&self) -> Vec<Box<dyn Migration<Sqlite>>> {
        self.replaces
            .iter()
            .map(|migration| Box::new(("main", migration.clone())) as Box<dyn Migration<Sqlite>>)
            .collect()
    }

    fn run_before(&self) -> Vec<Box<dyn Migration<Sqlite>>> {
        self.run_before
            .iter()
            .map(|migration| Box::new(("main", migration.clone())) as Box<dyn Migration<Sqlite>>)
            .collect()
    }
}

/// A long chain added from the last migration to the first is planned quickly,
/// plans which look for the related migrations included.
#[test]
fn long_chain_is_planned_quickly() {
    let size = 3000;
    let mut migrator = Migrator::<Sqlite>::default();
    migrator
        .add_migrations(nodes_to_migrations(
            (0..size)
                .rev()
                .map(|name| {
                    Node::new(
                        name,
                        &name.checked_sub(1).into_iter().collect::<Vec<_>>(),
                        &[],
                    )
                })
                .collect(),
        ))
        .unwrap();
    let rows = (0..size)
        .map(|name| {
            AppliedMigrationSqlRow::new(i32::try_from(name).unwrap(), "main", &format!("m{name}"))
        })
        .collect::<Vec<_>>();
    let last = format!("m{}", size - 1);
    let started = std::time::Instant::now();
    let plan = migrator.generate_migration_plan(None, &[]).unwrap();
    assert_eq!(plan.len(), size);
    assert_eq!(plan[0].name(), "m0");
    assert_eq!(plan[size - 1].name(), last);
    let apply = migrator
        .generate_migration_plan(Some(&Plan::apply_name("main", Some(&last))), &[])
        .unwrap();
    assert_eq!(apply.len(), size);
    let revert = migrator
        .generate_migration_plan(Some(&Plan::revert_name("main", Some("m0"))), &rows)
        .unwrap();
    assert_eq!(revert.len(), size);
    assert_eq!(revert[0].name(), last);
    assert!(started.elapsed() < std::time::Duration::from_secs(2));
}

/// Plan of the migrations `nodes` for `plan`, as names or the error message,
/// when the migrations with the indexes in `applied` are applied.
fn plan_of(
    nodes: Vec<Node>,
    plan: Option<&Plan>,
    applied: &[usize],
) -> Result<Vec<String>, String> {
    let mut migrator = Migrator::<Sqlite>::default();
    migrator.add_migrations(nodes_to_migrations(nodes)).unwrap();
    let rows = applied
        .iter()
        .map(|&index| {
            AppliedMigrationSqlRow::new(i32::try_from(index).unwrap(), "main", &format!("m{index}"))
        })
        .collect::<Vec<_>>();
    migrator
        .generate_migration_plan(plan, &rows)
        .map(|plan| {
            plan.iter()
                .map(|migration| migration.name().to_string())
                .collect()
        })
        .map_err(|error| error.to_string())
}

/// Of the migrations which are free the first one of the list is added first,
/// so one which was waiting for a later migration comes in as soon as that is
/// added.
#[test]
fn first_free_migration_of_the_list_is_added_first() {
    // m0 waits for m2
    let nodes = vec![
        Node::new(0, &[2], &[]),
        Node::new(1, &[], &[]),
        Node::new(2, &[], &[]),
        Node::new(3, &[], &[]),
    ];
    assert_eq!(plan_of(nodes, None, &[]).unwrap(), ["m1", "m2", "m0", "m3"]);
    // m1 waits for m0 which waits for m2
    let nodes = vec![
        Node::new(0, &[2], &[]),
        Node::new(1, &[0], &[]),
        Node::new(2, &[], &[]),
        Node::new(3, &[], &[]),
    ];
    assert_eq!(plan_of(nodes, None, &[]).unwrap(), ["m2", "m0", "m1", "m3"]);
}

/// A migration which has to come before another one is added first, and a
/// missing order is a deadlock.
#[test]
fn run_before_and_parents_set_the_order_and_cycles_deadlock() {
    // m0 is in the list first but has to come after m1 through m1's run before
    let nodes = vec![Node::new(0, &[], &[]), Node::new(1, &[], &[0])];
    assert_eq!(plan_of(nodes, None, &[]).unwrap(), ["m1", "m0"]);
    let nodes = vec![
        Node::new(0, &[2], &[]),
        Node::new(1, &[0], &[]),
        Node::new(2, &[1], &[]),
    ];
    assert_eq!(
        plan_of(nodes, None, &[]).unwrap_err(),
        "plan error: reached deadlock stage during plan generation"
    );
}

/// An applied migration is fine when the parent it needs is not applied itself
/// but replaced by an applied migration.
#[test]
fn applied_child_of_a_parent_replaced_by_an_applied_migration_is_fine() {
    // m2 replaces m0 and m1 has m0 as parent
    let nodes = || {
        vec![
            Node::new(0, &[], &[]),
            Node::new(1, &[0], &[]),
            Node::new(2, &[], &[]).replacing(&[0]),
        ]
    };
    assert_eq!(
        plan_of(nodes(), Some(&Plan::apply_all()), &[1, 2]).unwrap(),
        Vec::<String>::new()
    );
    assert_eq!(
        plan_of(nodes(), Some(&Plan::apply_all()), &[1]).unwrap_err(),
        "plan error: child migration main:m1 applied before its parent migration main:m0"
    );
}

/// Applying one migration brings along the migrations which have to come before
/// a migration it replaces, and leaves out the unrelated ones.
#[test]
fn apply_name_takes_over_the_run_before_of_replaced_migrations() {
    // m0 is unrelated, m1 has to come before m2 which m3 replaces
    let nodes = vec![
        Node::new(0, &[], &[]),
        Node::new(1, &[], &[2]),
        Node::new(2, &[], &[]),
        Node::new(3, &[], &[]).replacing(&[2]),
    ];
    assert_eq!(
        plan_of(nodes, Some(&Plan::apply_name("main", Some("m3"))), &[]).unwrap(),
        ["m1", "m3"]
    );
}

/// Reverting one migration also reverts the migration replacing one it has to
/// come before.
#[test]
fn revert_name_reaches_the_replacer_of_a_run_before_target() {
    // m1 has to come before m2 which m3 replaces. m0, m1 and m3 are applied
    let nodes = vec![
        Node::new(0, &[], &[]),
        Node::new(1, &[], &[2]),
        Node::new(2, &[], &[]),
        Node::new(3, &[], &[]).replacing(&[2]),
    ];
    assert_eq!(
        plan_of(
            nodes,
            Some(&Plan::revert_name("main", Some("m1"))),
            &[0, 1, 3]
        )
        .unwrap(),
        ["m3", "m1"]
    );
}

/// A migration which is replaced has nothing to apply, a replace cycle is an
/// error whatever its length.
#[test]
fn replaced_migration_has_nothing_to_apply_and_replace_cycles_are_errors() {
    let nodes = vec![
        Node::new(0, &[], &[]),
        Node::new(1, &[], &[]).replacing(&[0]),
    ];
    assert_eq!(
        plan_of(nodes, Some(&Plan::apply_name("main", Some("m0"))), &[]).unwrap(),
        Vec::<String>::new()
    );
    let nodes = vec![
        Node::new(0, &[], &[]).replacing(&[1]),
        Node::new(1, &[], &[]).replacing(&[2]),
        Node::new(2, &[], &[]).replacing(&[0]),
    ];
    assert_eq!(
        plan_of(nodes, None, &[]).unwrap_err(),
        "plan error: two migrations replace each other"
    );
}

// ── Heads ────────────────────────────────────────────────────────────────────

fn heads_of(nodes: Vec<Node>) -> Vec<String> {
    let mut migrator = Migrator::<Sqlite>::default();
    migrator.add_migrations(nodes_to_migrations(nodes)).unwrap();
    migrator
        .heads()
        .unwrap()
        .iter()
        .map(|head| head.name().to_string())
        .collect()
}

/// A migration is a head when no other migration of the app comes after it.
#[test]
fn heads_are_migrations_nothing_comes_after() {
    // a chain has one head
    let nodes = vec![
        Node::new(0, &[], &[]),
        Node::new(1, &[0], &[]),
        Node::new(2, &[1], &[]),
    ];
    assert_eq!(heads_of(nodes), ["m2"]);
    // the order of the list does not matter
    let nodes = vec![
        Node::new(2, &[1], &[]),
        Node::new(1, &[0], &[]),
        Node::new(0, &[], &[]),
    ];
    assert_eq!(heads_of(nodes), ["m2"]);
    // two branches have two heads, a merge joins them
    let branches = || {
        vec![
            Node::new(0, &[], &[]),
            Node::new(1, &[0], &[]),
            Node::new(2, &[0], &[]),
        ]
    };
    assert_eq!(heads_of(branches()), ["m1", "m2"]);
    let mut merged = branches();
    merged.push(Node::new(3, &[1, 2], &[]));
    assert_eq!(heads_of(merged), ["m3"]);
    // run before puts the target after the migration
    let nodes = vec![Node::new(0, &[], &[1]), Node::new(1, &[], &[])];
    assert_eq!(heads_of(nodes), ["m1"]);
}

/// A replaced migration is never a head, and its replacer comes after what the
/// replaced migration and its children come after.
#[test]
fn heads_follow_replaced_migrations() {
    // m1 replaces m0
    let nodes = vec![
        Node::new(0, &[], &[]),
        Node::new(1, &[], &[]).replacing(&[0]),
    ];
    assert_eq!(heads_of(nodes), ["m1"]);
    // m2 comes after m0 which m1 replaces, so m2 is the head and m1 is not
    let nodes = vec![
        Node::new(0, &[], &[]),
        Node::new(1, &[], &[]).replacing(&[0]),
        Node::new(2, &[0], &[]),
    ];
    assert_eq!(heads_of(nodes), ["m2"]);
    // m2 replaces m1 and takes over its parent m0
    let nodes = vec![
        Node::new(0, &[], &[]),
        Node::new(1, &[0], &[]),
        Node::new(2, &[], &[]).replacing(&[1]),
    ];
    assert_eq!(heads_of(nodes), ["m2"]);
}

/// More than one head in an app fails, other apps do not matter.
#[test]
fn multiple_heads_in_an_app_fail() {
    struct Other;
    migration!(Other, "other", vec_box!(), vec_box!(), vec_box!());
    let mut migrator = migrator_of(vec![
        SqlMigration::new("first", None, "SELECT 1"),
        SqlMigration::new("second", Some("first"), "SELECT 1"),
    ]);
    migrator.check_heads().unwrap();
    // a migration of another app is a head of its own
    migrator.add_migration(Box::new(Other)).unwrap();
    migrator.check_heads().unwrap();
    assert_eq!(migrator.heads().unwrap().len(), 2);

    migrator
        .add_migration(Box::new(SqlMigration::new(
            "third",
            Some("first"),
            "SELECT 1",
        )))
        .unwrap();
    let error = migrator.check_heads().unwrap_err();
    assert!(matches!(&error, Error::MultipleHeads { app, heads }
        if app == "main" && heads == &["second", "third"]));
    assert_eq!(
        error.to_string(),
        "app main has multiple heads: second, third"
    );
}

/// Running and checking pending migrations fail before doing anything when an
/// app has more than one head.
#[tokio::test]
async fn run_and_check_pending_fail_with_multiple_heads() {
    let migrator = migrator_of(vec![
        SqlMigration::new("first", None, "CREATE TABLE first (id INTEGER)"),
        SqlMigration::new("second", None, "CREATE TABLE second (id INTEGER)"),
    ]);
    let mut conn = make_conn().await;
    assert!(matches!(
        migrator.run(&mut conn, &Plan::apply_all()).await,
        Err(Error::MultipleHeads { .. })
    ));
    assert!(matches!(
        migrator.check_pending(&mut conn).await,
        Err(Error::MultipleHeads { .. })
    ));
    assert!(!sqlite_table_exists(&mut conn, "first").await);
    // planning alone does not check, so branches can be previewed
    assert!(
        migrator
            .generate_migration_plan(Some(&Plan::apply_all()), &[])
            .is_ok()
    );
}
