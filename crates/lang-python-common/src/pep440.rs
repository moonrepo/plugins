use proto_pdk_api::{AnyResult, UnresolvedVersionSpec};

/// Convert a PEP 440 version specifier, like `>=0.5, !=0.5.3`, into a proto requirement.
/// Versions are zero-padded, as PEP 440 compares them (`==0.5` and `!=0.5` are `0.5.0`).
/// Poetry's `^` and `~` pass through.
///
/// <https://peps.python.org/pep-0440/#version-specifiers>
pub fn parse_pep440_spec(spec: &str) -> AnyResult<UnresolvedVersionSpec> {
    let mut clauses = vec![];

    for clause in spec.split(',').map(str::trim).filter(|c| !c.is_empty()) {
        let Some(index) = clause.find(|c: char| c.is_ascii_digit()) else {
            clauses.push(clause.to_owned());
            continue;
        };

        let (op, version) = clause.split_at(index);
        let op = op.trim();
        let parts = version.split('.').collect::<Vec<_>>();
        let padded = format!(
            "{}.{}.{}",
            parts[0],
            parts.get(1).unwrap_or(&"0"),
            parts.get(2).unwrap_or(&"0")
        );

        match op {
            // A partial `!=` excludes every version with those parts, like a wildcard
            "!=" if version.ends_with('*') => {
                clauses.push(format!("!={}", version.trim_end_matches(".*")))
            }
            "!=" => clauses.push(format!("!={padded}")),
            "" | "==" | "===" if version.ends_with('*') => clauses.push(format!("={version}")),
            "" | "==" | "===" => clauses.push(format!("={padded}")),
            // `~=1.2` is `>=1.2, ==1.*`, and `~=1.2.3` is `>=1.2.3, ==1.2.*`
            "~=" => {
                clauses.push(format!(">={padded}"));
                clauses.push(format!("={}.*", parts[..parts.len() - 1].join(".")));
            }
            ">=" | "<=" | ">" | "<" => clauses.push(format!("{op}{padded}")),
            _ => clauses.push(clause.to_owned()),
        }
    }

    if clauses.is_empty() {
        clauses.push("*".into());
    }

    Ok(UnresolvedVersionSpec::parse(clauses.join(", "))?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proto_pdk_api::{MatchesVersion, Version};

    fn parse(spec: &str) -> UnresolvedVersionSpec {
        UnresolvedVersionSpec::parse(spec).unwrap()
    }

    // The highest version that the PEP 440 spec resolves to in proto
    fn resolve(spec: &str) -> String {
        let spec = parse_pep440_spec(spec).unwrap();

        [
            "0.4.30", "0.5.0", "0.5.3", "0.5.31", "0.13.7", "1.0.0", "1.2.0", "1.4.2",
        ]
        .into_iter()
        .map(|v| Version::parse(v).unwrap())
        .filter(|v| match &spec {
            UnresolvedVersionSpec::Range(range) => range.matches(v),
            UnresolvedVersionSpec::Requirement(req) => req.matches(v),
            UnresolvedVersionSpec::Version(version) => version == v,
            _ => false,
        })
        .max()
        .map(|v| v.to_string())
        .unwrap_or_default()
    }

    #[test]
    fn pads_exact_versions() {
        assert_eq!(parse_pep440_spec("0.5").unwrap(), parse("=0.5.0"));
        assert_eq!(parse_pep440_spec("==0.5").unwrap(), parse("=0.5.0"));
        assert_eq!(parse_pep440_spec("===0.5.3").unwrap(), parse("=0.5.3"));
        assert_eq!(resolve("==0.5"), "0.5.0");
    }

    #[test]
    fn pads_comparisons() {
        assert_eq!(parse_pep440_spec("<=0.5").unwrap(), parse("<=0.5.0"));
        assert_eq!(
            parse_pep440_spec(">= 0.4, < 0.6").unwrap(),
            parse(">=0.4.0, <0.6.0")
        );
        assert_eq!(resolve("<=0.5"), "0.5.0");
        assert_eq!(resolve(">=0.4,<0.6"), "0.5.31");
    }

    #[test]
    fn supports_wildcards() {
        assert_eq!(parse_pep440_spec("==0.5.*").unwrap(), parse("=0.5.*"));
        assert_eq!(resolve("==0.5.*"), "0.5.31");
    }

    #[test]
    fn supports_compatible_releases() {
        assert_eq!(parse_pep440_spec("~=0.5").unwrap(), parse(">=0.5.0, =0.*"));
        assert_eq!(
            parse_pep440_spec("~=1.2.3").unwrap(),
            parse(">=1.2.3, =1.2.*")
        );
        assert_eq!(resolve("~=0.5"), "0.13.7");
        assert_eq!(resolve("~=0.5.1"), "0.5.31");
        assert_eq!(resolve("~=1.2"), "1.4.2");
    }

    #[test]
    fn supports_exclusions() {
        assert_eq!(
            parse_pep440_spec(">=0.5, !=0.5.3").unwrap(),
            parse(">=0.5.0, !=0.5.3")
        );
        assert_eq!(parse_pep440_spec("!=0.5.3").unwrap(), parse("!=0.5.3"));
        assert_eq!(resolve("~=0.5.1, !=0.5.31"), "0.5.3");
        assert_eq!(resolve("<0.5.3, !=0.5"), "0.4.30");
        assert_eq!(resolve("<1.0, !=0.5.*"), "0.13.7");
    }

    #[test]
    fn passes_through_poetry_operators() {
        assert_eq!(parse_pep440_spec("^1.2").unwrap(), parse("^1.2"));
        assert_eq!(parse_pep440_spec("~1.2").unwrap(), parse("~1.2"));
    }
}
