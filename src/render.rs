use crate::graph::GraphIndex;
use std::collections::HashSet;

#[path = "render/ascii.rs"]
mod ascii;
#[path = "render/layout.rs"]
mod layout;

pub fn render_selected_nodes(graph: &GraphIndex, nodes: &HashSet<String>) -> String {
    render_components(graph, nodes)
}

pub fn render_all_models(graph: &GraphIndex) -> String {
    let nodes = graph
        .sorted_model_ids()
        .into_iter()
        .map(|id| id.to_string())
        .collect::<HashSet<_>>();
    render_components(graph, &nodes)
}

fn render_components(graph: &GraphIndex, nodes: &HashSet<String>) -> String {
    let components = connected_components(graph, nodes);
    components
        .into_iter()
        .map(|component| {
            let layout = layout::build_layout(graph, &component);
            ascii::render_ascii_cards(graph, &layout)
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn connected_components(graph: &GraphIndex, nodes: &HashSet<String>) -> Vec<HashSet<String>> {
    let mut remaining = nodes.clone();
    let mut components = Vec::<HashSet<String>>::new();

    while let Some(seed) = remaining.iter().next().cloned() {
        let mut component = HashSet::new();
        let mut stack = vec![seed];

        while let Some(current) = stack.pop() {
            if !remaining.remove(&current) {
                continue;
            }
            component.insert(current.clone());

            for child in graph.children_of(&current) {
                if remaining.contains(child) {
                    stack.push(child.clone());
                }
            }
            for parent in graph.parents_of(&current) {
                if remaining.contains(parent) {
                    stack.push(parent.clone());
                }
            }
        }

        components.push(component);
    }

    components.sort_by(|left, right| {
        let left_key = component_sort_key(graph, left);
        let right_key = component_sort_key(graph, right);
        left_key.cmp(&right_key)
    });
    components
}

fn component_sort_key(graph: &GraphIndex, component: &HashSet<String>) -> String {
    let mut ids = component.iter().cloned().collect::<Vec<_>>();
    layout::sort_ids(graph, &mut ids);
    ids.first()
        .map(|id| graph.node_label(id))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{render_all_models, render_selected_nodes};
    use crate::{
        graph::GraphIndex,
        manifest::{Manifest, NodeEntry},
    };
    use std::collections::{HashMap, HashSet};

    fn model_entry(name: &str) -> NodeEntry {
        NodeEntry {
            resource_type: "model".to_string(),
            name: name.to_string(),
            package_name: "pkg".to_string(),
            fqn: vec![],
            tags: vec![],
            original_file_path: String::new(),
            config: serde_json::json!({}),
        }
    }

    fn graph_fixture() -> GraphIndex {
        let mut nodes = HashMap::new();
        nodes.insert("model.pkg.root".to_string(), model_entry("root"));
        nodes.insert("model.pkg.mid".to_string(), model_entry("mid"));
        nodes.insert("model.pkg.leaf".to_string(), model_entry("leaf"));

        let mut parent_map = HashMap::new();
        parent_map.insert(
            "model.pkg.mid".to_string(),
            vec!["model.pkg.root".to_string()],
        );
        parent_map.insert(
            "model.pkg.leaf".to_string(),
            vec!["model.pkg.mid".to_string()],
        );

        let mut child_map = HashMap::new();
        child_map.insert(
            "model.pkg.root".to_string(),
            vec!["model.pkg.mid".to_string()],
        );
        child_map.insert(
            "model.pkg.mid".to_string(),
            vec!["model.pkg.leaf".to_string()],
        );

        GraphIndex::from_manifest(&Manifest {
            nodes,
            parent_map,
            child_map,
        })
    }

    #[test]
    fn renders_selected_subset() {
        let graph = graph_fixture();
        let nodes = HashSet::from(["model.pkg.mid".to_string()]);
        let rendered = render_selected_nodes(&graph, &nodes);
        assert!(rendered.contains("[mid]"));
        assert!(!rendered.contains("[root]"));
        assert!(!rendered.contains("[leaf]"));
    }

    #[test]
    fn renders_all_models_in_single_component() {
        let graph = graph_fixture();
        let rendered = render_all_models(&graph);
        assert!(rendered.contains("[root]"));
        assert!(rendered.contains("[mid]"));
        assert!(rendered.contains("[leaf]"));
        assert!(!rendered.contains("\n\n"));
    }
}
