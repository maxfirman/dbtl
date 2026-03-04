mod cli;
mod error;
mod graph;
mod manifest;
mod render;
mod selector;

use clap::Parser;
use clap::error::ErrorKind;
use cli::Cli;
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
    let manifest_path = resolve_manifest_path(&cli.target_path);
    let manifest = Manifest::from_path(&manifest_path)?;
    let graph = GraphIndex::from_manifest(&manifest);

    let output = if let Some(raw_selectors) = cli.select {
        let selected_nodes = selector::resolve_selectors(&graph, &raw_selectors)?;
        render::render_selected_nodes(&graph, &selected_nodes)
    } else {
        render::render_all_models(&graph)
    };

    println!("{output}");
    Ok(())
}

fn resolve_manifest_path(target_path: &str) -> PathBuf {
    PathBuf::from(target_path).join("manifest.json")
}
