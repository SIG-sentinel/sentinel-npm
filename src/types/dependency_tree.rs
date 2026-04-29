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

pub(crate) struct DfsCycleDetectionParams<'a> {
    pub(crate) node_key: &'a str,
    pub(crate) visited: &'a mut HashSet<String>,
    pub(crate) recursion_stack: &'a mut HashSet<String>,
    pub(crate) path: &'a mut Vec<String>,
    pub(crate) cycles: &'a mut Vec<Vec<String>>,
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
}

#[cfg(test)]
#[path = "../../tests/internal/dependency_tree_tests.rs"]
mod tests;
