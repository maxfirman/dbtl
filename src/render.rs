use crate::{cli::SelectorSpec, graph::GraphIndex};
use std::collections::HashMap;
use std::collections::HashSet;

pub fn render_selection(graph: &GraphIndex, root_id: &str, selector: &SelectorSpec) -> String {
    let nodes = collect_selected_nodes(graph, root_id, selector);
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

#[derive(Debug)]
struct Layout {
    layers: Vec<Vec<String>>,
    edges: Vec<(String, String)>,
}

#[derive(Clone, Copy)]
struct NodePos {
    x: usize,
    y: usize,
    w: usize,
}

fn collect_selected_nodes(graph: &GraphIndex, root_id: &str, selector: &SelectorSpec) -> HashSet<String> {
    let mut nodes = HashSet::new();
    nodes.insert(root_id.to_string());

    if selector.include_ancestors {
        collect_reachable(graph, root_id, Direction::Up, &mut nodes);
    }
    if selector.include_descendants {
        collect_reachable(graph, root_id, Direction::Down, &mut nodes);
    }

    nodes
}

fn collect_reachable(graph: &GraphIndex, start: &str, direction: Direction, out: &mut HashSet<String>) {
    let mut stack = vec![start.to_string()];
    while let Some(current) = stack.pop() {
        let neighbors = match direction {
            Direction::Up => graph.parents_of(&current),
            Direction::Down => graph.children_of(&current),
        };
        for neighbor in neighbors {
            if out.insert(neighbor.clone()) {
                stack.push(neighbor.clone());
            }
        }
    }
}

fn render_components(graph: &GraphIndex, nodes: &HashSet<String>) -> String {
    let components = connected_components(graph, nodes);
    components
        .into_iter()
        .map(|component| {
            let layout = build_layout(graph, &component);
            render_ascii_cards(graph, &layout)
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
    sort_ids(graph, &mut ids);
    ids.first()
        .map(|id| graph.node_label(id))
        .unwrap_or_default()
}

fn build_layout(graph: &GraphIndex, nodes: &HashSet<String>) -> Layout {
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
    let mut order = Vec::new();

    while let Some(current) = pop_first(&mut ready) {
        order.push(current.clone());
        let next_level = level.get(&current).copied().unwrap_or(0) + 1;

        let child_ids = graph
            .sorted_neighbors(graph.children_of(&current))
            .into_iter()
            .filter(|child| nodes.contains(*child))
            .map(|child| child.to_string())
            .collect::<Vec<_>>();

        for child in child_ids {
            if let Some(entry) = level.get_mut(&child) {
                *entry = (*entry).max(next_level);
            }
            if let Some(entry) = indegree.get_mut(&child) {
                *entry = entry.saturating_sub(1);
                if *entry == 0 {
                    ready.push(child);
                }
            }
        }
        sort_ids(graph, &mut ready);
    }

    if order.len() < nodes.len() {
        let mut remainder = nodes
            .iter()
            .filter(|id| !order.contains(*id))
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

fn render_ascii_cards(graph: &GraphIndex, layout: &Layout) -> String {
    if layout.layers.is_empty() {
        return String::new();
    }

    let layer_gap = 8usize;
    let base_node_gap = 2usize;

    let layer_widths = layout
        .layers
        .iter()
        .map(|layer| layer.iter().map(|id| graph.node_label(id).len() + 2).max().unwrap_or(0))
        .collect::<Vec<_>>();
    let (out_degree, in_degree) = degree_maps(&layout.edges);
    let layer_heights = layout
        .layers
        .iter()
        .map(|layer| compute_layer_height(layer, base_node_gap, &out_degree, &in_degree))
        .collect::<Vec<_>>();
    let max_height = layer_heights.iter().copied().max().unwrap_or(1);

    let mut positions: HashMap<String, NodePos> = HashMap::new();
    let mut max_x = 0usize;
    let mut max_y = 0usize;
    let mut x = 0usize;

    for (layer_idx, layer) in layout.layers.iter().enumerate() {
        if layer.is_empty() {
            x += layer_widths[layer_idx] + layer_gap;
            continue;
        }
        let mut y = (max_height.saturating_sub(layer_heights[layer_idx])) / 2;
        // Keep rows aligned on even coordinates so major junctions are less likely
        // to appear on adjacent lines; this improves visible vertical trunks.
        if y % 2 != 0 {
            y = y.saturating_add(1);
        }
        for (idx, node_id) in layer.iter().enumerate() {
            let label = graph.node_label(node_id);
            let w = label.len() + 2;
            positions.insert(node_id.clone(), NodePos { x, y, w });
            max_x = max_x.max(x + w);
            max_y = max_y.max(y);
            if idx + 1 < layer.len() {
                let current_bonus = node_spacing_bonus(node_id, &out_degree, &in_degree);
                let next_bonus = node_spacing_bonus(&layer[idx + 1], &out_degree, &in_degree);
                y += base_node_gap + current_bonus.max(next_bonus);
            }
        }
        x += layer_widths[layer_idx] + layer_gap;
    }

    let height = max_y + 1;
    let width = max_x.max(1);
    let mut canvas = vec![vec![' '; width]; height];

    for (node_id, pos) in &positions {
        let label = format!("[{}]", graph.node_label(node_id));
        draw_text(&mut canvas, pos.x, pos.y, &label);
    }

    for (from, to) in &layout.edges {
        let Some(src) = positions.get(from) else {
            continue;
        };
        let Some(dst) = positions.get(to) else {
            continue;
        };

        if dst.x <= src.x {
            continue;
        }

        let src_x = src.x + src.w;
        let dst_x = dst.x.saturating_sub(1);
        let src_y = src.y;
        let dst_y = dst.y;
        let diag_end_x = src_x;
        let diag_end_y = src_y;

        // Keep each edge on its own horizontal track as long as possible,
        // then turn near the destination to reduce early branch bundling.
        // Reserve two characters before the arrowhead for a required '--' stem.
        let stem_end_x = dst_x.saturating_sub(1);
        let stem_start_x = dst_x.saturating_sub(2);
        let turn_x = stem_start_x.saturating_sub(1).max(diag_end_x);
        if diag_end_x < turn_x {
            draw_horizontal(&mut canvas, diag_end_y, diag_end_x, turn_x);
        }
        if diag_end_y != dst_y {
            draw_vertical(&mut canvas, turn_x, diag_end_y.min(dst_y), diag_end_y.max(dst_y));
        }
        if turn_x < stem_start_x {
            draw_horizontal(&mut canvas, dst_y, turn_x, stem_start_x);
        }
        put_char(&mut canvas, stem_start_x, dst_y, '-');
        put_char(&mut canvas, stem_end_x, dst_y, '-');
        put_char(&mut canvas, dst_x, dst_y, '>');
    }

    reinforce_vertical_between_junctions(&mut canvas);
    ensure_arrow_stems(&mut canvas);

    canvas
        .into_iter()
        .map(|row| {
            let mut line = row.into_iter().collect::<String>();
            while line.ends_with(' ') {
                line.pop();
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn reinforce_vertical_between_junctions(canvas: &mut [Vec<char>]) {
    if canvas.is_empty() || canvas[0].is_empty() {
        return;
    }

    let height = canvas.len();
    let width = canvas[0].len();

    for x in 0..width {
        let plus_rows = (0..height)
            .filter(|&y| canvas[y][x] == '+')
            .collect::<Vec<_>>();

        for pair in plus_rows.windows(2) {
            let y1 = pair[0];
            let y2 = pair[1];
            if y2 <= y1 + 1 {
                continue;
            }
            let has_pipe = (y1 + 1..y2).any(|y| canvas[y][x] == '|');
            if has_pipe {
                continue;
            }
            for y in y1 + 1..y2 {
                if canvas[y][x] == ' ' {
                    put_char(canvas, x, y, '|');
                }
            }
        }
    }
}

fn ensure_arrow_stems(canvas: &mut [Vec<char>]) {
    for row in canvas.iter_mut() {
        for x in 2..row.len() {
            if row[x] == '>' {
                row[x - 1] = '-';
                row[x - 2] = '-';
            }
        }
    }
}

fn degree_maps(edges: &[(String, String)]) -> (HashMap<String, usize>, HashMap<String, usize>) {
    let mut out_degree = HashMap::<String, usize>::new();
    let mut in_degree = HashMap::<String, usize>::new();
    for (from, to) in edges {
        *out_degree.entry(from.clone()).or_insert(0) += 1;
        *in_degree.entry(to.clone()).or_insert(0) += 1;
    }
    (out_degree, in_degree)
}

fn node_spacing_bonus(
    node_id: &str,
    out_degree: &HashMap<String, usize>,
    in_degree: &HashMap<String, usize>,
) -> usize {
    let fan_out = out_degree.get(node_id).copied().unwrap_or(0);
    let fan_in = in_degree.get(node_id).copied().unwrap_or(0);
    let branchiness = fan_out.max(fan_in).saturating_sub(1);
    (branchiness * 2).min(6)
}

fn compute_layer_height(
    layer: &[String],
    base_node_gap: usize,
    out_degree: &HashMap<String, usize>,
    in_degree: &HashMap<String, usize>,
) -> usize {
    if layer.is_empty() {
        return 0;
    }
    let mut y = 0usize;
    for idx in 0..(layer.len() - 1) {
        let current_bonus = node_spacing_bonus(&layer[idx], out_degree, in_degree);
        let next_bonus = node_spacing_bonus(&layer[idx + 1], out_degree, in_degree);
        y += base_node_gap + current_bonus.max(next_bonus);
    }
    y + 1
}

fn draw_text(canvas: &mut [Vec<char>], x: usize, y: usize, text: &str) {
    for (idx, ch) in text.chars().enumerate() {
        put_char(canvas, x + idx, y, ch);
    }
}

fn draw_vertical(canvas: &mut [Vec<char>], x: usize, y_start: usize, y_end: usize) {
    for y in y_start..=y_end {
        put_char(canvas, x, y, '|');
    }
}

fn draw_horizontal(canvas: &mut [Vec<char>], y: usize, x1: usize, x2: usize) {
    let (start, end) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
    for x in start..=end {
        put_char(canvas, x, y, '-');
    }
}

fn put_char(canvas: &mut [Vec<char>], x: usize, y: usize, ch: char) {
    if y >= canvas.len() || x >= canvas[y].len() {
        return;
    }
    let existing = canvas[y][x];
    canvas[y][x] = merge_chars(existing, ch);
}

fn merge_chars(existing: char, incoming: char) -> char {
    if existing == ' ' || existing == incoming {
        return incoming;
    }
    if incoming == '>' {
        return '>';
    }
    if existing == '>' {
        return '>';
    }
    match (existing, incoming) {
        ('|', '-') | ('-', '|') | ('+', '|') | ('|', '+') | ('+', '-') | ('-', '+') => '+',
        _ => existing,
    }
}

fn sort_ids(graph: &GraphIndex, ids: &mut [String]) {
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

    entries.sort_by(|(left_id, left_bc, left_idx), (right_id, right_bc, right_idx)| {
        match (left_bc, right_bc) {
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
        }
    });

    for (idx, (node, _, _)) in entries.into_iter().enumerate() {
        target_layer[idx] = node;
    }
}

fn refine_with_local_swaps(
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

            // Second phase: sift each node through all positions in its layer
            // and keep the placement with the lowest local crossing score.
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
                    let score =
                        crossing_score_for_layer_window(layers, edges, levels, layer_idx);
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
    let mut total = 0usize;
    if layer_idx > 0 {
        total += count_crossings_between_layers(layers, edges, levels, layer_idx - 1, layer_idx);
    }
    if layer_idx + 1 < layers.len() {
        total += count_crossings_between_layers(layers, edges, levels, layer_idx, layer_idx + 1);
    }
    total
}

fn count_crossings_between_layers(
    layers: &[Vec<String>],
    edges: &[(String, String)],
    levels: &HashMap<String, usize>,
    left_layer_idx: usize,
    right_layer_idx: usize,
) -> usize {
    if right_layer_idx != left_layer_idx + 1 {
        return 0;
    }
    if left_layer_idx >= layers.len() || right_layer_idx >= layers.len() {
        return 0;
    }

    let mut node_positions = HashMap::<&str, usize>::new();
    for layer in layers {
        for (pos, node) in layer.iter().enumerate() {
            node_positions.insert(node.as_str(), pos);
        }
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

fn pop_first<T>(items: &mut Vec<T>) -> Option<T> {
    if items.is_empty() {
        None
    } else {
        Some(items.remove(0))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        count_crossings_between_layers, ensure_arrow_stems, refine_with_local_swaps,
        reinforce_vertical_between_junctions, render_all_models, render_selection,
    };
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
        parent_map.insert("model.pkg.mid".to_string(), vec!["model.pkg.root".to_string()]);
        parent_map.insert("model.pkg.leaf".to_string(), vec!["model.pkg.mid".to_string()]);

        let mut child_map = HashMap::new();
        child_map.insert("model.pkg.root".to_string(), vec!["model.pkg.mid".to_string()]);
        child_map.insert("model.pkg.mid".to_string(), vec!["model.pkg.leaf".to_string()]);

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
            include_ancestors: false,
            include_descendants: false,
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
            include_ancestors: false,
            include_descendants: true,
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
            include_ancestors: true,
            include_descendants: false,
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
            include_ancestors: true,
            include_descendants: true,
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
            include_ancestors: false,
            include_descendants: true,
            model_name: "a".to_string(),
        };

        let rendered = render_selection(&graph, "model.pkg.a", &selector);
        assert_eq!(rendered.matches("[a]").count(), 1);
        assert_eq!(rendered.matches("[b]").count(), 1);
        assert_eq!(rendered.matches("[c]").count(), 1);
        assert_eq!(rendered.matches("[d]").count(), 1);
    }

    #[test]
    fn reduces_crossings_with_barycenter_ordering() {
        let mut nodes = HashMap::new();
        for name in ["a", "b", "c", "d"] {
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
        child_map.insert("model.pkg.a".to_string(), vec!["model.pkg.d".to_string()]);
        child_map.insert("model.pkg.b".to_string(), vec!["model.pkg.c".to_string()]);

        let mut parent_map = HashMap::new();
        parent_map.insert("model.pkg.c".to_string(), vec!["model.pkg.b".to_string()]);
        parent_map.insert("model.pkg.d".to_string(), vec!["model.pkg.a".to_string()]);

        let graph = GraphIndex::from_manifest(&Manifest {
            nodes,
            parent_map,
            child_map,
        });

        let rendered = render_all_models(&graph);
        let d_line = rendered
            .lines()
            .position(|line| line.contains("[d]"))
            .expect("line with [d] should exist");
        let c_line = rendered
            .lines()
            .position(|line| line.contains("[c]"))
            .expect("line with [c] should exist");
        assert!(d_line < c_line);
    }

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

        let before = count_crossings_between_layers(&layers, &edges, &levels, 0, 1);
        let mut optimized_layers = layers.clone();
        refine_with_local_swaps(&mut optimized_layers, &edges, &levels, 4);
        let after = count_crossings_between_layers(&optimized_layers, &edges, &levels, 0, 1);

        assert_eq!(before, 1);
        assert_eq!(after, 0);
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
        parent_map.insert("model.pkg.alt".to_string(), vec!["model.pkg.top".to_string()]);
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
        child_map.insert("model.pkg.solo".to_string(), vec!["model.pkg.target".to_string()]);
        child_map.insert("model.pkg.via".to_string(), vec!["model.pkg.target".to_string()]);

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

    #[test]
    fn inserts_pipe_between_vertical_junctions() {
        let mut canvas = vec![vec![' '; 1]; 3];
        canvas[0][0] = '+';
        canvas[2][0] = '+';

        reinforce_vertical_between_junctions(&mut canvas);

        assert_eq!(canvas[1][0], '|');
    }

    #[test]
    fn keeps_middle_plus_in_tight_vertical_stack() {
        let mut canvas = vec![vec![' '; 1]; 3];
        canvas[0][0] = '+';
        canvas[1][0] = '+';
        canvas[2][0] = '+';

        reinforce_vertical_between_junctions(&mut canvas);

        assert_eq!(canvas[0][0], '+');
        assert_eq!(canvas[1][0], '+');
        assert_eq!(canvas[2][0], '+');
    }

    #[test]
    fn enforces_two_dashes_before_arrowhead() {
        let mut canvas = vec![vec![' '; 4]; 1];
        canvas[0][3] = '>';
        ensure_arrow_stems(&mut canvas);
        assert_eq!(canvas[0][2], '-');
        assert_eq!(canvas[0][1], '-');
    }
}
