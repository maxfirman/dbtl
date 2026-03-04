use crate::graph::GraphIndex;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug)]
pub(super) struct Layout {
    pub(super) layers: Vec<Vec<String>>,
    pub(super) edges: Vec<(String, String)>,
}

pub(super) fn build_layout(graph: &GraphIndex, nodes: &HashSet<String>) -> Layout {
    let mut indegree: HashMap<String, usize> = HashMap::new();
    let mut level: HashMap<String, usize> = HashMap::new();
    let mut edges: Vec<(String, String)> = Vec::new();

    for node in nodes {
        indegree.insert(node.clone(), 0);
        level.insert(node.clone(), 0);
    }

    for parent in nodes {
        for child in graph.children_of(parent) {
            if !nodes.contains(child) {
                continue;
            }
            if let Some(value) = indegree.get_mut(child) {
                *value += 1;
            }
            edges.push((parent.clone(), child.clone()));
        }
    }
    edges.sort_by(|(p1, c1), (p2, c2)| {
        graph
            .node_label(p1)
            .cmp(&graph.node_label(p2))
            .then_with(|| graph.node_label(c1).cmp(&graph.node_label(c2)))
            .then_with(|| p1.cmp(p2))
            .then_with(|| c1.cmp(c2))
    });

    let mut ready = indegree
        .iter()
        .filter_map(|(node, degree)| (*degree == 0).then_some(node.clone()))
        .collect::<Vec<_>>();
    sort_ids(graph, &mut ready);
    let mut ready = VecDeque::from(ready);
    let mut order = Vec::with_capacity(nodes.len());
    let mut ordered_set = HashSet::with_capacity(nodes.len());

    while let Some(current) = ready.pop_front() {
        order.push(current.clone());
        ordered_set.insert(current.clone());
        let next_level = level.get(&current).copied().unwrap_or(0) + 1;

        let child_ids = graph
            .sorted_neighbors(graph.children_of(&current))
            .into_iter()
            .filter(|child| nodes.contains(*child))
            .map(|child| child.to_string())
            .collect::<Vec<_>>();

        let mut pushed = false;
        for child in child_ids {
            if let Some(entry) = level.get_mut(&child) {
                *entry = (*entry).max(next_level);
            }
            if let Some(entry) = indegree.get_mut(&child) {
                *entry = entry.saturating_sub(1);
                if *entry == 0 {
                    ready.push_back(child);
                    pushed = true;
                }
            }
        }
        if pushed {
            let mut ready_sorted = ready.into_iter().collect::<Vec<_>>();
            sort_ids(graph, &mut ready_sorted);
            ready = VecDeque::from(ready_sorted);
        }
    }

    if order.len() < nodes.len() {
        let mut remainder = nodes
            .iter()
            .filter(|id| !ordered_set.contains(*id))
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>();
        sort_ids(graph, &mut remainder);
        order.extend(remainder);
    }

    let max_level = level.values().copied().max().unwrap_or(0);
    let mut layers = vec![Vec::<String>::new(); max_level + 1];
    for node in order {
        let node_level = level.get(&node).copied().unwrap_or(0);
        layers[node_level].push(node);
    }
    for layer_nodes in &mut layers {
        sort_ids(graph, layer_nodes);
    }

    let mut parent_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut child_map: HashMap<String, Vec<String>> = HashMap::new();
    for (from, to) in &edges {
        parent_map.entry(to.clone()).or_default().push(from.clone());
        child_map.entry(from.clone()).or_default().push(to.clone());
    }
    for parents in parent_map.values_mut() {
        sort_ids(graph, parents);
    }
    for children in child_map.values_mut() {
        sort_ids(graph, children);
    }
    reduce_crossings(graph, &mut layers, &parent_map, &child_map, 6);
    refine_with_local_swaps(&mut layers, &edges, &level, 6);

    Layout { layers, edges }
}

pub(super) fn sort_ids(graph: &GraphIndex, ids: &mut [String]) {
    ids.sort_by(|left, right| {
        graph
            .node_label(left)
            .cmp(&graph.node_label(right))
            .then_with(|| left.cmp(right))
    });
}

fn reduce_crossings(
    graph: &GraphIndex,
    layers: &mut [Vec<String>],
    parent_map: &HashMap<String, Vec<String>>,
    child_map: &HashMap<String, Vec<String>>,
    passes: usize,
) {
    if layers.len() < 2 {
        return;
    }

    for _ in 0..passes {
        for layer_index in 1..layers.len() {
            let (before, from_here) = layers.split_at_mut(layer_index);
            let anchor_layer = &before[layer_index - 1];
            let target_layer = &mut from_here[0];
            reorder_layer_by_barycenter(graph, target_layer, anchor_layer, parent_map);
        }
        for layer_index in (0..layers.len() - 1).rev() {
            let (through_target, after) = layers.split_at_mut(layer_index + 1);
            let target_layer = &mut through_target[layer_index];
            let anchor_layer = &after[0];
            reorder_layer_by_barycenter(graph, target_layer, anchor_layer, child_map);
        }
    }
}

fn reorder_layer_by_barycenter(
    graph: &GraphIndex,
    target_layer: &mut [String],
    anchor_layer: &[String],
    adjacent_map: &HashMap<String, Vec<String>>,
) {
    let anchor_positions = anchor_layer
        .iter()
        .enumerate()
        .map(|(idx, node)| (node.as_str(), idx))
        .collect::<HashMap<_, _>>();

    let mut entries = target_layer
        .iter()
        .enumerate()
        .map(|(idx, node)| {
            let barycenter = adjacent_map.get(node).and_then(|neighbors| {
                let coords = neighbors
                    .iter()
                    .filter_map(|neighbor| anchor_positions.get(neighbor.as_str()).copied())
                    .collect::<Vec<_>>();
                if coords.is_empty() {
                    None
                } else {
                    Some(coords.iter().sum::<usize>() as f64 / coords.len() as f64)
                }
            });
            (node.clone(), barycenter, idx)
        })
        .collect::<Vec<_>>();

    entries.sort_by(
        |(left_id, left_bc, left_idx), (right_id, right_bc, right_idx)| match (left_bc, right_bc) {
            (Some(a), Some(b)) => a
                .partial_cmp(b)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| graph.node_label(left_id).cmp(&graph.node_label(right_id)))
                .then_with(|| left_id.cmp(right_id))
                .then_with(|| left_idx.cmp(right_idx)),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => graph
                .node_label(left_id)
                .cmp(&graph.node_label(right_id))
                .then_with(|| left_id.cmp(right_id))
                .then_with(|| left_idx.cmp(right_idx)),
        },
    );

    for (idx, (node, _, _)) in entries.into_iter().enumerate() {
        target_layer[idx] = node;
    }
}

pub(super) fn refine_with_local_swaps(
    layers: &mut [Vec<String>],
    edges: &[(String, String)],
    levels: &HashMap<String, usize>,
    max_passes: usize,
) {
    if layers.len() < 2 {
        return;
    }

    for _ in 0..max_passes {
        let mut improved_any = false;
        for layer_idx in 0..layers.len() {
            if layers[layer_idx].len() < 2 {
                continue;
            }

            let mut improved_layer = true;
            while improved_layer {
                improved_layer = false;
                let mut swap_idx = 0usize;
                while swap_idx + 1 < layers[layer_idx].len() {
                    let before = crossing_score_for_layer_window(layers, edges, levels, layer_idx);
                    layers[layer_idx].swap(swap_idx, swap_idx + 1);
                    let after = crossing_score_for_layer_window(layers, edges, levels, layer_idx);
                    if after < before {
                        improved_layer = true;
                        improved_any = true;
                    } else {
                        layers[layer_idx].swap(swap_idx, swap_idx + 1);
                    }
                    swap_idx += 1;
                }
            }

            let layer_score_before =
                crossing_score_for_layer_window(layers, edges, levels, layer_idx);
            let layer_len = layers[layer_idx].len();
            for original_idx in 0..layer_len {
                if original_idx >= layers[layer_idx].len() {
                    break;
                }

                let node = layers[layer_idx].remove(original_idx);
                let mut best_pos = 0usize;
                let mut best_score = usize::MAX;

                for candidate_pos in 0..=layers[layer_idx].len() {
                    layers[layer_idx].insert(candidate_pos, node.clone());
                    let score = crossing_score_for_layer_window(layers, edges, levels, layer_idx);
                    if score < best_score {
                        best_score = score;
                        best_pos = candidate_pos;
                    }
                    layers[layer_idx].remove(candidate_pos);
                }

                layers[layer_idx].insert(best_pos, node);
            }
            let layer_score_after =
                crossing_score_for_layer_window(layers, edges, levels, layer_idx);
            if layer_score_after < layer_score_before {
                improved_any = true;
            }
        }
        if !improved_any {
            break;
        }
    }
}

fn crossing_score_for_layer_window(
    layers: &[Vec<String>],
    edges: &[(String, String)],
    levels: &HashMap<String, usize>,
    layer_idx: usize,
) -> usize {
    let node_positions = node_positions(layers);
    let mut total = 0usize;
    if layer_idx > 0 {
        total += count_crossings_between_layers(
            edges,
            levels,
            &node_positions,
            layer_idx - 1,
            layer_idx,
        );
    }
    if layer_idx + 1 < layers.len() {
        total += count_crossings_between_layers(
            edges,
            levels,
            &node_positions,
            layer_idx,
            layer_idx + 1,
        );
    }
    total
}

fn node_positions(layers: &[Vec<String>]) -> HashMap<&str, usize> {
    let mut node_positions = HashMap::<&str, usize>::new();
    for layer in layers {
        for (pos, node) in layer.iter().enumerate() {
            node_positions.insert(node.as_str(), pos);
        }
    }
    node_positions
}

pub(super) fn count_crossings_between_layers(
    edges: &[(String, String)],
    levels: &HashMap<String, usize>,
    node_positions: &HashMap<&str, usize>,
    left_layer_idx: usize,
    right_layer_idx: usize,
) -> usize {
    if right_layer_idx != left_layer_idx + 1 {
        return 0;
    }

    let mut layer_edges = Vec::<(f64, f64)>::new();
    for (from, to) in edges {
        let Some(&from_level) = levels.get(from.as_str()) else {
            continue;
        };
        let Some(&to_level) = levels.get(to.as_str()) else {
            continue;
        };
        if from_level >= to_level {
            continue;
        }
        if from_level > left_layer_idx || to_level < right_layer_idx {
            continue;
        }
        let Some(&from_pos) = node_positions.get(from.as_str()) else {
            continue;
        };
        let Some(&to_pos) = node_positions.get(to.as_str()) else {
            continue;
        };
        let span = (to_level - from_level) as f64;
        let slope = (to_pos as f64 - from_pos as f64) / span;
        let left_y = from_pos as f64 + slope * (left_layer_idx as f64 - from_level as f64);
        let right_y = from_pos as f64 + slope * (right_layer_idx as f64 - from_level as f64);
        layer_edges.push((left_y, right_y));
    }

    let mut crossings = 0usize;
    for i in 0..layer_edges.len() {
        for j in i + 1..layer_edges.len() {
            let (a_left, a_right) = layer_edges[i];
            let (b_left, b_right) = layer_edges[j];
            if (a_left < b_left && a_right > b_right) || (a_left > b_left && a_right < b_right) {
                crossings += 1;
            }
        }
    }
    crossings
}

#[cfg(test)]
mod tests {
    use super::{count_crossings_between_layers, node_positions, refine_with_local_swaps};
    use std::collections::HashMap;

    #[test]
    fn local_swap_refinement_reduces_crossings() {
        let layers = vec![
            vec!["a".to_string(), "b".to_string()],
            vec!["c".to_string(), "d".to_string()],
        ];
        let edges = vec![
            ("a".to_string(), "d".to_string()),
            ("b".to_string(), "c".to_string()),
        ];
        let levels = HashMap::from([
            ("a".to_string(), 0usize),
            ("b".to_string(), 0usize),
            ("c".to_string(), 1usize),
            ("d".to_string(), 1usize),
        ]);

        let before =
            count_crossings_between_layers(&edges, &levels, &node_positions(&layers), 0, 1);
        let mut optimized_layers = layers.clone();
        refine_with_local_swaps(&mut optimized_layers, &edges, &levels, 4);
        let after = count_crossings_between_layers(
            &edges,
            &levels,
            &node_positions(&optimized_layers),
            0,
            1,
        );

        assert_eq!(before, 1);
        assert_eq!(after, 0);
    }
}
