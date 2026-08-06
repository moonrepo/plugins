use moon_config::UnresolvedVersionSpec;

/// Official Microsoft install-script endpoints (stable redirect aliases).
pub const INSTALL_SCRIPT_URL_PS1: &str = "https://dot.net/v1/dotnet-install.ps1";
pub const INSTALL_SCRIPT_URL_SH: &str = "https://dot.net/v1/dotnet-install.sh";

pub fn install_script_url(windows: bool) -> &'static str {
    if windows {
        INSTALL_SCRIPT_URL_PS1
    } else {
        INSTALL_SCRIPT_URL_SH
    }
}

pub fn install_script_file_name(windows: bool) -> &'static str {
    if windows {
        "dotnet-install.ps1"
    } else {
        "dotnet-install.sh"
    }
}

/// The exact SDK version an exact spec would install (`8.0.404`), used to
/// short-circuit when `<root>/sdk/<version>` already exists. Channels and
/// aliases resolve server-side, so only fully-qualified versions qualify.
pub fn exact_version(spec: &UnresolvedVersionSpec) -> Option<String> {
    match spec {
        UnresolvedVersionSpec::Semantic(version) => Some(version.to_string()),
        _ => None,
    }
}

/// Map a configured version spec onto dotnet-install script arguments,
/// passing through the script's native semantics: `X.Y` requirements become
/// channels, `lts`/`sts`/`preview` aliases become named channels, and
/// fully-qualified versions install pinned.
pub fn install_version_args(
    spec: &UnresolvedVersionSpec,
    windows: bool,
) -> Result<Vec<String>, String> {
    let channel_flag = if windows { "-Channel" } else { "--channel" };
    let version_flag = if windows { "-Version" } else { "--version" };

    let unsupported = |value: &dyn std::fmt::Display| {
        format!(
            "Unsupported .NET version specification `{value}` — use a channel like `8.0`, \
             an exact version like `8.0.404`, or one of `lts`, `sts`, `preview`."
        )
    };

    match spec {
        UnresolvedVersionSpec::Semantic(version) => {
            Ok(vec![version_flag.into(), version.to_string()])
        }
        UnresolvedVersionSpec::Alias(alias) => {
            let channel = match alias.to_ascii_lowercase().as_str() {
                "lts" => "LTS",
                // "Current" was renamed to STS; treat "latest" the same way.
                "sts" | "current" | "latest" => "STS",
                "preview" => "Preview",
                _ => return Err(unsupported(alias)),
            };

            Ok(vec![channel_flag.into(), channel.into()])
        }
        UnresolvedVersionSpec::Req(req) => {
            let Some(comparator) = req.comparators.first() else {
                return Err(unsupported(req));
            };

            // dotnet-install channels are `major.minor` feature bands.
            Ok(vec![
                channel_flag.into(),
                format!("{}.{}", comparator.major, comparator.minor.unwrap_or(0)),
            ])
        }
        other => Err(unsupported(other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(value: &str) -> UnresolvedVersionSpec {
        UnresolvedVersionSpec::parse(value).unwrap()
    }

    #[test]
    fn exact_version_only_for_fully_qualified() {
        assert_eq!(exact_version(&parse("8.0.404")).as_deref(), Some("8.0.404"));
        assert_eq!(exact_version(&parse("8.0")), None);
        assert_eq!(exact_version(&parse("lts")), None);
    }

    #[test]
    fn exact_versions_pass_through() {
        assert_eq!(
            install_version_args(&parse("8.0.404"), true).unwrap(),
            vec!["-Version", "8.0.404"]
        );
        assert_eq!(
            install_version_args(&parse("8.0.404"), false).unwrap(),
            vec!["--version", "8.0.404"]
        );
    }

    #[test]
    fn partial_versions_become_channels() {
        assert_eq!(
            install_version_args(&parse("8.0"), true).unwrap(),
            vec!["-Channel", "8.0"]
        );
        assert_eq!(
            install_version_args(&parse("9"), false).unwrap(),
            vec!["--channel", "9.0"]
        );
    }

    #[test]
    fn aliases_become_named_channels() {
        assert_eq!(
            install_version_args(&parse("lts"), false).unwrap(),
            vec!["--channel", "LTS"]
        );
        assert_eq!(
            install_version_args(&parse("latest"), true).unwrap(),
            vec!["-Channel", "STS"]
        );
    }

    #[test]
    fn unsupported_specs_error() {
        assert!(install_version_args(&parse("canary"), true).is_err());
        assert!(install_version_args(&parse("banana"), false).is_err());
    }
}
