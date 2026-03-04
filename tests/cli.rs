use assert_cmd::Command;
use predicates::prelude::*;
use std::collections::BTreeSet;
use std::{fs, path::Path};
use tempfile::TempDir;

fn setup_target_path(target_name: &str) -> TempDir {
    let temp = TempDir::new().expect("temp dir should be created");
    let target_path = temp.path().join(target_name);
    fs::create_dir_all(&target_path).expect("target path should be created");
    copy_fixture_manifest(&target_path.join("manifest.json"));
    temp
}

fn setup_target_path_with_manifest(target_name: &str, manifest_json: &str) -> TempDir {
    let temp = TempDir::new().expect("temp dir should be created");
    let target_path = temp.path().join(target_name);
    fs::create_dir_all(&target_path).expect("target path should be created");
    fs::write(target_path.join("manifest.json"), manifest_json)
        .expect("manifest should be written");
    temp
}

fn copy_fixture_manifest(dest: &Path) {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("manifest.json");
    fs::copy(fixture, dest).expect("fixture manifest should be copied");
}

fn binary_cmd() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("dbtl"))
}

fn extracted_nodes(output: &str) -> BTreeSet<String> {
    let mut nodes = BTreeSet::new();
    let bytes = output.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'[' {
            let start = i + 1;
            if let Some(end_rel) = bytes[start..].iter().position(|b| *b == b']') {
                let end = start + end_rel;
                if let Ok(label) = std::str::from_utf8(&bytes[start..end])
                    && !label.is_empty()
                {
                    nodes.insert(label.to_string());
                }
                i = end + 1;
                continue;
            }
        }
        i += 1;
    }
    nodes
}

fn assert_selected_nodes(args: &[&str], expected: &[&str]) {
    let temp = setup_target_path("target");
    let assert = binary_cmd()
        .current_dir(temp.path())
        .args(args)
        .assert()
        .success();
    let output =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout should be utf-8");

    let expected_nodes = expected
        .iter()
        .map(|s| s.to_string())
        .collect::<BTreeSet<_>>();
    assert_eq!(extracted_nodes(&output), expected_nodes);
}

#[test]
fn help_flag_exits_successfully() {
    binary_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: dbtl"))
        .stdout(predicate::str::contains("self"));
}

#[test]
fn self_update_help_exits_successfully() {
    binary_cmd()
        .args(["self", "update", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: dbtl self update"));
}

#[test]
fn prints_all_models_when_select_is_omitted() {
    let temp = setup_target_path("target");
    let assert = binary_cmd().current_dir(temp.path()).assert().success();
    let output =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout should be utf-8");

    assert_eq!(
        extracted_nodes(&output),
        BTreeSet::from([
            "child_a".to_string(),
            "child_b".to_string(),
            "grandchild".to_string(),
            "my_model".to_string(),
        ])
    );
    insta::assert_snapshot!("render_all_models_fixture", output);
}

#[test]
fn target_path_flag_overrides_default_target() {
    let temp = setup_target_path("custom_target");
    binary_cmd()
        .current_dir(temp.path())
        .args(["--target-path", "custom_target", "--select", "my_model"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[my_model]"));
}

#[test]
fn supports_union_with_space_separated_selectors() {
    assert_selected_nodes(
        &["-s", "my_model", "child_a+"],
        &["my_model", "child_a", "grandchild"],
    );
}

#[test]
fn supports_intersection_with_commas() {
    assert_selected_nodes(
        &["-s", "tag:finance,config.meta.contains_pii:true"],
        &["child_a"],
    );
}

#[test]
fn supports_tag_method() {
    assert_selected_nodes(&["-s", "tag:finance"], &["child_a", "grandchild"]);
}

#[test]
fn supports_fqn_method() {
    assert_selected_nodes(
        &["-s", "fqn:pkg.marts.*"],
        &["child_a", "child_b", "grandchild"],
    );
}

#[test]
fn supports_path_method() {
    assert_selected_nodes(
        &["-s", "path:models/marts"],
        &["child_a", "child_b", "grandchild"],
    );
}

#[test]
fn supports_config_method() {
    assert_selected_nodes(&["-s", "config.materialized:view"], &["my_model"]);
}

#[test]
fn supports_descendants_for_plus_suffix() {
    assert_selected_nodes(
        &["-s", "my_model+"],
        &["my_model", "child_a", "child_b", "grandchild"],
    );
}

#[test]
fn supports_ancestors_for_plus_prefix() {
    assert_selected_nodes(
        &["-s", "+grandchild"],
        &["my_model", "child_a", "grandchild"],
    );
}

#[test]
fn supports_depth_bounded_plus() {
    assert_selected_nodes(&["-s", "my_model+1"], &["my_model", "child_a", "child_b"]);
}

#[test]
fn supports_at_graph_operator() {
    assert_selected_nodes(
        &["-s", "@my_model"],
        &["my_model", "child_a", "child_b", "grandchild"],
    );
    assert_selected_nodes(&["-s", "@child_a"], &["my_model", "child_a", "grandchild"]);
    assert_selected_nodes(
        &["-s", "@grandchild"],
        &["my_model", "child_a", "grandchild"],
    );
}

#[test]
fn rejects_at_with_trailing_plus() {
    let temp = setup_target_path("target");
    binary_cmd()
        .current_dir(temp.path())
        .args(["-s", "@my_model+"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "\"@\" and trailing \"+\" are incompatible",
        ));
}

#[test]
fn invalid_selector_exits_with_usage_code() {
    let temp = setup_target_path("target");
    binary_cmd()
        .current_dir(temp.path())
        .args(["--select", "tag:finance,"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "intersection groups cannot be empty",
        ));
}

#[test]
fn unknown_model_exits_with_runtime_code() {
    let temp = setup_target_path("target");
    binary_cmd()
        .current_dir(temp.path())
        .args(["-s", "does_not_exist"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("model 'does_not_exist' not found"));
}

#[test]
fn ambiguous_model_name_exits_with_runtime_code() {
    let temp = setup_target_path_with_manifest(
        "target",
        r#"{
            "nodes": {
                "model.pkg_a.orders": {"resource_type":"model","name":"orders","package_name":"pkg_a"},
                "model.pkg_b.orders": {"resource_type":"model","name":"orders","package_name":"pkg_b"}
            },
            "parent_map": {},
            "child_map": {}
        }"#,
    );
    binary_cmd()
        .current_dir(temp.path())
        .args(["-s", "orders"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("is ambiguous"));
}

#[test]
fn missing_manifest_exits_with_runtime_code() {
    let temp = TempDir::new().expect("temp dir should be created");
    fs::create_dir_all(temp.path().join("target")).expect("target dir should be created");

    binary_cmd()
        .current_dir(temp.path())
        .args(["-s", "my_model"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("manifest.json not found"));
}
