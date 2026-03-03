mod cli;
mod graph;
mod manifest;
mod render;

use clap::Parser;
use cli::{Cli, SelectorSpec};
use graph::GraphIndex;
use manifest::Manifest;
use std::path::PathBuf;

#[derive(Debug)]
enum AppError {
    Usage(String),
    Runtime(String),
}

impl AppError {
    fn usage(msg: impl Into<String>) -> Self {
        Self::Usage(msg.into())
    }

    fn runtime(msg: impl Into<String>) -> Self {
        Self::Runtime(msg.into())
    }

    fn exit_code(&self) -> i32 {
        match self {
            Self::Usage(_) => 2,
            Self::Runtime(_) => 1,
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage(msg) | Self::Runtime(msg) => write!(f, "{msg}"),
        }
    }
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(err.exit_code());
    }
}

fn run() -> Result<(), AppError> {
    let cli = Cli::try_parse().map_err(|e| AppError::usage(e.to_string()))?;
    let manifest_path = resolve_manifest_path(&cli.state);
    let manifest = Manifest::from_path(&manifest_path).map_err(AppError::runtime)?;
    let graph = GraphIndex::from_manifest(&manifest);
    let output = match cli.select {
        Some(raw_selector) => {
            let selector = SelectorSpec::parse(&raw_selector)?;
            let root_id = graph
                .resolve_model(&selector.model_name)
                .map_err(AppError::runtime)?;
            render::render_selection(&graph, root_id, &selector)
        }
        None => render::render_all_models(&graph),
    };
    println!("{output}");
    Ok(())
}

fn resolve_manifest_path(state_dir: &str) -> PathBuf {
    PathBuf::from(state_dir).join("manifest.json")
}
