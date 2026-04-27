use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use super::package::PackageRef;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DependencyNode {
    pub package: PackageRef,
    pub dependencies: Vec<String>,
    pub is_dev: bool,
    pub is_direct: bool,
    #[serde(default)]
    pub direct_parent: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DependencyTree {
    pub nodes: HashMap<String, DependencyNode>,
}

pub struct ResolveChildrenForParentAssignmentParams<'a> {
    pub tree: &'a mut DependencyTree,
    pub dependency_key: &'a str,
    pub direct_key: &'a str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyTreeAnalysis {
    pub total_packages: usize,
    pub direct_packages: Vec<PackageRef>,
    pub transitive_packages: Vec<PackageRef>,
    pub cycles: Vec<Vec<String>>,
    pub orphaned: Vec<PackageRef>,
    pub max_depth: usize,
}

struct DfsCycleDetectionParams<'a> {
    node_key: &'a str,
    visited: &'a mut HashSet<String>,
    recursion_stack: &'a mut HashSet<String>,
    path: &'a mut Vec<String>,
    cycles: &'a mut Vec<Vec<String>>,
}

impl DependencyTree {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    pub fn insert(&mut self, node: DependencyNode) {
        self.nodes.insert(node.package.to_string(), node);
    }

    pub fn get_transitive_deps(&self, package_ref: &PackageRef) -> HashSet<String> {
        let mut visited = HashSet::new();
        let mut to_visit = vec![package_ref.to_string()];

        while let Some(current) = to_visit.pop() {
            if visited.insert(current.clone())
                && let Some(node) = self.nodes.get(&current)
            {
                to_visit.extend(node.dependencies.iter().cloned());
            }
        }

        visited.remove(&package_ref.to_string());
        visited
    }

    pub fn detect_cycles(&self) -> Vec<Vec<String>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for node_key in self.nodes.keys() {
            if !visited.contains(node_key) {
                self.dfs_cycle_detection(DfsCycleDetectionParams {
                    node_key,
                    visited: &mut visited,
                    recursion_stack: &mut rec_stack,
                    path: &mut path,
                    cycles: &mut cycles,
                });
            }
        }

        cycles
    }

    fn dfs_cycle_detection(&self, params: DfsCycleDetectionParams<'_>) {
        let DfsCycleDetectionParams {
            node_key,
            visited,
            recursion_stack,
            path,
            cycles,
        } = params;

        visited.insert(node_key.to_string());
        recursion_stack.insert(node_key.to_string());
        path.push(node_key.to_string());

        if let Some(node) = self.nodes.get(node_key) {
            for dep in &node.dependencies {
                if !visited.contains(dep) {
                    self.dfs_cycle_detection(DfsCycleDetectionParams {
                        node_key: dep,
                        visited,
                        recursion_stack,
                        path,
                        cycles,
                    });
                    continue;
                }

                if !recursion_stack.contains(dep) {
                    continue;
                }

                if let Some(cycle_start_idx) = path.iter().position(|p| p == dep) {
                    let cycle: Vec<String> = path[cycle_start_idx..].to_vec();
                    cycles.push(cycle);
                }
            }
        }

        path.pop();
        recursion_stack.remove(node_key);
    }

    pub fn analyze(&self) -> DependencyTreeAnalysis {
        let total_packages = self.nodes.len();

        let mut all_as_deps = HashSet::new();
        for node in self.nodes.values() {
            all_as_deps.extend(node.dependencies.iter().cloned());
        }

        let mut direct_packages = Vec::new();
        let mut transitive_packages = Vec::new();

        for (key, node) in &self.nodes {
            let is_direct_package = !all_as_deps.contains(key);
            if is_direct_package {
                direct_packages.push(node.package.clone());

                continue;
            }

            transitive_packages.push(node.package.clone());
        }

        let cycles = self.detect_cycles();

        let mut orphaned = Vec::new();
        let root_keys: HashSet<String> = direct_packages.iter().map(ToString::to_string).collect();

        for (key, node) in &self.nodes {
            if root_keys.contains(key) {
                continue;
            }

            let is_reachable_from_roots = root_keys.iter().any(|root_key| {
                self.nodes.get(root_key).is_some_and(|root_node| {
                    self.get_transitive_deps(&root_node.package).contains(key)
                })
            });

            if is_reachable_from_roots {
                continue;
            }

            orphaned.push(node.package.clone());
        }

        let max_depth = self.calculate_max_depth();

        DependencyTreeAnalysis {
            total_packages,
            direct_packages,
            transitive_packages,
            cycles,
            orphaned,
            max_depth,
        }
    }

    fn calculate_max_depth(&self) -> usize {
        let mut max = 1;

        let mut all_as_deps = HashSet::new();
        for node in self.nodes.values() {
            all_as_deps.extend(node.dependencies.iter().cloned());
        }

        for key in self.nodes.keys() {
            if !all_as_deps.contains(key) {
                let depth = self.bfs_max_depth(key);
                max = max.max(depth);
            }
        }

        max
    }

    fn bfs_max_depth(&self, start: &str) -> usize {
        use std::collections::VecDeque;

        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        queue.push_back((start.to_string(), 1));
        let mut max_depth = 1;

        while let Some((node_key, depth)) = queue.pop_front() {
            if visited.insert(node_key.clone()) {
                max_depth = max_depth.max(depth);
                if let Some(node) = self.nodes.get(&node_key) {
                    queue.extend(
                        node.dependencies
                            .iter()
                            .cloned()
                            .map(|dependency| (dependency, depth + 1)),
                    );
                }
            }
        }

        max_depth
    }

    pub fn topological_sort(&self) -> Result<Vec<String>, Vec<Vec<String>>> {
        let cycles = self.detect_cycles();
        if !cycles.is_empty() {
            return Err(cycles);
        }

        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut dependents_by_dependency: HashMap<String, Vec<String>> = HashMap::new();
        for (key, node) in &self.nodes {
            in_degree.insert(key.clone(), node.dependencies.len());
            for dependency in &node.dependencies {
                dependents_by_dependency
                    .entry(dependency.clone())
                    .or_default()
                    .push(key.clone());
            }
        }

        let mut queue: Vec<String> = in_degree
            .iter()
            .filter(|&(_, &degree)| degree == 0)
            .map(|(k, _)| k.clone())
            .collect();

        let mut sorted = Vec::new();

        while let Some(node_key) = queue.pop() {
            sorted.push(node_key.clone());

            let dependent_keys = dependents_by_dependency
                .get(&node_key)
                .cloned()
                .unwrap_or_default();
            for dependent_key in dependent_keys {
                if let Some(degree) = in_degree.get_mut(&dependent_key) {
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push(dependent_key);
                    }
                }
            }
        }

        if sorted.len() != self.nodes.len() {
            return Err(vec![]);
        }

        Ok(sorted)
    }
}

#[cfg(test)]
#[path = "../../tests/internal/dependency_tree_tests.rs"]
mod tests;
