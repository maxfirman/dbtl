use assert_cmd::Command;
use predicates::prelude::*;
use std::collections::BTreeSet;
use std::{fs, path::Path};
use tempfile::TempDir;

fn setup_state_dir(state_name: &str) -> TempDir {
    let temp = TempDir::new().expect("temp dir should be created");
    let state_path = temp.path().join(state_name);
    fs::create_dir_all(&state_path).expect("state dir should be created");
    copy_fixture_manifest(&state_path.join("manifest.json"));
    temp
}

fn setup_state_dir_with_manifest(state_name: &str, manifest_json: &str) -> TempDir {
    let temp = TempDir::new().expect("temp dir should be created");
    let state_path = temp.path().join(state_name);
    fs::create_dir_all(&state_path).expect("state dir should be created");
    fs::write(state_path.join("manifest.json"), manifest_json).expect("manifest should be written");
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

#[test]
fn help_flag_exits_successfully() {
    binary_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: dbtl [OPTIONS]"));
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

#[test]
fn prints_selected_only() {
    let temp = setup_state_dir("target");
    let assert = binary_cmd()
        .current_dir(temp.path())
        .args(["--select", "my_model"])
        .assert()
        .success();
    let output =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout should be utf-8");
    assert_eq!(
        extracted_nodes(&output),
        BTreeSet::from(["my_model".to_string()])
    );
}

#[test]
fn prints_all_models_when_select_is_omitted() {
    let temp = setup_state_dir("target");
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
fn prints_descendants_for_plus_suffix() {
    let temp = setup_state_dir("target");
    let assert = binary_cmd()
        .current_dir(temp.path())
        .args(["--select", "my_model+"])
        .assert()
        .success();
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
}

#[test]
fn prints_ancestors_for_plus_prefix() {
    let temp = setup_state_dir("target");
    let assert = binary_cmd()
        .current_dir(temp.path())
        .args(["--select", "+grandchild"])
        .assert()
        .success();
    let output =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout should be utf-8");

    assert_eq!(
        extracted_nodes(&output),
        BTreeSet::from([
            "child_a".to_string(),
            "grandchild".to_string(),
            "my_model".to_string(),
        ])
    );
}

#[test]
fn prints_both_sections_for_surrounded_plus() {
    let temp = setup_state_dir("target");
    let assert = binary_cmd()
        .current_dir(temp.path())
        .args(["--select", "+child_a+"])
        .assert()
        .success();
    let output =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout should be utf-8");

    assert_eq!(
        extracted_nodes(&output),
        BTreeSet::from([
            "child_a".to_string(),
            "grandchild".to_string(),
            "my_model".to_string(),
        ])
    );
}

#[test]
fn target_path_flag_overrides_default_target() {
    let temp = setup_state_dir("custom_state");
    binary_cmd()
        .current_dir(temp.path())
        .args(["--target-path", "custom_state", "--select", "my_model"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[my_model]"));
}

#[test]
fn invalid_selector_exits_with_usage_code() {
    let temp = setup_state_dir("target");
    binary_cmd()
        .current_dir(temp.path())
        .args(["--select", "my_model++"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("Invalid selector"));
}

#[test]
fn short_select_flag_is_supported() {
    let temp = setup_state_dir("target");
    binary_cmd()
        .current_dir(temp.path())
        .args(["-s", "my_model+"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[my_model]"))
        .stdout(predicate::str::contains("[child_a]"));
}

#[test]
fn supports_union_with_space_separated_selectors() {
    let temp = setup_state_dir("target");
    let assert = binary_cmd()
        .current_dir(temp.path())
        .args(["-s", "my_model", "child_a+"])
        .assert()
        .success();
    let output =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout should be utf-8");

    assert_eq!(
        extracted_nodes(&output),
        BTreeSet::from([
            "child_a".to_string(),
            "grandchild".to_string(),
            "my_model".to_string(),
        ])
    );
}

#[test]
fn supports_ancestor_depth_prefix() {
    let temp = setup_state_dir("target");
    let assert = binary_cmd()
        .current_dir(temp.path())
        .args(["-s", "1+grandchild"])
        .assert()
        .success();
    let output =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout should be utf-8");

    assert_eq!(
        extracted_nodes(&output),
        BTreeSet::from(["child_a".to_string(), "grandchild".to_string()])
    );
}

#[test]
fn supports_descendant_depth_suffix() {
    let temp = setup_state_dir("target");
    let assert = binary_cmd()
        .current_dir(temp.path())
        .args(["-s", "my_model+1"])
        .assert()
        .success();
    let output =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout should be utf-8");

    assert_eq!(
        extracted_nodes(&output),
        BTreeSet::from([
            "child_a".to_string(),
            "child_b".to_string(),
            "my_model".to_string(),
        ])
    );
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

#[test]
fn unknown_model_exits_with_runtime_code() {
    let temp = setup_state_dir("target");
    binary_cmd()
        .current_dir(temp.path())
        .args(["-s", "does_not_exist"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("model 'does_not_exist' not found"));
}

#[test]
fn ambiguous_model_name_exits_with_runtime_code() {
    let temp = setup_state_dir_with_manifest(
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
