use std::collections::HashMap;

use petgraph::graph::NodeIndex;
use petgraph::visit::Dfs;
use petgraph::Graph;

use crate::migration::Migration;

/// Struct to store different migration
pub struct Migrator<'a, D>
where
    D: sqlx::Database,
{
    graph: Graph<&'a Migration<'a, D>, ()>,
    migration_map: HashMap<&'a Migration<'a, D>, NodeIndex>,
}

impl<'a, D> Migrator<'a, D>
where
    D: sqlx::Database,
{
    /// Create new migrator
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
            migration_map: HashMap::new(),
        }
    }

    /// Add migration to migrator
    pub fn add_migration(&mut self, migration: &'a Migration<'a, D>) {
        self.migration_map
            .entry(migration)
            .or_insert_with(|| self.graph.add_node(migration));
        for parent in migration.parents() {
            self.add_migration(parent);
            let migration_node_index = self
                .migration_map
                .get(migration)
                .expect("Failed to get migration node index");
            let parent_node_index = self
                .migration_map
                .get(parent)
                .expect("Failed to get parent node index");
            self.graph
                .add_edge(*parent_node_index, *migration_node_index, ());
        }
    }

    // Create full migration plan
    fn _create_full_migration_plan(&self) -> Vec<&Migration<'a, D>> {
        let mut plan = vec![];
        let mut visited_node = vec![];
        while visited_node.len() < self.graph.node_indices().len() {
            for node_index in self.graph.node_indices() {
                let mut dfs = Dfs::new(&self.graph, node_index);
                while let Some(nx) = dfs.next(&self.graph) {
                    let migration = self.graph[nx];
                    let all_parents_visited = migration
                        .parents()
                        .iter()
                        .all(|x| visited_node.contains(&x));
                    if all_parents_visited {
                        plan.push(migration);
                        visited_node.push(migration);
                    }
                }
            }
        }
        plan
    }
}
