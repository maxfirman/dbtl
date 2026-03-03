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
