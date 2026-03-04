use crate::{error::AppError, graph::GraphIndex};
use std::collections::HashSet;

const INVALID_SELECTOR_MSG: &str = "Invalid selector syntax";

#[derive(Debug, Clone, PartialEq, Eq)]
enum BaseSelector {
    Bare(String),
    Tag(String),
    Fqn(String),
    Path(String),
    Config {
        key_path: Vec<String>,
        expected: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GraphOps {
    at: bool,
    ancestor_depth: Option<usize>,
    descendant_depth: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AtomicSelector {
    graph: GraphOps,
    base: BaseSelector,
}

pub fn resolve_selectors(
    graph: &GraphIndex,
    raw_selectors: &[String],
) -> Result<HashSet<String>, AppError> {
    let mut union_result = HashSet::<String>::new();

    for raw_group in raw_selectors {
        let terms = raw_group.split(',').map(str::trim).collect::<Vec<_>>();
        if terms.is_empty() || terms.iter().any(|t| t.is_empty()) {
            return Err(AppError::usage(format!(
                "{INVALID_SELECTOR_MSG}: intersection groups cannot be empty"
            )));
        }

        let mut group_result: Option<HashSet<String>> = None;
        for term in terms {
            let parsed = parse_atomic_selector(term)?;
            let selected = evaluate_atomic_selector(graph, &parsed)?;
            group_result = Some(match group_result {
                None => selected,
                Some(current) => current.intersection(&selected).cloned().collect(),
            });
        }

        union_result.extend(group_result.unwrap_or_default());
    }

    Ok(union_result)
}

fn parse_atomic_selector(raw: &str) -> Result<AtomicSelector, AppError> {
    if raw.is_empty() {
        return Err(AppError::usage(format!(
            "{INVALID_SELECTOR_MSG}: empty selector"
        )));
    }

    let mut remaining = raw;
    let mut at = false;
    if let Some(stripped) = remaining.strip_prefix('@') {
        at = true;
        remaining = stripped;
    }

    let (ancestor_depth, after_prefix) = parse_prefix_plus(remaining)?;
    let (descendant_depth, core) = parse_suffix_plus(after_prefix)?;

    if core.is_empty() {
        return Err(AppError::usage(format!(
            "{INVALID_SELECTOR_MSG}: missing selector body"
        )));
    }

    let base = parse_base_selector(&core)?;

    Ok(AtomicSelector {
        graph: GraphOps {
            at,
            ancestor_depth,
            descendant_depth,
        },
        base,
    })
}

fn parse_prefix_plus(raw: &str) -> Result<(Option<usize>, &str), AppError> {
    if let Some(stripped) = raw.strip_prefix('+') {
        return Ok((Some(usize::MAX), stripped));
    }

    let digit_count = raw.chars().take_while(|c| c.is_ascii_digit()).count();
    if digit_count == 0 {
        return Ok((None, raw));
    }

    if raw.chars().nth(digit_count) != Some('+') {
        return Err(AppError::usage(format!(
            "{INVALID_SELECTOR_MSG}: invalid prefix graph operator"
        )));
    }

    let depth = parse_positive_depth(&raw[..digit_count])?;
    Ok((Some(depth), &raw[digit_count + 1..]))
}

fn parse_suffix_plus(raw: &str) -> Result<(Option<usize>, String), AppError> {
    let Some(idx) = raw.rfind('+') else {
        return Ok((None, raw.to_string()));
    };

    let model = &raw[..idx];
    let suffix = &raw[idx + 1..];
    if model.contains('+') {
        return Err(AppError::usage(format!(
            "{INVALID_SELECTOR_MSG}: multiple suffix graph operators"
        )));
    }

    if suffix.is_empty() {
        return Ok((Some(usize::MAX), model.to_string()));
    }
    if !suffix.chars().all(|c| c.is_ascii_digit()) {
        return Err(AppError::usage(format!(
            "{INVALID_SELECTOR_MSG}: invalid suffix depth"
        )));
    }

    let depth = parse_positive_depth(suffix)?;
    Ok((Some(depth), model.to_string()))
}

fn parse_positive_depth(raw: &str) -> Result<usize, AppError> {
    let parsed = raw
        .parse::<usize>()
        .map_err(|_| AppError::usage("Invalid selector depth. Depth must be a positive integer"))?;
    if parsed == 0 {
        return Err(AppError::usage(
            "Invalid selector depth. Depth must be >= 1",
        ));
    }
    Ok(parsed)
}

fn parse_base_selector(raw: &str) -> Result<BaseSelector, AppError> {
    let Some((prefix, value)) = raw.split_once(':') else {
        return Ok(BaseSelector::Bare(raw.to_string()));
    };

    if value.is_empty() {
        return Err(AppError::usage(format!(
            "{INVALID_SELECTOR_MSG}: missing method value"
        )));
    }

    match prefix {
        "tag" => Ok(BaseSelector::Tag(value.to_string())),
        "fqn" => Ok(BaseSelector::Fqn(value.to_string())),
        "path" => Ok(BaseSelector::Path(value.to_string())),
        _ if prefix.starts_with("config.") => {
            let key_path = prefix
                .split('.')
                .skip(1)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>();
            if key_path.is_empty() {
                return Err(AppError::usage(format!(
                    "{INVALID_SELECTOR_MSG}: missing config key path"
                )));
            }
            Ok(BaseSelector::Config {
                key_path,
                expected: value.to_string(),
            })
        }
        _ => Err(AppError::usage(format!(
            "{INVALID_SELECTOR_MSG}: unsupported selector method '{prefix}'"
        ))),
    }
}

fn evaluate_atomic_selector(
    graph: &GraphIndex,
    selector: &AtomicSelector,
) -> Result<HashSet<String>, AppError> {
    let base = evaluate_base_selector(graph, &selector.base)?;
    let mut selected = base.clone();

    if let Some(depth) = selector.graph.ancestor_depth {
        selected.extend(graph.expand_ancestors(&base, depth));
    }
    if let Some(depth) = selector.graph.descendant_depth {
        selected.extend(graph.expand_descendants(&base, depth));
    }

    if selector.graph.at {
        let descendants = graph.expand_descendants(&base, usize::MAX);
        let needed_ancestors = graph.expand_ancestors(&descendants, usize::MAX);
        selected.extend(descendants);
        selected.extend(needed_ancestors);
    }

    Ok(selected)
}

fn evaluate_base_selector(
    graph: &GraphIndex,
    selector: &BaseSelector,
) -> Result<HashSet<String>, AppError> {
    match selector {
        BaseSelector::Tag(pattern) => Ok(graph.select_by_tag_pattern(pattern)),
        BaseSelector::Fqn(pattern) => Ok(graph.select_by_fqn_pattern(pattern)),
        BaseSelector::Path(pattern) => Ok(graph.select_by_path_pattern(pattern)),
        BaseSelector::Config { key_path, expected } => {
            Ok(graph.select_by_config_value(key_path, expected))
        }
        BaseSelector::Bare(raw) => evaluate_bare_selector(graph, raw),
    }
}

fn evaluate_bare_selector(graph: &GraphIndex, raw: &str) -> Result<HashSet<String>, AppError> {
    if is_plain_model_name(raw) {
        match graph.resolve_model(raw) {
            Ok(id) => {
                let mut selected = HashSet::new();
                selected.insert(id.to_string());
                return Ok(selected);
            }
            Err(AppError::ModelNotFound { .. }) => {
                return Err(AppError::ModelNotFound {
                    model_name: raw.to_string(),
                });
            }
            Err(other) => return Err(other),
        }
    }

    let mut selected = graph.select_by_fqn_pattern(raw);
    selected.extend(graph.select_by_path_pattern(raw));
    selected.extend(graph.select_by_name_pattern(raw));
    Ok(selected)
}

fn is_plain_model_name(raw: &str) -> bool {
    !raw.contains('*')
        && !raw.contains('?')
        && !raw.contains('/')
        && !raw.contains('.')
        && raw
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

#[cfg(test)]
mod tests {
    use super::{parse_atomic_selector, resolve_selectors};
    use crate::{
        graph::GraphIndex,
        manifest::{Manifest, NodeEntry},
    };
    use std::collections::{HashMap, HashSet};

    fn node(
        name: &str,
        pkg: &str,
        fqn: &[&str],
        tags: &[&str],
        path: &str,
        config: serde_json::Value,
    ) -> NodeEntry {
        NodeEntry {
            resource_type: "model".to_string(),
            name: name.to_string(),
            package_name: pkg.to_string(),
            fqn: fqn.iter().map(|s| s.to_string()).collect(),
            tags: tags.iter().map(|s| s.to_string()).collect(),
            original_file_path: path.to_string(),
            config,
        }
    }

    fn graph_fixture() -> GraphIndex {
        let mut nodes = HashMap::new();
        nodes.insert(
            "model.pkg.root".to_string(),
            node(
                "root",
                "pkg",
                &["pkg", "staging", "root"],
                &["nightly"],
                "models/staging/root.sql",
                serde_json::json!({"materialized":"view","meta":{"contains_pii":false}}),
            ),
        );
        nodes.insert(
            "model.pkg.mid".to_string(),
            node(
                "mid",
                "pkg",
                &["pkg", "marts", "mid"],
                &["nightly", "finance"],
                "models/marts/mid.sql",
                serde_json::json!({"materialized":"table","meta":{"contains_pii":true},"tags":["finance"]}),
            ),
        );
        nodes.insert(
            "model.pkg.leaf".to_string(),
            node(
                "leaf",
                "pkg",
                &["pkg", "marts", "leaf"],
                &["daily"],
                "models/marts/leaf.sql",
                serde_json::json!({"materialized":"incremental"}),
            ),
        );
        nodes.insert(
            "model.pkg.other".to_string(),
            node(
                "other",
                "pkg",
                &["pkg", "other", "other"],
                &["daily"],
                "models/other/other.sql",
                serde_json::json!({"materialized":"view"}),
            ),
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

    fn set(items: &[&str]) -> HashSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parses_graph_operators_and_methods() {
        let parsed = parse_atomic_selector("@1+tag:nightly+2").expect("selector should parse");
        assert!(parsed.graph.at);
        assert_eq!(parsed.graph.ancestor_depth, Some(1));
        assert_eq!(parsed.graph.descendant_depth, Some(2));
    }

    #[test]
    fn tag_selector_and_union_work() {
        let graph = graph_fixture();
        let selected = resolve_selectors(
            &graph,
            &["tag:nightly".to_string(), "tag:daily".to_string()],
        )
        .expect("selectors should resolve");
        assert_eq!(
            selected,
            set(&[
                "model.pkg.root",
                "model.pkg.mid",
                "model.pkg.leaf",
                "model.pkg.other"
            ])
        );
    }

    #[test]
    fn comma_intersection_works() {
        let graph = graph_fixture();
        let selected = resolve_selectors(
            &graph,
            &["tag:nightly,config.materialized:table".to_string()],
        )
        .expect("selectors should resolve");
        assert_eq!(selected, set(&["model.pkg.mid"]));
    }

    #[test]
    fn fqn_selector_works() {
        let graph = graph_fixture();
        let selected = resolve_selectors(&graph, &["fqn:pkg.marts.*".to_string()])
            .expect("selectors should resolve");
        assert_eq!(selected, set(&["model.pkg.mid", "model.pkg.leaf"]));
    }

    #[test]
    fn path_selector_works_for_prefix() {
        let graph = graph_fixture();
        let selected = resolve_selectors(&graph, &["path:models/marts".to_string()])
            .expect("selectors should resolve");
        assert_eq!(selected, set(&["model.pkg.mid", "model.pkg.leaf"]));
    }

    #[test]
    fn config_selector_supports_nested_values() {
        let graph = graph_fixture();
        let selected = resolve_selectors(&graph, &["config.meta.contains_pii:true".to_string()])
            .expect("selectors should resolve");
        assert_eq!(selected, set(&["model.pkg.mid"]));
    }

    #[test]
    fn plus_graph_operators_work() {
        let graph = graph_fixture();
        let selected =
            resolve_selectors(&graph, &["+mid+".to_string()]).expect("selectors should resolve");
        assert_eq!(
            selected,
            set(&["model.pkg.root", "model.pkg.mid", "model.pkg.leaf"])
        );
    }

    #[test]
    fn at_graph_operator_adds_descendants_and_needed_ancestors() {
        let graph = graph_fixture();
        let selected =
            resolve_selectors(&graph, &["@root".to_string()]).expect("selectors should resolve");
        assert_eq!(
            selected,
            set(&["model.pkg.root", "model.pkg.mid", "model.pkg.leaf"])
        );
    }

    #[test]
    fn intersection_with_empty_term_fails() {
        let graph = graph_fixture();
        let err = resolve_selectors(&graph, &["tag:nightly,".to_string()])
            .expect_err("selector should fail");
        assert!(err.to_string().contains("cannot be empty"));
    }
}
