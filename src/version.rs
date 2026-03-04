pub fn current_version() -> &'static str {
    normalize(option_env!("DBTL_VERSION").unwrap_or(env!("CARGO_PKG_VERSION")))
}

fn normalize(version: &str) -> &str {
    version.strip_prefix('v').unwrap_or(version)
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn strips_v_prefix() {
        assert_eq!(normalize("v0.1.6"), "0.1.6");
    }

    #[test]
    fn keeps_plain_semver() {
        assert_eq!(normalize("0.1.6"), "0.1.6");
    }
}
