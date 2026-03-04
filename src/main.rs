mod cli;
mod error;
mod graph;
mod manifest;
mod render;
mod selector;
mod version;

use clap::Parser;
use clap::error::ErrorKind;
use cli::{Cli, Command, SelfCommand};
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

    if cli.version {
        println!("dbtl {}", version::current_version());
        return Ok(());
    }

    if let Some(command) = cli.command {
        match command {
            Command::SelfCmd {
                command: SelfCommand::Update,
            } => return run_self_update(),
        }
    }

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

fn run_self_update() -> Result<(), AppError> {
    let current_version = version::current_version();
    let status = self_update::backends::github::Update::configure()
        .repo_owner("maxfirman")
        .repo_name("dbtl")
        .bin_name("dbtl")
        .show_download_progress(true)
        .current_version(current_version)
        .build()
        .map_err(|err| AppError::self_update(err.to_string()))?
        .update()
        .map_err(|err| AppError::self_update(err.to_string()))?;

    println!("Updated dbtl to {}", status.version());
    Ok(())
}
