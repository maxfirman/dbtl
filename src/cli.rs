use clap::{ArgAction, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "dbtl")]
#[command(about = "Print model lineage slices from a dbt manifest")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
    #[arg(long, global = true, action = ArgAction::SetTrue)]
    pub version: bool,
    #[arg(short = 's', long, num_args = 1..)]
    pub select: Option<Vec<String>>,
    #[arg(long, default_value = "target")]
    pub target_path: String,
}

#[derive(Debug, Subcommand, PartialEq, Eq)]
pub enum Command {
    #[command(name = "self", about = "Manage dbtl itself")]
    SelfCmd {
        #[command(subcommand)]
        command: SelfCommand,
    },
}

#[derive(Debug, Subcommand, PartialEq, Eq)]
pub enum SelfCommand {
    #[command(about = "Update dbtl to the latest GitHub release")]
    Update,
}

#[cfg(test)]
mod tests {
    use super::{Cli, Command, SelfCommand};
    use clap::Parser;

    #[test]
    fn target_path_defaults_to_target() {
        let cli = Cli::try_parse_from(["dbtl"]).expect("cli should parse");
        assert_eq!(cli.target_path, "target");
    }

    #[test]
    fn accepts_multiple_select_arguments() {
        let cli =
            Cli::try_parse_from(["dbtl", "-s", "a", "b,c"]).expect("cli should parse selectors");
        assert_eq!(cli.select.unwrap_or_default(), vec!["a", "b,c"]);
    }

    #[test]
    fn parses_self_update_subcommand() {
        let cli = Cli::try_parse_from(["dbtl", "self", "update"]).expect("cli should parse");
        assert_eq!(
            cli.command,
            Some(Command::SelfCmd {
                command: SelfCommand::Update
            })
        );
    }

    #[test]
    fn parses_version_flag() {
        let cli = Cli::try_parse_from(["dbtl", "--version"]).expect("cli should parse");
        assert!(cli.version);
    }
}
