use assert_cmd::Command;
use predicates::prelude::*;
use std::{fs, path::Path};
use tempfile::TempDir;

fn setup_state_dir(state_name: &str) -> TempDir {
    let temp = TempDir::new().expect("temp dir should be created");
    let state_path = temp.path().join(state_name);
    fs::create_dir_all(&state_path).expect("state dir should be created");
    copy_fixture_manifest(&state_path.join("manifest.json"));
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
fn prints_selected_only() {
    let temp = setup_state_dir("target");
    let mut cmd = binary_cmd();
    cmd.current_dir(temp.path())
        .args(["--select", "my_model"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[my_model]"))
        .stdout(predicate::str::contains("["));
}

#[test]
fn prints_all_models_when_select_is_omitted() {
    let temp = setup_state_dir("target");
    let mut cmd = binary_cmd();
    cmd.current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("[my_model]"))
        .stdout(predicate::str::contains("[child_a]"))
        .stdout(predicate::str::contains("[child_b]"))
        .stdout(predicate::str::contains("[grandchild]"));
}

#[test]
fn prints_descendants_for_plus_suffix() {
    let temp = setup_state_dir("target");
    let mut cmd = binary_cmd();
    cmd.current_dir(temp.path())
        .args(["--select", "my_model+"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[my_model]"))
        .stdout(predicate::str::contains("[child_a]"))
        .stdout(predicate::str::contains("[child_b]"))
        .stdout(predicate::str::contains("[grandchild]"));
}

#[test]
fn prints_ancestors_for_plus_prefix() {
    let temp = setup_state_dir("target");
    let mut cmd = binary_cmd();
    cmd.current_dir(temp.path())
        .args(["--select", "+grandchild"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[grandchild]"))
        .stdout(predicate::str::contains("[child_a]"))
        .stdout(predicate::str::contains("[my_model]"));
}

#[test]
fn prints_both_sections_for_surrounded_plus() {
    let temp = setup_state_dir("target");
    let mut cmd = binary_cmd();
    cmd.current_dir(temp.path())
        .args(["--select", "+child_a+"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[child_a]"))
        .stdout(predicate::str::contains("[my_model]"))
        .stdout(predicate::str::contains("[grandchild]"));
}

#[test]
fn state_flag_overrides_default_target() {
    let temp = setup_state_dir("custom_state");
    let mut cmd = binary_cmd();
    cmd.current_dir(temp.path())
        .args(["--state", "custom_state", "--select", "my_model"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[my_model]"));
}

#[test]
fn invalid_selector_exits_with_usage_code() {
    let temp = setup_state_dir("target");
    let mut cmd = binary_cmd();
    cmd.current_dir(temp.path())
        .args(["--select", "my_model++"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("Invalid selector"));
}

#[test]
fn short_select_flag_is_supported() {
    let temp = setup_state_dir("target");
    let mut cmd = binary_cmd();
    cmd.current_dir(temp.path())
        .args(["-s", "my_model+"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[my_model]"))
        .stdout(predicate::str::contains("[child_a]"));
}
