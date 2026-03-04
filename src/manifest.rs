use serde::Deserialize;
use std::{collections::HashMap, fs, path::Path};

#[derive(Debug, Deserialize)]
pub struct Manifest {
    #[serde(default)]
    pub nodes: HashMap<String, NodeEntry>,
    #[serde(default)]
    pub parent_map: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub child_map: HashMap<String, Vec<String>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct NodeEntry {
    pub resource_type: String,
    pub name: String,
    pub package_name: String,
}

impl Manifest {
    pub fn from_path(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Err(format!(
                "manifest.json not found at {}",
                path.to_string_lossy()
            ));
        }

        let content = fs::read_to_string(path).map_err(|e| {
            format!(
                "failed reading manifest at {}: {e}",
                path.to_string_lossy()
            )
        })?;
        serde_json::from_str(&content).map_err(|e| {
            format!(
                "failed parsing manifest JSON at {}: {e}",
                path.to_string_lossy()
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Manifest;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn loads_valid_manifest() {
        let temp = TempDir::new().expect("temp dir should be created");
        let path = temp.path().join("manifest.json");
        fs::write(
            &path,
            r#"{
                "nodes": {
                    "model.pkg.a": {
                        "resource_type": "model",
                        "name": "a",
                        "package_name": "pkg"
                    }
                },
                "parent_map": {},
                "child_map": {}
            }"#,
        )
        .expect("manifest should be written");

        let parsed = Manifest::from_path(&path).expect("manifest should parse");
        assert_eq!(parsed.nodes.len(), 1);
    }

    #[test]
    fn errors_when_manifest_missing() {
        let temp = TempDir::new().expect("temp dir should be created");
        let path = temp.path().join("missing_manifest.json");
        let error = Manifest::from_path(&path).expect_err("missing manifest should error");
        assert!(error.contains("manifest.json not found"));
    }

    #[test]
    fn errors_when_manifest_invalid_json() {
        let temp = TempDir::new().expect("temp dir should be created");
        let path = temp.path().join("manifest.json");
        fs::write(&path, "{invalid_json").expect("manifest should be written");

        let error = Manifest::from_path(&path).expect_err("invalid json should error");
        assert!(error.contains("failed parsing manifest JSON"));
    }
}
