use crate::{error::AppError, manifest::Manifest};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct ModelNode {
    pub unique_id: String,
    pub name: String,
    pub package_name: String,
    pub fqn: Vec<String>,
    pub tags: Vec<String>,
    pub original_file_path: String,
    pub config: Value,
}

#[derive(Debug)]
pub struct GraphIndex {
    by_unique_id: HashMap<String, ModelNode>,
    name_to_ids: HashMap<String, Vec<String>>,
    parents: HashMap<String, Vec<String>>,
    children: HashMap<String, Vec<String>>,
}

impl GraphIndex {
    pub fn from_manifest(manifest: &Manifest) -> Self {
        let mut by_unique_id: HashMap<String, ModelNode> = HashMap::new();
        let mut name_to_ids: HashMap<String, Vec<String>> = HashMap::new();

        for (unique_id, node) in &manifest.nodes {
            if node.resource_type != "model" {
                continue;
            }

            let model = ModelNode {
                unique_id: unique_id.clone(),
                name: node.name.clone(),
                package_name: node.package_name.clone(),
                fqn: node.fqn.clone(),
                tags: node.tags.clone(),
                original_file_path: node.original_file_path.clone(),
                config: node.config.clone(),
            };
            by_unique_id.insert(unique_id.clone(), model);
            name_to_ids
                .entry(node.name.clone())
                .or_default()
                .push(unique_id.clone());
        }

        let model_ids: HashSet<&String> = by_unique_id.keys().collect();
        let parents = filter_edges(&manifest.parent_map, &model_ids);
        let children = filter_edges(&manifest.child_map, &model_ids);

        Self {
            by_unique_id,
            name_to_ids,
            parents,
            children,
        }
    }

    pub fn resolve_model(&self, model_name: &str) -> Result<&str, AppError> {
        match self.name_to_ids.get(model_name) {
            None => Err(AppError::ModelNotFound {
                model_name: model_name.to_string(),
            }),
            Some(matches) if matches.len() == 1 => Ok(matches[0].as_str()),
            Some(matches) => {
                let mut candidates = Vec::new();
                for id in matches {
                    let node = self
                        .by_unique_id
                        .get(id)
                        .expect("model id in name_to_ids must exist in by_unique_id");
                    candidates.push(format!(
                        "{}.{} ({})",
                        node.package_name, node.name, node.unique_id
                    ));
                }
                candidates.sort();
                Err(AppError::ModelAmbiguous {
                    model_name: model_name.to_string(),
                    candidates: candidates
                        .into_iter()
                        .map(|c| format!("  - {c}"))
                        .collect::<Vec<_>>()
                        .join("\n"),
                })
            }
        }
    }

    pub fn node_name<'a>(&'a self, unique_id: &'a str) -> &'a str {
        match self.by_unique_id.get(unique_id) {
            Some(node) => node.name.as_str(),
            None => unique_id,
        }
    }

    pub fn node_label(&self, unique_id: &str) -> String {
        match self.by_unique_id.get(unique_id) {
            Some(node) => {
                let collisions = self
                    .name_to_ids
                    .get(node.name.as_str())
                    .map_or(0, std::vec::Vec::len);
                if collisions > 1 {
                    format!("{}.{}", node.package_name, node.name)
                } else {
                    node.name.clone()
                }
            }
            None => unique_id.to_string(),
        }
    }

    pub fn parents_of(&self, unique_id: &str) -> &[String] {
        self.parents
            .get(unique_id)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    pub fn children_of(&self, unique_id: &str) -> &[String] {
        self.children
            .get(unique_id)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    pub fn sorted_neighbors<'a>(&'a self, ids: &'a [String]) -> Vec<&'a String> {
        let mut out: Vec<&String> = ids.iter().collect();
        out.sort_by(|left, right| {
            let left_name = self.node_name(left);
            let right_name = self.node_name(right);
            left_name.cmp(right_name).then_with(|| left.cmp(right))
        });
        out
    }

    pub fn sorted_model_ids(&self) -> Vec<&String> {
        let mut ids: Vec<&String> = self.by_unique_id.keys().collect();
        ids.sort_by(|left, right| {
            let left_name = self.node_name(left);
            let right_name = self.node_name(right);
            left_name.cmp(right_name).then_with(|| left.cmp(right))
        });
        ids
    }

    pub fn select_by_tag_pattern(&self, pattern: &str) -> HashSet<String> {
        self.by_unique_id
            .iter()
            .filter(|(_, node)| {
                node.tags
                    .iter()
                    .any(|tag| wildcard_match(pattern, tag.as_str()))
            })
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn select_by_fqn_pattern(&self, pattern: &str) -> HashSet<String> {
        self.by_unique_id
            .iter()
            .filter(|(_, node)| wildcard_match(pattern, node.fqn.join(".").as_str()))
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn select_by_path_pattern(&self, pattern: &str) -> HashSet<String> {
        self.by_unique_id
            .iter()
            .filter(|(_, node)| path_matches(pattern, node.original_file_path.as_str()))
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn select_by_config_value(&self, key_path: &[String], expected: &str) -> HashSet<String> {
        self.by_unique_id
            .iter()
            .filter(|(_, node)| match_config_value(&node.config, key_path, expected))
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn select_by_name_pattern(&self, pattern: &str) -> HashSet<String> {
        self.by_unique_id
            .iter()
            .filter(|(_, node)| wildcard_match(pattern, node.name.as_str()))
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn expand_ancestors(&self, seeds: &HashSet<String>, max_depth: usize) -> HashSet<String> {
        expand_direction(self, seeds, max_depth, Direction::Up)
    }

    pub fn expand_descendants(&self, seeds: &HashSet<String>, max_depth: usize) -> HashSet<String> {
        expand_direction(self, seeds, max_depth, Direction::Down)
    }
}

#[derive(Clone, Copy)]
enum Direction {
    Up,
    Down,
}

fn expand_direction(
    graph: &GraphIndex,
    seeds: &HashSet<String>,
    max_depth: usize,
    direction: Direction,
) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut best_depth = HashMap::<String, usize>::new();
    let mut stack = Vec::<(String, usize)>::new();
    for seed in seeds {
        best_depth.insert(seed.clone(), 0);
        stack.push((seed.clone(), 0));
    }

    while let Some((current, depth)) = stack.pop() {
        if depth >= max_depth {
            continue;
        }
        let neighbors = match direction {
            Direction::Up => graph.parents_of(&current),
            Direction::Down => graph.children_of(&current),
        };
        for neighbor in neighbors {
            let next_depth = depth + 1;
            let should_visit = best_depth
                .get(neighbor.as_str())
                .is_none_or(|prev| next_depth < *prev);
            if should_visit {
                best_depth.insert(neighbor.clone(), next_depth);
                out.insert(neighbor.clone());
                stack.push((neighbor.clone(), next_depth));
            }
        }
    }

    out
}

fn wildcard_match(pattern: &str, input: &str) -> bool {
    let p = pattern.as_bytes();
    let s = input.as_bytes();
    let mut dp = vec![vec![false; s.len() + 1]; p.len() + 1];
    dp[0][0] = true;

    for i in 1..=p.len() {
        if p[i - 1] == b'*' {
            dp[i][0] = dp[i - 1][0];
        }
    }

    for i in 1..=p.len() {
        for j in 1..=s.len() {
            dp[i][j] = match p[i - 1] {
                b'*' => dp[i - 1][j] || dp[i][j - 1],
                b'?' => dp[i - 1][j - 1],
                ch => dp[i - 1][j - 1] && ch == s[j - 1],
            };
        }
    }

    dp[p.len()][s.len()]
}

fn path_matches(pattern: &str, path: &str) -> bool {
    if pattern.contains('*') || pattern.contains('?') {
        return wildcard_match(pattern, path);
    }
    path == pattern || path.starts_with(format!("{pattern}/").as_str())
}

fn match_config_value(value: &Value, key_path: &[String], expected: &str) -> bool {
    let Some(found) = lookup_config_value(value, key_path) else {
        return false;
    };
    value_contains(found, expected)
}

fn lookup_config_value<'a>(value: &'a Value, key_path: &[String]) -> Option<&'a Value> {
    if key_path.is_empty() {
        return Some(value);
    }
    let mut current = value;
    for key in key_path {
        current = current.get(key)?;
    }
    Some(current)
}

fn value_contains(value: &Value, expected: &str) -> bool {
    match value {
        Value::String(v) => v == expected,
        Value::Bool(v) => v.to_string() == expected,
        Value::Number(v) => v.to_string() == expected,
        Value::Array(values) => values.iter().any(|v| value_contains(v, expected)),
        _ => false,
    }
}

fn filter_edges(
    edges: &HashMap<String, Vec<String>>,
    model_ids: &HashSet<&String>,
) -> HashMap<String, Vec<String>> {
    let mut filtered = HashMap::new();
    for (source, targets) in edges {
        if !model_ids.contains(source) {
            continue;
        }
        let mut kept = Vec::new();
        for target in targets {
            if model_ids.contains(target) {
                kept.push(target.clone());
            }
        }
        if !kept.is_empty() {
            filtered.insert(source.clone(), kept);
        }
    }
    filtered
}

#[cfg(test)]
mod tests {
    use super::GraphIndex;
    use crate::manifest::{Manifest, NodeEntry};
    use proptest::prelude::*;
    use std::collections::HashMap;

    fn model_entry(name: &str, package: &str) -> NodeEntry {
        NodeEntry {
            resource_type: "model".to_string(),
            name: name.to_string(),
            package_name: package.to_string(),
            fqn: vec![],
            tags: vec![],
            original_file_path: String::new(),
            config: serde_json::json!({}),
        }
    }

    fn test_entry(name: &str, package: &str) -> NodeEntry {
        NodeEntry {
            resource_type: "test".to_string(),
            name: name.to_string(),
            package_name: package.to_string(),
            fqn: vec![],
            tags: vec![],
            original_file_path: String::new(),
            config: serde_json::json!({}),
        }
    }

    fn fixture_manifest() -> Manifest {
        let mut nodes = HashMap::new();
        nodes.insert("model.pkg.a".to_string(), model_entry("a", "pkg"));
        nodes.insert("model.pkg.b".to_string(), model_entry("b", "pkg"));
        nodes.insert(
            "test.pkg.b_not_null".to_string(),
            test_entry("b_not_null", "pkg"),
        );
        nodes.insert("model.other.b".to_string(), model_entry("b", "other"));

        let mut parent_map = HashMap::new();
        parent_map.insert("model.pkg.b".to_string(), vec!["model.pkg.a".to_string()]);

        let mut child_map = HashMap::new();
        child_map.insert(
            "model.pkg.a".to_string(),
            vec!["model.pkg.b".to_string(), "test.pkg.b_not_null".to_string()],
        );

        Manifest {
            nodes,
            parent_map,
            child_map,
        }
    }

    #[test]
    fn filters_to_model_edges_only() {
        let graph = GraphIndex::from_manifest(&fixture_manifest());
        assert_eq!(
            graph.children_of("model.pkg.a"),
            &["model.pkg.b".to_string()]
        );
    }

    #[test]
    fn resolves_single_and_ambiguous_names() {
        let graph = GraphIndex::from_manifest(&fixture_manifest());
        assert_eq!(
            graph.resolve_model("a").expect("model should resolve"),
            "model.pkg.a"
        );
        assert!(graph.resolve_model("b").is_err(), "b should be ambiguous");
        assert!(
            graph.resolve_model("missing").is_err(),
            "missing should error"
        );
    }

    proptest! {
        #[test]
        fn non_model_nodes_are_always_filtered(edge_count in 0usize..20) {
            let mut nodes = HashMap::new();
            nodes.insert(
                "model.pkg.m".to_string(),
                model_entry("m", "pkg"),
            );
            nodes.insert(
                "test.pkg.t".to_string(),
                test_entry("t", "pkg"),
            );

            let mut parent_map = HashMap::new();
            let mut child_map = HashMap::new();
            let mut test_edges = Vec::new();
            for _ in 0..edge_count {
                test_edges.push("test.pkg.t".to_string());
            }
            parent_map.insert("model.pkg.m".to_string(), test_edges.clone());
            child_map.insert("model.pkg.m".to_string(), test_edges);

            let graph = GraphIndex::from_manifest(&Manifest {
                nodes,
                parent_map,
                child_map,
            });

            prop_assert!(graph.parents_of("model.pkg.m").is_empty());
            prop_assert!(graph.children_of("model.pkg.m").is_empty());
        }
    }
}
