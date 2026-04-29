use std::collections::{HashMap, HashSet, VecDeque};

use crate::types::dependency_tree::DfsCycleDetectionParams;
use crate::types::{DependencyTree, DependencyTreeAnalysis, PackageRef};

const ROOT_DEPTH: usize = 1;
const INITIAL_MAX_DEPTH: usize = 1;
const ZERO_IN_DEGREE: usize = 0;
const IN_DEGREE_DECREMENT: usize = 1;

fn build_all_as_deps(tree: &DependencyTree) -> HashSet<String> {
    let mut all_as_deps = HashSet::new();

    for node in tree.nodes.values() {
        all_as_deps.extend(node.dependencies.iter().cloned());
    }

    all_as_deps
}

fn detect_cycles_for_tree(tree: &DependencyTree) -> Vec<Vec<String>> {
    let mut cycles = Vec::new();
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();
    let mut path = Vec::new();

    for node_key in tree.nodes.keys() {
        if !visited.contains(node_key) {
            let dfs_cycle_detection_params = DfsCycleDetectionParams {
                node_key,
                visited: &mut visited,
                recursion_stack: &mut rec_stack,
                path: &mut path,
                cycles: &mut cycles,
            };

            dfs_cycle_detection(tree, dfs_cycle_detection_params);
        }
    }

    cycles
}

fn dfs_cycle_detection(tree: &DependencyTree, params: DfsCycleDetectionParams<'_>) {
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

    let Some(node) = tree.nodes.get(node_key) else {
        path.pop();
        recursion_stack.remove(node_key);

        return;
    };

    for dep in &node.dependencies {
        let is_unvisited_dep = !visited.contains(dep);

        if is_unvisited_dep {
            let dfs_cycle_detection_params = DfsCycleDetectionParams {
                node_key: dep,
                visited,
                recursion_stack,
                path,
                cycles,
            };

            dfs_cycle_detection(tree, dfs_cycle_detection_params);

            continue;
        }

        let is_back_edge = recursion_stack.contains(dep);

        if !is_back_edge {
            continue;
        }

        let Some(cycle_start_idx) = path.iter().position(|p| p == dep) else {
            continue;
        };

        let cycle: Vec<String> = path[cycle_start_idx..].to_vec();

        cycles.push(cycle);
    }

    path.pop();
    recursion_stack.remove(node_key);
}

fn calculate_max_depth_for_tree(tree: &DependencyTree) -> usize {
    let mut max = INITIAL_MAX_DEPTH;
    let all_as_deps = build_all_as_deps(tree);

    for key in tree.nodes.keys() {
        if !all_as_deps.contains(key) {
            let depth = bfs_max_depth_for_tree(tree, key);
            max = max.max(depth);
        }
    }

    max
}

fn bfs_max_depth_for_tree(tree: &DependencyTree, start: &str) -> usize {
    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();

    queue.push_back((start.to_string(), ROOT_DEPTH));

    let mut max_depth = INITIAL_MAX_DEPTH;

    while let Some((node_key, depth)) = queue.pop_front() {
        let is_new_node = visited.insert(node_key.clone());

        if !is_new_node {
            continue;
        }

        max_depth = max_depth.max(depth);

        let Some(node) = tree.nodes.get(&node_key) else {
            continue;
        };

        queue.extend(
            node.dependencies
                .iter()
                .cloned()
                .map(|dependency| (dependency, depth + IN_DEGREE_DECREMENT)),
        );
    }

    max_depth
}

fn build_in_degree_map(tree: &DependencyTree) -> HashMap<String, usize> {
    let mut in_degree = HashMap::new();

    for (key, node) in &tree.nodes {
        in_degree.insert(key.clone(), node.dependencies.len());
    }

    in_degree
}

fn build_dependents_by_dependency(tree: &DependencyTree) -> HashMap<String, Vec<String>> {
    let mut dependents_by_dependency: HashMap<String, Vec<String>> = HashMap::new();

    let dependency_pairs = tree.nodes.iter().flat_map(|(key, node)| {
        node.dependencies
            .iter()
            .cloned()
            .map(move |dependency| (dependency, key.clone()))
    });

    for (dependency, dependent_key) in dependency_pairs {
        dependents_by_dependency
            .entry(dependency)
            .or_default()
            .push(dependent_key);
    }

    dependents_by_dependency
}

fn collect_zero_in_degree_queue(in_degree: &HashMap<String, usize>) -> Vec<String> {
    in_degree
        .iter()
        .filter(|&(_, &degree)| degree == ZERO_IN_DEGREE)
        .map(|(key, _)| key.clone())
        .collect()
}

fn traverse_topological_order(
    in_degree: &mut HashMap<String, usize>,
    dependents_by_dependency: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    let mut queue = collect_zero_in_degree_queue(in_degree);
    let mut sorted = Vec::new();

    while let Some(node_key) = queue.pop() {
        sorted.push(node_key.clone());

        let dependent_keys = dependents_by_dependency
            .get(&node_key)
            .cloned()
            .unwrap_or_default();

        for dependent_key in dependent_keys {
            let Some(degree) = in_degree.get_mut(&dependent_key) else {
                continue;
            };

            *degree -= IN_DEGREE_DECREMENT;

            let reached_zero = *degree == ZERO_IN_DEGREE;

            if !reached_zero {
                continue;
            }

            queue.push(dependent_key);
        }
    }

    sorted
}

impl DependencyTree {
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
        detect_cycles_for_tree(self)
    }

    pub fn analyze(&self) -> DependencyTreeAnalysis {
        let total_packages = self.nodes.len();
        let all_as_deps = build_all_as_deps(self);
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

        let max_depth = calculate_max_depth_for_tree(self);

        DependencyTreeAnalysis {
            total_packages,
            direct_packages,
            transitive_packages,
            cycles,
            orphaned,
            max_depth,
        }
    }

    pub fn topological_sort(&self) -> Result<Vec<String>, Vec<Vec<String>>> {
        let cycles = self.detect_cycles();

        if !cycles.is_empty() {
            return Err(cycles);
        }

        let mut in_degree = build_in_degree_map(self);
        let dependents_by_dependency = build_dependents_by_dependency(self);
        let sorted = traverse_topological_order(&mut in_degree, &dependents_by_dependency);

        if sorted.len() != self.nodes.len() {
            return Err(vec![]);
        }

        Ok(sorted)
    }
}
