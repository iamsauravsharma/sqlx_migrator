//! The migrations of a migrator as a graph, which orders them and narrows them
//! down to what a plan asks for.
//!
//! Parents, run before and replaces of every migration are read once when the
//! graph is built, everything after that works on indexes into its nodes.

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

use super::BoxMigration;
use super::plan::{Plan, PlanType};
use crate::error::Error;

/// A migration and how it relates to the others, which are given as indexes
/// into the nodes, that is into the migration list.
struct Node {
    app: String,
    name: String,
    /// Migrations it has to run after
    parents: Vec<usize>,
    /// Migrations which said they run before it, so it runs after them too
    run_after: Vec<usize>,
    /// Migrations it said it runs before
    run_before: Vec<usize>,
    /// The migration replacing it
    replaced_by: Option<usize>,
    /// Migrations it replaces
    replaces: Vec<usize>,
    /// Migrations it replaces and the ones they replace, and so on
    all_replaces: Vec<usize>,
}

impl Node {
    fn new(app: &str, name: &str) -> Self {
        Self {
            app: app.to_string(),
            name: name.to_string(),
            parents: Vec::new(),
            run_after: Vec::new(),
            run_before: Vec::new(),
            replaced_by: None,
            replaces: Vec::new(),
            all_replaces: Vec::new(),
        }
    }
}

/// How the migrations of the migration list relate to each other, one node
/// for each migration in the order of the list
pub(super) struct Relations {
    nodes: Vec<Node>,
}

impl Relations {
    /// Graph of the migrations in the migration list
    ///
    /// # Errors
    /// When a migration refers to one which is not in the list, is replaced
    /// twice or is replaced by a migration it replaces
    pub(super) fn new<DB>(migrations: &[BoxMigration<DB>]) -> Result<Self, Error> {
        let index_of: HashMap<(&str, &str), usize> = migrations
            .iter()
            .enumerate()
            .map(|(index, migration)| ((migration.app(), migration.name()), index))
            .collect();
        let index = |migration: &BoxMigration<DB>| {
            index_of
                .get(&(migration.app(), migration.name()))
                .copied()
                .ok_or_else(|| {
                    Error::PlanError {
                        message: format!(
                            "migration {}:{} is not in the migration list",
                            migration.app(),
                            migration.name()
                        ),
                    }
                })
        };
        let mut nodes = migrations
            .iter()
            .map(|migration| Node::new(migration.app(), migration.name()))
            .collect::<Vec<_>>();
        for (migration_index, migration) in migrations.iter().enumerate() {
            nodes[migration_index].parents = migration
                .parents()
                .iter()
                .map(index)
                .collect::<Result<_, _>>()?;
            for target in migration.run_before() {
                let target = index(&target)?;
                nodes[target].run_after.push(migration_index);
                nodes[migration_index].run_before.push(target);
            }
        }
        // a migration can only be replaced by one migration but one migration
        // can replace many
        for (migration_index, migration) in migrations.iter().enumerate() {
            for child in migration.replaces() {
                let child_index = index(&child)?;
                if nodes[child_index].replaced_by.is_some() {
                    return Err(Error::PlanError {
                        message: format!(
                            "migration {}:{} replaced multiple times",
                            child.app(),
                            child.name()
                        ),
                    });
                }
                nodes[child_index].replaced_by = Some(migration_index);
                nodes[migration_index].replaces.push(child_index);
            }
        }
        let mut relations = Self { nodes };
        for replacer in 0..relations.nodes.len() {
            relations.nodes[replacer].all_replaces = relations.find_all_replaces(replacer)?;
        }
        Ok(relations)
    }

    /// Migrations a migration replaces and the ones they replace
    fn find_all_replaces(&self, replacer: usize) -> Result<Vec<usize>, Error> {
        let mut all = Vec::new();
        let mut seen = HashSet::new();
        let mut stack = self.nodes[replacer].replaces.clone();
        while let Some(migration) = stack.pop() {
            if migration == replacer {
                return Err(Error::PlanError {
                    message: "two migrations replace each other".to_string(),
                });
            }
            if seen.insert(migration) {
                all.push(migration);
                stack.extend(&self.nodes[migration].replaces);
            }
        }
        Ok(all)
    }

    /// The heads of every app: migrations no other migration of the same app
    /// comes after. A migration replaced by another is not a head, and a
    /// migration replacing others comes after what they come after.
    pub(super) fn heads(&self) -> Vec<usize> {
        let mut has_successor = vec![false; self.nodes.len()];
        for (migration, node) in self.nodes.iter().enumerate() {
            if node.replaced_by.is_some() {
                continue;
            }
            let replaced = node.all_replaces.iter().collect::<HashSet<_>>();
            let mut predecessors = node
                .parents
                .iter()
                .chain(&node.run_after)
                .copied()
                .collect::<Vec<_>>();
            for &replaced_migration in &node.all_replaces {
                predecessors.extend(
                    self.nodes[replaced_migration]
                        .parents
                        .iter()
                        .chain(&self.nodes[replaced_migration].run_after)
                        .filter(|predecessor| !replaced.contains(predecessor)),
                );
            }
            for mut current in predecessors {
                loop {
                    if current != migration && self.nodes[current].app == node.app {
                        has_successor[current] = true;
                    }
                    // Follow the chain of replacements to mark all successors
                    // as having a predecessor.
                    match self.nodes[current].replaced_by {
                        Some(successor) => current = successor,
                        None => break,
                    }
                }
            }
        }
        (0..self.nodes.len())
            .filter(|&migration| {
                self.nodes[migration].replaced_by.is_none() && !has_successor[migration]
            })
            .collect()
    }

    /// Order of the migrations in which they can be applied.
    ///
    /// A migration comes after its parents, the migrations which have it in
    /// their run before and the migration replacing it, and a migration
    /// replacing others also after the parents and run before of those.
    /// Of the migrations which are free the first one of the list is added
    /// first.
    ///
    /// # Errors
    /// When the migrations wait for each other
    pub(super) fn order(&self) -> Result<Vec<usize>, Error> {
        let mut waiting_for = vec![0_usize; self.nodes.len()];
        let mut dependents = vec![Vec::new(); self.nodes.len()];
        for migration in 0..self.nodes.len() {
            let node = &self.nodes[migration];
            let mut prerequisites = node
                .parents
                .iter()
                .chain(&node.run_after)
                .copied()
                .collect::<HashSet<_>>();
            prerequisites.extend(node.replaced_by);
            let replaced = node.all_replaces.iter().collect::<HashSet<_>>();
            for &child in &node.all_replaces {
                prerequisites.extend(&self.nodes[child].parents);
                prerequisites.extend(
                    self.nodes[child]
                        .run_after
                        .iter()
                        .filter(|parent| !replaced.contains(parent)),
                );
            }
            waiting_for[migration] = prerequisites.len();
            for before in prerequisites {
                dependents[before].push(migration);
            }
        }

        let mut free = (0..self.nodes.len())
            .filter(|&migration| waiting_for[migration] == 0)
            .collect::<BTreeSet<_>>();
        let mut order = Vec::with_capacity(self.nodes.len());
        while let Some(next) = free.pop_first() {
            order.push(next);
            for &dependent in &dependents[next] {
                waiting_for[dependent] -= 1;
                if waiting_for[dependent] == 0 {
                    free.insert(dependent);
                }
            }
        }
        // migrations which are left wait for each other
        if order.len() != self.nodes.len() {
            return Err(Error::PlanError {
                message: "reached deadlock stage during plan generation".to_string(),
            });
        }
        Ok(order)
    }

    /// Narrows the order of all migrations down to what `plan` asks for,
    /// given which migrations are applied
    ///
    /// # Errors
    /// When the applied migrations do not make sense together or the plan
    /// names an app, a migration or a count which does not exist
    pub(super) fn plan(
        &self,
        mut order: Vec<usize>,
        applied: &[bool],
        plan: &Plan,
        last_batch_keys: &HashSet<(&str, &str)>,
    ) -> Result<Vec<usize>, Error> {
        self.check_applied(applied)?;
        self.remove_replaced(&mut order, applied)?;
        self.process(&mut order, applied, plan, last_batch_keys)?;
        Ok(order)
    }

    /// Checks that no applied migration comes before one of its parents, the
    /// parent being applied itself or through a migration replacing it
    fn check_applied(&self, applied: &[bool]) -> Result<(), Error> {
        for migration in (0..self.nodes.len()).filter(|&migration| applied[migration]) {
            let parents = self.nodes[migration]
                .run_after
                .iter()
                .chain(&self.nodes[migration].parents);
            for &parent in parents {
                // the parent itself or a migration replacing it
                let satisfied = std::iter::successors(Some(parent), |&candidate| {
                    self.nodes[candidate].replaced_by
                })
                .any(|candidate| applied[candidate]);
                if !satisfied {
                    return Err(Error::PlanError {
                        message: format!(
                            "child migration {}:{} applied before its parent migration {}:{}",
                            self.nodes[migration].app,
                            self.nodes[migration].name,
                            self.nodes[parent].app,
                            self.nodes[parent].name
                        ),
                    });
                }
            }
        }
        Ok(())
    }

    /// Removes the migrations which are replaced and the ones replacing a
    /// migration which is applied, since one of them stands in for the other
    fn remove_replaced(&self, order: &mut Vec<usize>, applied: &[bool]) -> Result<(), Error> {
        let mut remove = vec![false; self.nodes.len()];
        for &migration in order.iter() {
            let replaced = &self.nodes[migration].all_replaces;
            if replaced.iter().any(|&child| applied[child]) {
                // both cannot be applied together
                if applied[migration] {
                    return Err(Error::PlanError {
                        message: format!(
                            "migration {}:{} and its replaces are applied together",
                            self.nodes[migration].app, self.nodes[migration].name,
                        ),
                    });
                }
                remove[migration] = true;
            } else {
                // this migration applies in place of them
                for &child in replaced {
                    remove[child] = true;
                }
            }
        }
        order.retain(|&migration| !remove[migration]);
        Ok(())
    }

    /// Keeps the migrations of `order` the plan asks for
    fn process(
        &self,
        order: &mut Vec<usize>,
        applied: &[bool],
        plan: &Plan,
        last_batch_keys: &HashSet<(&str, &str)>,
    ) -> Result<(), Error> {
        match &plan.plan_type {
            PlanType::Apply => order.retain(|&migration| !applied[migration]),
            PlanType::Revert(revert_plan) => {
                order.retain(|&migration| applied[migration]);
                // If the revert plan targets the last batch, filter out
                // migrations not in the last batch.
                if revert_plan.last_batch {
                    order.retain(|&migration| {
                        last_batch_keys
                            .contains(&(&self.nodes[migration].app, &self.nodes[migration].name))
                    });
                }
                order.reverse();
            }
        }

        if let Some((app, migration_name)) = &plan.app_migration {
            // A target that exists in the full migration list but not in the
            // processed list has nothing to do (it is already applied/reverted
            // or replaced), so the plan is empty rather than an error. This
            // keeps targeted apply/revert idempotent. Only a target that
            // doesn't exist at all is an error.
            let is_target = |migration: usize| {
                self.nodes[migration].app == *app
                    && migration_name
                        .as_ref()
                        .is_none_or(|name| self.nodes[migration].name == *name)
            };
            let Some(position) = order.iter().rposition(|&migration| is_target(migration)) else {
                if !(0..self.nodes.len()).any(|migration| self.nodes[migration].app == *app) {
                    return Err(Error::PlanError {
                        message: format!("app {app} doesn't exist"),
                    });
                }
                if let Some(name) = migration_name
                    && !(0..self.nodes.len()).any(is_target)
                {
                    return Err(Error::PlanError {
                        message: format!("migration {name} doesn't exist for app {app}"),
                    });
                }
                order.clear();
                return Ok(());
            };
            order.truncate(position + 1);
            let targets = order
                .iter()
                .copied()
                .filter(|&migration| is_target(migration))
                .collect::<Vec<_>>();
            self.only_related(order, &targets, &plan.plan_type);
        } else if let Some(count) = plan.count {
            if count == 0 {
                return Err(Error::PlanError {
                    message: "count must be greater than 0".to_string(),
                });
            }
            let actual_len = order.len();
            if count > actual_len {
                return Err(Error::PlanError {
                    message: format!(
                        "passed count value is larger than migration length: {actual_len}"
                    ),
                });
            }
            order.truncate(count);
        }
        Ok(())
    }

    /// Narrows `order` down to the migrations required to apply or revert the
    /// `targets`, which are the ones they depend on when applying and the ones
    /// depending on them when reverting. A migration replacing others also
    /// takes over the relations of those.
    fn only_related(&self, order: &mut Vec<usize>, targets: &[usize], plan_type: &PlanType) {
        let mut in_order = vec![false; self.nodes.len()];
        for &migration in order.iter() {
            in_order[migration] = true;
        }
        let neighbours = match plan_type {
            PlanType::Apply => self.dependencies(),
            PlanType::Revert(_) => self.dependents(),
        };
        let mut related = vec![false; self.nodes.len()];
        let mut queue = targets.iter().copied().collect::<VecDeque<_>>();
        for &target in targets {
            related[target] = true;
        }
        while let Some(current) = queue.pop_front() {
            for &migration in &neighbours[current] {
                if in_order[migration] && !related[migration] {
                    related[migration] = true;
                    queue.push_back(migration);
                }
            }
        }
        order.retain(|&migration| related[migration]);
    }

    /// For every migration, the migrations which have to come before it:
    /// its parents and the migrations having it in their run before, and the
    /// same of the migrations it replaces
    fn dependencies(&self) -> Vec<Vec<usize>> {
        (0..self.nodes.len())
            .map(|migration| {
                std::iter::once(migration)
                    .chain(self.nodes[migration].all_replaces.iter().copied())
                    .flat_map(|source| {
                        self.nodes[source]
                            .parents
                            .iter()
                            .chain(&self.nodes[source].run_after)
                            .copied()
                    })
                    .collect()
            })
            .collect()
    }

    /// For every migration, the migrations which have to come after it: the
    /// ones with it as a parent, directly or through a migration they replace,
    /// and the ones with a migration it has to come before, or one that
    /// migration replaces, in their run before... read the other way round
    fn dependents(&self) -> Vec<Vec<usize>> {
        let mut dependents = vec![Vec::new(); self.nodes.len()];
        // migrations named by the run before of others, and the migrations
        // replacing them, which are all reverted after the others
        let mut replacing = vec![Vec::new(); self.nodes.len()];
        for (migration, node) in self.nodes.iter().enumerate() {
            for source in std::iter::once(migration).chain(node.all_replaces.iter().copied()) {
                for &parent in &self.nodes[source].parents {
                    dependents[parent].push(migration);
                }
                replacing[source].push(migration);
            }
        }
        for (migration, node) in self.nodes.iter().enumerate() {
            for &target in &node.run_before {
                dependents[migration].extend(&replacing[target]);
            }
        }
        dependents
    }
}
