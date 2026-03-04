use crate::{cli::SelectorSpec, graph::GraphIndex};
use std::collections::{HashMap, HashSet};

#[path = "render/ascii.rs"]
mod ascii;
#[path = "render/layout.rs"]
mod layout;

pub fn render_selection(graph: &GraphIndex, root_id: &str, selector: &SelectorSpec) -> String {
    let selections = vec![(root_id.to_string(), selector.clone())];
    render_union_selection(graph, &selections)
}

pub fn render_union_selection(graph: &GraphIndex, selections: &[(String, SelectorSpec)]) -> String {
    let mut nodes = HashSet::new();
    for (root_id, selector) in selections {
        nodes.extend(collect_selected_nodes(graph, root_id, selector));
    }
    render_components(graph, &nodes)
}

pub fn render_all_models(graph: &GraphIndex) -> String {
    let nodes = graph
        .sorted_model_ids()
        .into_iter()
        .map(|id| id.to_string())
        .collect::<HashSet<_>>();
    render_components(graph, &nodes)
}

#[derive(Clone, Copy)]
enum Direction {
    Up,
    Down,
}

fn collect_selected_nodes(
    graph: &GraphIndex,
    root_id: &str,
    selector: &SelectorSpec,
) -> HashSet<String> {
    let mut nodes = HashSet::new();
    nodes.insert(root_id.to_string());

    if let Some(depth) = selector.ancestor_depth_limit() {
        collect_reachable(graph, root_id, Direction::Up, depth, &mut nodes);
    }
    if let Some(depth) = selector.descendant_depth_limit() {
        collect_reachable(graph, root_id, Direction::Down, depth, &mut nodes);
    }

    nodes
}

fn collect_reachable(
    graph: &GraphIndex,
    start: &str,
    direction: Direction,
    max_depth: usize,
    out: &mut HashSet<String>,
) {
    let mut stack = vec![(start.to_string(), 0usize)];
    let mut best_depth = HashMap::<String, usize>::new();
    best_depth.insert(start.to_string(), 0);

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
    use super::{render_all_models, render_selection};
    use crate::{
        cli::SelectorSpec,
        graph::GraphIndex,
        manifest::{Manifest, NodeEntry},
    };
    use std::collections::HashMap;

    fn graph_fixture() -> GraphIndex {
        let mut nodes = HashMap::new();
        nodes.insert(
            "model.pkg.root".to_string(),
            NodeEntry {
                resource_type: "model".to_string(),
                name: "root".to_string(),
                package_name: "pkg".to_string(),
            },
        );
        nodes.insert(
            "model.pkg.mid".to_string(),
            NodeEntry {
                resource_type: "model".to_string(),
                name: "mid".to_string(),
                package_name: "pkg".to_string(),
            },
        );
        nodes.insert(
            "model.pkg.leaf".to_string(),
            NodeEntry {
                resource_type: "model".to_string(),
                name: "leaf".to_string(),
                package_name: "pkg".to_string(),
            },
        );

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
    fn renders_selected_only() {
        let graph = graph_fixture();
        let selector = SelectorSpec {
            ancestor_depth: None,
            descendant_depth: None,
            model_name: "mid".to_string(),
        };
        let rendered = render_selection(&graph, "model.pkg.mid", &selector);
        assert!(rendered.contains("[mid]"));
        assert!(!rendered.contains("[root]"));
        assert!(!rendered.contains("[leaf]"));
    }

    #[test]
    fn renders_descendants_dag() {
        let graph = graph_fixture();
        let selector = SelectorSpec {
            ancestor_depth: None,
            descendant_depth: Some(usize::MAX),
            model_name: "mid".to_string(),
        };
        let rendered = render_selection(&graph, "model.pkg.mid", &selector);
        assert!(rendered.contains("[mid]"));
        assert!(rendered.contains("[leaf]"));
    }

    #[test]
    fn renders_ancestors_dag() {
        let graph = graph_fixture();
        let selector = SelectorSpec {
            ancestor_depth: Some(usize::MAX),
            descendant_depth: None,
            model_name: "mid".to_string(),
        };
        let rendered = render_selection(&graph, "model.pkg.mid", &selector);
        assert!(rendered.contains("[root]"));
        assert!(rendered.contains("[mid]"));
    }

    #[test]
    fn renders_both_directions_in_single_dag() {
        let graph = graph_fixture();
        let selector = SelectorSpec {
            ancestor_depth: Some(usize::MAX),
            descendant_depth: Some(usize::MAX),
            model_name: "mid".to_string(),
        };
        let rendered = render_selection(&graph, "model.pkg.mid", &selector);
        assert!(rendered.contains("[root]"));
        assert!(rendered.contains("[mid]"));
        assert!(rendered.contains("[leaf]"));
    }

    #[test]
    fn renders_all_models_in_single_dag() {
        let graph = graph_fixture();
        let rendered = render_all_models(&graph);
        assert!(rendered.contains("[root]"));
        assert!(rendered.contains("[mid]"));
        assert!(rendered.contains("[leaf]"));
        assert!(!rendered.contains("\n\n"));
    }

    #[test]
    fn does_not_duplicate_nodes_in_diamond_dag() {
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
            "model.pkg.c".to_string(),
            NodeEntry {
                resource_type: "model".to_string(),
                name: "c".to_string(),
                package_name: "pkg".to_string(),
            },
        );
        nodes.insert(
            "model.pkg.d".to_string(),
            NodeEntry {
                resource_type: "model".to_string(),
                name: "d".to_string(),
                package_name: "pkg".to_string(),
            },
        );

        let mut child_map = HashMap::new();
        child_map.insert(
            "model.pkg.a".to_string(),
            vec!["model.pkg.b".to_string(), "model.pkg.c".to_string()],
        );
        child_map.insert("model.pkg.b".to_string(), vec!["model.pkg.d".to_string()]);
        child_map.insert("model.pkg.c".to_string(), vec!["model.pkg.d".to_string()]);

        let mut parent_map = HashMap::new();
        parent_map.insert("model.pkg.b".to_string(), vec!["model.pkg.a".to_string()]);
        parent_map.insert("model.pkg.c".to_string(), vec!["model.pkg.a".to_string()]);
        parent_map.insert(
            "model.pkg.d".to_string(),
            vec!["model.pkg.b".to_string(), "model.pkg.c".to_string()],
        );

        let graph = GraphIndex::from_manifest(&Manifest {
            nodes,
            parent_map,
            child_map,
        });
        let selector = SelectorSpec {
            ancestor_depth: None,
            descendant_depth: Some(usize::MAX),
            model_name: "a".to_string(),
        };

        let rendered = render_selection(&graph, "model.pkg.a", &selector);
        assert_eq!(rendered.matches("[a]").count(), 1);
        assert_eq!(rendered.matches("[b]").count(), 1);
        assert_eq!(rendered.matches("[c]").count(), 1);
        assert_eq!(rendered.matches("[d]").count(), 1);
    }

    #[test]
    fn uses_orthogonal_segments_without_diagonals() {
        let mut nodes = HashMap::new();
        nodes.insert(
            "model.pkg.top".to_string(),
            NodeEntry {
                resource_type: "model".to_string(),
                name: "top".to_string(),
                package_name: "pkg".to_string(),
            },
        );
        nodes.insert(
            "model.pkg.bottom".to_string(),
            NodeEntry {
                resource_type: "model".to_string(),
                name: "bottom".to_string(),
                package_name: "pkg".to_string(),
            },
        );
        nodes.insert(
            "model.pkg.alt".to_string(),
            NodeEntry {
                resource_type: "model".to_string(),
                name: "alt".to_string(),
                package_name: "pkg".to_string(),
            },
        );
        nodes.insert(
            "model.pkg.target".to_string(),
            NodeEntry {
                resource_type: "model".to_string(),
                name: "target".to_string(),
                package_name: "pkg".to_string(),
            },
        );

        let mut child_map = HashMap::new();
        child_map.insert(
            "model.pkg.top".to_string(),
            vec!["model.pkg.target".to_string(), "model.pkg.alt".to_string()],
        );
        child_map.insert(
            "model.pkg.bottom".to_string(),
            vec!["model.pkg.target".to_string()],
        );

        let mut parent_map = HashMap::new();
        parent_map.insert(
            "model.pkg.alt".to_string(),
            vec!["model.pkg.top".to_string()],
        );
        parent_map.insert(
            "model.pkg.target".to_string(),
            vec!["model.pkg.top".to_string(), "model.pkg.bottom".to_string()],
        );

        let graph = GraphIndex::from_manifest(&Manifest {
            nodes,
            parent_map,
            child_map,
        });

        let rendered = render_all_models(&graph);
        assert!(!rendered.contains('/'));
        assert!(!rendered.contains('\\'));
    }

    #[test]
    fn single_child_edges_avoid_early_diagonal_merging() {
        let mut nodes = HashMap::new();
        for name in ["solo", "via", "target"] {
            nodes.insert(
                format!("model.pkg.{name}"),
                NodeEntry {
                    resource_type: "model".to_string(),
                    name: name.to_string(),
                    package_name: "pkg".to_string(),
                },
            );
        }

        let mut child_map = HashMap::new();
        child_map.insert(
            "model.pkg.solo".to_string(),
            vec!["model.pkg.target".to_string()],
        );
        child_map.insert(
            "model.pkg.via".to_string(),
            vec!["model.pkg.target".to_string()],
        );

        let mut parent_map = HashMap::new();
        parent_map.insert(
            "model.pkg.target".to_string(),
            vec!["model.pkg.solo".to_string(), "model.pkg.via".to_string()],
        );

        let graph = GraphIndex::from_manifest(&Manifest {
            nodes,
            parent_map,
            child_map,
        });

        let rendered = render_all_models(&graph);
        assert!(!rendered.contains("[solo]\\"));
        assert!(!rendered.contains("[solo]/"));
    }

    #[test]
    fn separates_disconnected_components() {
        let mut nodes = HashMap::new();
        for name in ["a", "b", "x", "y"] {
            nodes.insert(
                format!("model.pkg.{name}"),
                NodeEntry {
                    resource_type: "model".to_string(),
                    name: name.to_string(),
                    package_name: "pkg".to_string(),
                },
            );
        }

        let mut child_map = HashMap::new();
        child_map.insert("model.pkg.a".to_string(), vec!["model.pkg.b".to_string()]);
        child_map.insert("model.pkg.x".to_string(), vec!["model.pkg.y".to_string()]);

        let mut parent_map = HashMap::new();
        parent_map.insert("model.pkg.b".to_string(), vec!["model.pkg.a".to_string()]);
        parent_map.insert("model.pkg.y".to_string(), vec!["model.pkg.x".to_string()]);

        let graph = GraphIndex::from_manifest(&Manifest {
            nodes,
            parent_map,
            child_map,
        });

        let rendered = render_all_models(&graph);
        assert!(rendered.contains("[a]"));
        assert!(rendered.contains("[b]"));
        assert!(rendered.contains("[x]"));
        assert!(rendered.contains("[y]"));
        assert!(rendered.contains("\n\n"));
    }
}
