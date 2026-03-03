use crate::AppError;
use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "dbtl")]
#[command(about = "Print model lineage slices from a dbt manifest")]
pub struct Cli {
    #[arg(short = 's', long)]
    pub select: Option<String>,
    #[arg(long, default_value = "target")]
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectorSpec {
    pub include_ancestors: bool,
    pub include_descendants: bool,
    pub model_name: String,
}

impl SelectorSpec {
    pub fn parse(raw: &str) -> Result<Self, AppError> {
        let include_ancestors = raw.starts_with('+');
        let include_descendants = raw.ends_with('+');
        let core = raw
            .trim_start_matches('+')
            .trim_end_matches('+')
            .to_string();

        if core.is_empty()
            || !core.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            || raw.matches('+').count() > (include_ancestors as usize + include_descendants as usize)
        {
            return Err(AppError::usage(
                "Invalid selector. Allowed forms: model, model+, +model, +model+",
            ));
        }

        Ok(Self {
            include_ancestors,
            include_descendants,
            model_name: core,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::SelectorSpec;

    #[test]
    fn parses_valid_selectors() {
        assert_eq!(
            SelectorSpec::parse("orders").expect("selector should parse"),
            SelectorSpec {
                include_ancestors: false,
                include_descendants: false,
                model_name: "orders".to_string(),
            }
        );
        assert_eq!(
            SelectorSpec::parse("orders+").expect("selector should parse"),
            SelectorSpec {
                include_ancestors: false,
                include_descendants: true,
                model_name: "orders".to_string(),
            }
        );
        assert_eq!(
            SelectorSpec::parse("+orders").expect("selector should parse"),
            SelectorSpec {
                include_ancestors: true,
                include_descendants: false,
                model_name: "orders".to_string(),
            }
        );
        assert_eq!(
            SelectorSpec::parse("+orders+").expect("selector should parse"),
            SelectorSpec {
                include_ancestors: true,
                include_descendants: true,
                model_name: "orders".to_string(),
            }
        );
    }

    #[test]
    fn rejects_invalid_selectors() {
        for selector in ["", "orders++", "pkg.orders", "orders,customers", "+", "++orders"] {
            assert!(
                SelectorSpec::parse(selector).is_err(),
                "selector should fail: {selector}"
            );
        }
    }
}
