use crate::error::AppError;
use clap::Parser;

const INVALID_SELECTOR_MSG: &str =
    "Invalid selector. Allowed forms: model, model+, +model, +model+, N+model, model+N, N+model+M";
const INVALID_SELECTOR_DEPTH_MSG: &str =
    "Invalid selector depth. Depth must be a positive integer (for example: 1+model or model+2)";

#[derive(Debug, Parser)]
#[command(name = "dbtl")]
#[command(about = "Print model lineage slices from a dbt manifest")]
pub struct Cli {
    #[arg(short = 's', long, num_args = 1..)]
    pub select: Option<Vec<String>>,
    #[arg(long, default_value = "target")]
    pub target_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectorSpec {
    pub ancestor_depth: Option<usize>,
    pub descendant_depth: Option<usize>,
    pub model_name: String,
}

impl SelectorSpec {
    const UNBOUNDED_DEPTH: usize = usize::MAX;

    pub fn parse(raw: &str) -> Result<Self, AppError> {
        let (ancestor_depth, remaining) = parse_prefix(raw)?;
        let (descendant_depth, core) = parse_suffix(remaining)?;

        if core.is_empty() || !core.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(AppError::usage(INVALID_SELECTOR_MSG));
        }

        Ok(Self {
            ancestor_depth,
            descendant_depth,
            model_name: core,
        })
    }

    pub fn ancestor_depth_limit(&self) -> Option<usize> {
        self.ancestor_depth
    }

    pub fn descendant_depth_limit(&self) -> Option<usize> {
        self.descendant_depth
    }
}

fn parse_prefix(raw: &str) -> Result<(Option<usize>, &str), AppError> {
    if let Some(stripped) = raw.strip_prefix('+') {
        return Ok((Some(SelectorSpec::UNBOUNDED_DEPTH), stripped));
    }

    let digit_count = raw.chars().take_while(|c| c.is_ascii_digit()).count();
    if digit_count == 0 {
        return Ok((None, raw));
    }
    if raw.chars().nth(digit_count) != Some('+') {
        return Err(AppError::usage(INVALID_SELECTOR_MSG));
    }

    let depth = parse_positive_depth(&raw[..digit_count])?;
    Ok((Some(depth), &raw[digit_count + 1..]))
}

fn parse_suffix(raw: &str) -> Result<(Option<usize>, String), AppError> {
    let Some(plus_idx) = raw.rfind('+') else {
        return Ok((None, raw.to_string()));
    };

    let model = &raw[..plus_idx];
    let suffix = &raw[plus_idx + 1..];
    if model.contains('+') {
        return Err(AppError::usage(INVALID_SELECTOR_MSG));
    }
    if suffix.is_empty() {
        return Ok((Some(SelectorSpec::UNBOUNDED_DEPTH), model.to_string()));
    }
    if !suffix.chars().all(|c| c.is_ascii_digit()) {
        return Err(AppError::usage(INVALID_SELECTOR_MSG));
    }

    let depth = parse_positive_depth(suffix)?;
    Ok((Some(depth), model.to_string()))
}

fn parse_positive_depth(raw: &str) -> Result<usize, AppError> {
    let parsed = raw
        .parse::<usize>()
        .map_err(|_| AppError::usage(INVALID_SELECTOR_DEPTH_MSG))?;
    if parsed == 0 {
        return Err(AppError::usage(
            "Invalid selector depth. Depth must be >= 1",
        ));
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::SelectorSpec;
    use proptest::prelude::*;

    #[test]
    fn parses_valid_selectors() {
        assert_eq!(
            SelectorSpec::parse("orders").expect("selector should parse"),
            SelectorSpec {
                ancestor_depth: None,
                descendant_depth: None,
                model_name: "orders".to_string(),
            }
        );
        assert_eq!(
            SelectorSpec::parse("orders+").expect("selector should parse"),
            SelectorSpec {
                ancestor_depth: None,
                descendant_depth: Some(usize::MAX),
                model_name: "orders".to_string(),
            }
        );
        assert_eq!(
            SelectorSpec::parse("+orders").expect("selector should parse"),
            SelectorSpec {
                ancestor_depth: Some(usize::MAX),
                descendant_depth: None,
                model_name: "orders".to_string(),
            }
        );
        assert_eq!(
            SelectorSpec::parse("+orders+").expect("selector should parse"),
            SelectorSpec {
                ancestor_depth: Some(usize::MAX),
                descendant_depth: Some(usize::MAX),
                model_name: "orders".to_string(),
            }
        );
        assert_eq!(
            SelectorSpec::parse("1+orders").expect("selector should parse"),
            SelectorSpec {
                ancestor_depth: Some(1),
                descendant_depth: None,
                model_name: "orders".to_string(),
            }
        );
        assert_eq!(
            SelectorSpec::parse("orders+2").expect("selector should parse"),
            SelectorSpec {
                ancestor_depth: None,
                descendant_depth: Some(2),
                model_name: "orders".to_string(),
            }
        );
        assert_eq!(
            SelectorSpec::parse("1+orders+2").expect("selector should parse"),
            SelectorSpec {
                ancestor_depth: Some(1),
                descendant_depth: Some(2),
                model_name: "orders".to_string(),
            }
        );
    }

    #[test]
    fn rejects_invalid_selectors() {
        for selector in [
            "",
            "orders++",
            "pkg.orders",
            "orders,customers",
            "+",
            "++orders",
            "0+orders",
            "orders+0",
            "1+",
        ] {
            assert!(
                SelectorSpec::parse(selector).is_err(),
                "selector should fail: {selector}"
            );
        }
    }

    proptest! {
        #[test]
        fn valid_model_names_roundtrip(name in "[A-Za-z_][A-Za-z0-9_]{0,20}") {
            let parsed = SelectorSpec::parse(&name).expect("generated selector should parse");
            prop_assert_eq!(parsed.model_name.as_str(), name.as_str());
            prop_assert_eq!(parsed.ancestor_depth_limit(), None);
            prop_assert_eq!(parsed.descendant_depth_limit(), None);
        }

        #[test]
        fn random_selector_without_allowed_charset_fails(s in "[^A-Za-z0-9_+]{1,12}") {
            prop_assert!(SelectorSpec::parse(&s).is_err());
        }
    }
}
