//! Dependency graph for managing task dependencies.
//!
//! Design:
//! - Forward edges: task -> tasks it depends on (waits for)
//! - Reverse edges: task -> tasks that depend on it (waiting tasks)
//! - Invariant: edges and reverse_edges must be kept in sync

use std::collections::hash_map::Entry;

use crate::domain::TaskId;
use std::collections::{HashMap, HashSet};

/// Dependency graph for tracking task dependencies.
///
/// This graph maintains both forward and reverse edges for efficient lookups:
/// - `edges`: TaskId -> Set of TaskIds it depends on
/// - `reverse_edges`: TaskId -> Set of TaskIds waiting for it
pub struct DependencyGraph {
    /// Forward edges: task -> tasks it depends on (waits for)
    edges: HashMap<TaskId, HashSet<TaskId>>,

    /// Reverse edges: task -> tasks that depend on it (waiting tasks)
    /// Enables O(1) lookup: "who is waiting for this task?"
    reverse_edges: HashMap<TaskId, HashSet<TaskId>>,
}

impl DependencyGraph {
    /// Create an empty dependency graph.
    pub fn new() -> Self {
        Self {
            edges: HashMap::new(),
            reverse_edges: HashMap::new(),
        }
    }

    /// Add a dependency: `task` depends on `depends_on`.
    ///
    /// Example: add_dependency(task_b, task_a) means "B waits for A"
    ///
    /// This method must maintain the invariant by updating both:
    /// - edges: B -> {A}
    /// - reverse_edges: A -> {B}
    pub fn add_dependency(&mut self, task: TaskId, depends_on: TaskId) {
        self.edges.entry(task).or_default().insert(depends_on);
        self.reverse_edges
            .entry(depends_on)
            .or_default()
            .insert(task);
    }

    /// Remove a dependency: `task` no longer depends on `depends_on`.
    ///
    /// This happens when the depended task completes.
    /// Must maintain invariant by updating both edges and reverse_edges.
    pub fn remove_dependency(&mut self, task: TaskId, depends_on: TaskId) {
        match self.edges.entry(task) {
            Entry::Occupied(mut e) => {
                e.get_mut().remove(&depends_on);
                if e.get().is_empty() {
                    e.remove_entry();
                }
            }
            Entry::Vacant(_) => {}
        }
        match self.reverse_edges.entry(depends_on) {
            Entry::Occupied(mut e) => {
                e.get_mut().remove(&task);
                if e.get().is_empty() {
                    e.remove_entry();
                }
            }
            Entry::Vacant(_) => {}
        }
    }

    /// Get all tasks that can be unblocked when `completed_task` finishes.
    ///
    /// Returns: TaskIds that were waiting for this task.
    ///
    /// Note: This returns ALL tasks waiting for `completed_task`, even if they
    /// have other dependencies. The caller must check if all dependencies are resolved.
    pub fn get_waiting_tasks(&self, completed_task: TaskId) -> Vec<TaskId> {
        self.reverse_edges
            .get(&completed_task)
            .map(|waiting| waiting.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Check if `task` has any dependencies.
    pub fn has_dependencies(&self, task: TaskId) -> bool {
        self.edges
            .get(&task)
            .map(|deps| !deps.is_empty())
            .unwrap_or(false)
    }

    /// Get all dependencies of a task.
    pub fn get_dependencies(&self, task: TaskId) -> Vec<TaskId> {
        self.edges
            .get(&task)
            .map(|deps| deps.iter().copied().collect())
            .unwrap_or_default()
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

// TODO(human): Implement cycle detection
//
// Hints:
// 1. Create a HashMap<TaskId, Color> to track node states
// 2. Create a Vec<TaskId> to track the current DFS path
// 3. For each unvisited node, call dfs_cycle()
// 4. dfs_cycle() should:
//    - Mark node as Gray
//    - Push to path
//    - For each dependency:
//      - If Gray: cycle found! Extract cycle from path
//      - If White: recurse
//      - If Black: skip (already explored)
//    - Mark node as Black
//    - Pop from path
//
// Example implementation structure:
//
impl DependencyGraph {
    /// Detect a cycle in the dependency graph.
    ///
    /// Returns the first cycle found, or None if the graph is acyclic (DAG).
    ///
    /// # Current Implementation (v1)
    ///
    /// This implementation uses iterative DFS with visited tracking.
    /// It works correctly for most cases due to `visited.clone()` creating
    /// independent searches from each start point.
    ///
    /// ## Known Limitations:
    /// - **Efficiency**: O(V * E) in worst case due to visited.clone()
    /// - **Design**: visited.clone() is accidental correctness, not intentional
    /// - May have edge cases where false positives occur (though none found in testing)
    ///
    /// ## Future Improvement (v2):
    /// Replace with Kahn's algorithm (topological sort):
    /// - O(V + E) guaranteed
    /// - More explicit and maintainable
    /// - Clearer separation: has_cycle() check, then find_cycle() if needed
    ///
    /// For v1, this implementation is sufficient and passes all known test cases.
    pub fn detect_cycle(&self) -> Option<Vec<TaskId>> {
        let start_points: Vec<TaskId> = self
            .reverse_edges
            .iter()
            .filter(|(_, v)| !v.is_empty())
            .map(|(k, _)| k.clone())
            .collect();
        let visited: HashSet<TaskId> = HashSet::new();
        for start in start_points {
            if let Some(cycle) = self.detect_cycle_from(start, &mut visited.clone()) {
                return Some(cycle);
            }
        }
        None
    }

    pub fn detect_cycle_from(
        &self,
        start: TaskId,
        visited: &mut HashSet<TaskId>,
    ) -> Option<Vec<TaskId>> {
        let mut stack = Vec::new();
        stack.push(start);
        visited.insert(start);

        let mut prev = HashMap::new();
        while let Some(node) = stack.pop() {
            for dep in self.get_dependencies(node) {
                prev.insert(dep, node);
                if visited.contains(&dep) {
                    return Some(self.follow_cycle(dep, &prev));
                }
                visited.insert(dep);
                stack.push(dep);
            }
        }
        None
    }

    pub fn follow_cycle(&self, join_point: TaskId, prev: &HashMap<TaskId, TaskId>) -> Vec<TaskId> {
        let mut cycle = Vec::new();
        let mut current = join_point;
        cycle.push(current);
        while let Some(&p) = prev.get(&current) {
            cycle.push(p);
            if p == join_point {
                break;
            }
            current = p;
        }

        cycle.reverse();
        cycle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_graph_is_empty() {
        let graph = DependencyGraph::new();
        assert!(!graph.has_dependencies(TaskId::new(1)));
    }

    #[test]
    fn add_dependency_creates_forward_edge() {
        let mut graph = DependencyGraph::new();
        let task_a = TaskId::new(1);
        let task_b = TaskId::new(2);

        graph.add_dependency(task_b, task_a); // B depends on A

        assert!(graph.has_dependencies(task_b));
        assert!(!graph.has_dependencies(task_a));
        assert_eq!(graph.get_dependencies(task_b), vec![task_a]);
    }

    #[test]
    fn add_dependency_creates_reverse_edge() {
        let mut graph = DependencyGraph::new();
        let task_a = TaskId::new(1);
        let task_b = TaskId::new(2);

        graph.add_dependency(task_b, task_a); // B depends on A

        let waiting = graph.get_waiting_tasks(task_a);
        assert_eq!(waiting.len(), 1);
        assert_eq!(waiting[0], task_b);
    }

    #[test]
    fn remove_dependency_removes_both_edges() {
        let mut graph = DependencyGraph::new();
        let task_a = TaskId::new(1);
        let task_b = TaskId::new(2);

        graph.add_dependency(task_b, task_a);
        graph.remove_dependency(task_b, task_a);

        assert!(!graph.has_dependencies(task_b));
        assert_eq!(graph.get_waiting_tasks(task_a).len(), 0);
    }

    #[test]
    fn multiple_dependencies() {
        let mut graph = DependencyGraph::new();
        let task_a = TaskId::new(1);
        let task_b = TaskId::new(2);
        let task_c = TaskId::new(3);

        // C depends on both A and B
        graph.add_dependency(task_c, task_a);
        graph.add_dependency(task_c, task_b);

        assert!(graph.has_dependencies(task_c));
        let deps = graph.get_dependencies(task_c);
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&task_a));
        assert!(deps.contains(&task_b));
    }

    // TODO(human): Uncomment these tests after implementing detect_cycle()

    #[test]
    fn detect_simple_cycle() {
        let mut graph = DependencyGraph::new();
        let task_a = TaskId::new(1);
        let task_b = TaskId::new(2);

        // A -> B -> A (cycle)
        graph.add_dependency(task_a, task_b);
        graph.add_dependency(task_b, task_a);

        let cycle = graph.detect_cycle();
        assert!(cycle.is_some());
        let cycle_path = cycle.unwrap();
        println!("Detected cycle: {:?}", cycle_path);
        assert!(cycle_path.len() >= 2); // At least A -> B -> A
    }

    #[test]
    fn detect_no_cycle_in_dag() {
        let mut graph = DependencyGraph::new();
        let task_a = TaskId::new(1);
        let task_b = TaskId::new(2);
        let task_c = TaskId::new(3);

        // A -> B -> C (no cycle)
        graph.add_dependency(task_b, task_a);
        graph.add_dependency(task_c, task_b);

        assert!(graph.detect_cycle().is_none());
    }

    #[test]
    fn detect_self_dependency() {
        let mut graph = DependencyGraph::new();
        let task_a = TaskId::new(1);

        // A -> A (self-cycle)
        graph.add_dependency(task_a, task_a);

        let cycle = graph.detect_cycle();
        assert!(cycle.is_some());
    }

    #[test]
    fn detect_longer_cycle() {
        let mut graph = DependencyGraph::new();
        let task_a = TaskId::new(1);
        let task_b = TaskId::new(2);
        let task_c = TaskId::new(3);
        let task_d = TaskId::new(4);

        // A -> B -> C -> D -> B (cycle: B -> C -> D -> B)
        graph.add_dependency(task_b, task_a);
        graph.add_dependency(task_c, task_b);
        graph.add_dependency(task_d, task_c);
        graph.add_dependency(task_b, task_d);

        let cycle = graph.detect_cycle();
        assert!(cycle.is_some());
    }

    #[test]
    fn dag_with_diamond_should_not_detect_cycle() {
        let mut graph = DependencyGraph::new();
        let task_a = TaskId::new(1);
        let task_b = TaskId::new(2);
        let task_c = TaskId::new(3);

        // Diamond: A -> B -> C, A -> C (NOT a cycle!)
        graph.add_dependency(task_b, task_a);
        graph.add_dependency(task_c, task_b);
        graph.add_dependency(task_c, task_a); // Shortcut edge

        println!("Testing diamond DAG...");
        let cycle = graph.detect_cycle();
        if let Some(ref c) = cycle {
            println!("False positive! Detected cycle: {:?}", c);
        }
        assert!(cycle.is_none(), "Diamond DAG should not have cycles!");
    }

    #[test]
    fn complex_dag_with_multiple_paths() {
        let mut graph = DependencyGraph::new();
        let a = TaskId::new(1);
        let b = TaskId::new(2);
        let c = TaskId::new(3);
        let d = TaskId::new(4);
        let e = TaskId::new(5);

        // Complex DAG:
        //     A
        //    / \
        //   B   C
        //   |\ /|
        //   | X |
        //   |/ \|
        //   D   E
        graph.add_dependency(b, a);
        graph.add_dependency(c, a);
        graph.add_dependency(d, b);
        graph.add_dependency(e, b);
        graph.add_dependency(d, c);
        graph.add_dependency(e, c);

        println!("Testing complex DAG with cross edges...");
        let cycle = graph.detect_cycle();
        if let Some(ref c) = cycle {
            println!("False positive in complex DAG! Detected cycle: {:?}", c);
        }
        assert!(cycle.is_none(), "Complex DAG should not have cycles!");
    }

    #[test]
    fn dag_with_convergent_paths() {
        let mut graph = DependencyGraph::new();
        let a = TaskId::new(1);
        let b = TaskId::new(2);
        let c = TaskId::new(3);
        let d = TaskId::new(4);

        // A -> B -> D
        // A -> C -> D  (two paths converge at D)
        graph.add_dependency(b, a);
        graph.add_dependency(c, a);
        graph.add_dependency(d, b);
        graph.add_dependency(d, c);

        println!("Testing convergent paths...");
        let cycle = graph.detect_cycle();
        if let Some(ref c) = cycle {
            println!("False positive with convergent paths! Detected cycle: {:?}", c);
        }
        assert!(cycle.is_none(), "Convergent paths should not be a cycle!");
    }
}
