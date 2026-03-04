mod cli;
mod error;
mod graph;
mod manifest;
mod render;

use clap::Parser;
use clap::error::ErrorKind;
use cli::{Cli, SelectorSpec};
use error::AppError;
use graph::GraphIndex;
use manifest::Manifest;
use std::path::PathBuf;

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(err.exit_code());
    }
}

fn run() -> Result<(), AppError> {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => match err.kind() {
            ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                print!("{err}");
                return Ok(());
            }
            _ => return Err(AppError::usage(err.to_string())),
        },
    };
    let manifest_path = resolve_manifest_path(&cli.state);
    let manifest = Manifest::from_path(&manifest_path)?;
    let graph = GraphIndex::from_manifest(&manifest);

    let output = if let Some(raw_selectors) = cli.select {
        let selections = resolve_selections(&graph, raw_selectors)?;
        if selections.len() == 1 {
            let (root_id, selector) = &selections[0];
            render::render_selection(&graph, root_id, selector)
        } else {
            render::render_union_selection(&graph, &selections)
        }
    } else {
        render::render_all_models(&graph)
    };

    println!("{output}");
    Ok(())
}

fn resolve_selections(
    graph: &GraphIndex,
    raw_selectors: Vec<String>,
) -> Result<Vec<(String, SelectorSpec)>, AppError> {
    raw_selectors
        .into_iter()
        .map(|raw_selector| {
            let selector = SelectorSpec::parse(&raw_selector)?;
            let root_id = graph.resolve_model(&selector.model_name)?;
            Ok((root_id.to_string(), selector))
        })
        .collect()
}

fn resolve_manifest_path(state_dir: &str) -> PathBuf {
    PathBuf::from(state_dir).join("manifest.json")
}
