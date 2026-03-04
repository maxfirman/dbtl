use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

#[test]
fn parses_fresh_jaffle_project_and_queries_lineage() {
    if std::env::var("DBTL_RUN_DBT_ITEST").as_deref() != Ok("1") {
        eprintln!("skipping: set DBTL_RUN_DBT_ITEST=1 to run dbt integration tests");
        return;
    }
    if !dbt_available() {
        eprintln!("skipping: dbt CLI is not installed");
        return;
    }

    let temp = TempDir::new().expect("temp dir should be created");
    let project_src = fixture_project_dir();
    let project_dst = temp.path().join("jaffle_shop_project");
    copy_dir_recursive(&project_src, &project_dst).expect("fixture project should copy");
    let target_dir = temp.path().join("target");

    Command::new("dbt")
        .args([
            "parse",
            "--project-dir",
            project_dst.to_str().expect("path should be valid utf-8"),
            "--profiles-dir",
            project_dst.to_str().expect("path should be valid utf-8"),
            "--target-path",
            target_dir.to_str().expect("path should be valid utf-8"),
            "--no-version-check",
        ])
        .assert()
        .success();

    Command::new(assert_cmd::cargo::cargo_bin!("dbtl"))
        .args([
            "--state",
            target_dir.to_str().expect("path should be valid utf-8"),
            "--select",
            "orders+",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("orders"))
        .stdout(predicate::str::contains("customers"));

    Command::new(assert_cmd::cargo::cargo_bin!("dbtl"))
        .args([
            "--state",
            target_dir.to_str().expect("path should be valid utf-8"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("[stg_orders]"))
        .stdout(predicate::str::contains("[customers]"))
        .stdout(predicate::str::contains(">"));
}

fn dbt_available() -> bool {
    Command::new("dbt").arg("--version").output().is_ok()
}

fn fixture_project_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("jaffle_shop_project")
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else if ty.is_file() {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
