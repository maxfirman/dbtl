use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "dbtl")]
#[command(about = "Print model lineage slices from a dbt manifest")]
pub struct Cli {
    #[arg(short = 's', long, num_args = 1..)]
    pub select: Option<Vec<String>>,
    #[arg(long, default_value = "target")]
    pub target_path: String,
}

#[cfg(test)]
mod tests {
    use super::Cli;
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
}
