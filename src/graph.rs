use crate::{error::AppError, manifest::Manifest};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct ModelNode {
    pub unique_id: String,
    pub name: String,
    pub package_name: String,
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

    fn fixture_manifest() -> Manifest {
        let mut nodes = HashMap::new();
        nodes.insert(
            "model.pkg.a".to_string(),
            NodeEntry {
                resource_type: "model".to_string(),
                name: "a".to_string(),
                package_name: "pkg".to_string(),
            },
        );
        nodes.insert(
            "model.pkg.b".to_string(),
            NodeEntry {
                resource_type: "model".to_string(),
                name: "b".to_string(),
                package_name: "pkg".to_string(),
            },
        );
        nodes.insert(
            "test.pkg.b_not_null".to_string(),
            NodeEntry {
                resource_type: "test".to_string(),
                name: "b_not_null".to_string(),
                package_name: "pkg".to_string(),
            },
        );
        nodes.insert(
            "model.other.b".to_string(),
            NodeEntry {
                resource_type: "model".to_string(),
                name: "b".to_string(),
                package_name: "other".to_string(),
            },
        );

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
                NodeEntry {
                    resource_type: "model".to_string(),
                    name: "m".to_string(),
                    package_name: "pkg".to_string(),
                },
            );
            nodes.insert(
                "test.pkg.t".to_string(),
                NodeEntry {
                    resource_type: "test".to_string(),
                    name: "t".to_string(),
                    package_name: "pkg".to_string(),
                },
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
