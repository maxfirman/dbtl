use super::layout::Layout;
use crate::graph::GraphIndex;
use std::collections::HashMap;

#[derive(Clone, Copy)]
struct NodePos {
    x: usize,
    y: usize,
    w: usize,
}

pub(super) fn render_ascii_cards(graph: &GraphIndex, layout: &Layout) -> String {
    if layout.layers.is_empty() {
        return String::new();
    }

    let layer_gap = 8usize;
    let base_node_gap = 2usize;

    let layer_widths = layout
        .layers
        .iter()
        .map(|layer| {
            layer
                .iter()
                .map(|id| graph.node_label(id).len() + 2)
                .max()
                .unwrap_or(0)
        })
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

        let stem_end_x = dst_x.saturating_sub(1);
        let stem_start_x = dst_x.saturating_sub(2);
        let turn_x = stem_start_x.saturating_sub(1).max(src_x);
        if src_x < turn_x {
            draw_horizontal(&mut canvas, src_y, src_x, turn_x);
        }
        if src_y != dst_y {
            draw_vertical(&mut canvas, turn_x, src_y.min(dst_y), src_y.max(dst_y));
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
    replace_branch_junctions(&mut canvas);
    enforce_junction_vertical_spacing(&mut canvas);

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

pub(super) fn reinforce_vertical_between_junctions(canvas: &mut [Vec<char>]) {
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

pub(super) fn ensure_arrow_stems(canvas: &mut [Vec<char>]) {
    for row in canvas.iter_mut() {
        for x in 2..row.len() {
            if row[x] == '>' {
                row[x - 1] = '-';
                row[x - 2] = '-';
            }
        }
    }
}

fn replace_branch_junctions(canvas: &mut [Vec<char>]) {
    if canvas.is_empty() || canvas[0].is_empty() {
        return;
    }

    let height = canvas.len();
    let width = canvas[0].len();
    let mut to_replace = Vec::<(usize, usize)>::new();

    for y in 0..height {
        for x in 0..width {
            if canvas[y][x] != '+' {
                continue;
            }
            let mut connections = 0usize;
            if x > 0 && connects_right(canvas[y][x - 1]) {
                connections += 1;
            }
            if x + 1 < width && connects_left(canvas[y][x + 1]) {
                connections += 1;
            }
            if y > 0 && connects_down(canvas[y - 1][x]) {
                connections += 1;
            }
            if y + 1 < height && connects_up(canvas[y + 1][x]) {
                connections += 1;
            }

            // T-junctions are where edges join/diverge.
            // Keep '+' for corners (2-way turns) and 4-way crossings.
            if connections == 3 {
                to_replace.push((x, y));
            }
        }
    }

    for (x, y) in to_replace {
        canvas[y][x] = '•';
    }
}

fn enforce_junction_vertical_spacing(canvas: &mut Vec<Vec<char>>) {
    if canvas.is_empty() || canvas[0].is_empty() {
        return;
    }

    loop {
        let mut inserted = false;
        let height = canvas.len();
        let width = canvas[0].len();

        for y in 0..height.saturating_sub(1) {
            let mut found_adjacent_junction = false;
            for x in 0..width {
                if is_junction(canvas[y][x]) && is_junction(canvas[y + 1][x]) {
                    found_adjacent_junction = true;
                    break;
                }
            }

            if !found_adjacent_junction {
                continue;
            }

            let mut spacer = vec![' '; width];
            for x in 0..width {
                if connects_down(canvas[y][x]) && connects_up(canvas[y + 1][x]) {
                    spacer[x] = '|';
                }
            }
            canvas.insert(y + 1, spacer);
            inserted = true;
            break;
        }

        if !inserted {
            break;
        }
    }
}

fn connects_left(ch: char) -> bool {
    matches!(ch, '-' | '+' | '•' | '>')
}

fn connects_right(ch: char) -> bool {
    matches!(ch, '-' | '+' | '•')
}

fn connects_up(ch: char) -> bool {
    matches!(ch, '|' | '+' | '•')
}

fn connects_down(ch: char) -> bool {
    matches!(ch, '|' | '+' | '•')
}

fn is_junction(ch: char) -> bool {
    matches!(ch, '+' | '•')
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
    branchiness.min(1)
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
    if incoming == '>' || existing == '>' {
        return '>';
    }
    match (existing, incoming) {
        ('|', '-') | ('-', '|') | ('+', '|') | ('|', '+') | ('+', '-') | ('-', '+') => '+',
        _ => existing,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        enforce_junction_vertical_spacing, ensure_arrow_stems,
        reinforce_vertical_between_junctions, replace_branch_junctions,
    };

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

    #[test]
    fn replaces_tee_junction_with_bullet() {
        let mut canvas = vec![vec![' '; 3]; 3];
        canvas[1][0] = '-';
        canvas[1][1] = '+';
        canvas[1][2] = '-';
        canvas[2][1] = '|';

        replace_branch_junctions(&mut canvas);
        assert_eq!(canvas[1][1], '•');
    }

    #[test]
    fn keeps_corner_plus() {
        let mut canvas = vec![vec![' '; 2]; 2];
        canvas[0][0] = '+';
        canvas[0][1] = '-';
        canvas[1][0] = '|';

        replace_branch_junctions(&mut canvas);
        assert_eq!(canvas[0][0], '+');
    }

    #[test]
    fn keeps_crossing_plus() {
        let mut canvas = vec![vec![' '; 3]; 3];
        canvas[1][1] = '+';
        canvas[1][0] = '-';
        canvas[1][2] = '-';
        canvas[0][1] = '|';
        canvas[2][1] = '|';

        replace_branch_junctions(&mut canvas);
        assert_eq!(canvas[1][1], '+');
    }

    #[test]
    fn inserts_spacer_pipe_between_adjacent_junctions() {
        let mut canvas = vec![vec![' '; 1]; 2];
        canvas[0][0] = '+';
        canvas[1][0] = '•';

        enforce_junction_vertical_spacing(&mut canvas);
        assert_eq!(canvas.len(), 3);
        assert_eq!(canvas[0][0], '+');
        assert_eq!(canvas[1][0], '|');
        assert_eq!(canvas[2][0], '•');
    }
}
