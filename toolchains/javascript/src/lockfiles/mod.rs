mod bun;
mod deno;
mod npm;
mod pnpm;
mod yarn;

pub use bun::*;
pub use deno::*;
pub use npm::*;
pub use pnpm::*;
pub use yarn::*;

use moon_pdk::AnyResult;
use moon_pdk_api::VersionSpec;

pub(super) fn parse_version_spec<T: AsRef<str>>(version: T) -> AnyResult<Option<VersionSpec>> {
    let version = version.as_ref();

    if version.is_empty() || version.contains('#') || version.contains(':') {
        Ok(None)
    } else {
        Ok(Some(VersionSpec::parse(version)?))
    }
}

pub(super) fn parse_name_and_version<'a>(
    value: &'a str,
    delimiter: &str,
) -> Option<(&'a str, &'a str)> {
    // Remove parents:
    //  @babel/preset-react@7.27.1_@babel+core@7.28.3
    //  @jest/core@29.7.0(@babel/types@7.26.10)
    let value = if delimiter.is_empty() {
        value
    } else {
        match value.find(delimiter) {
            Some(index) => &value[0..index],
            None => value,
        }
    };

    // Split on @ but preserve scope (skip the leading character so a scope's
    // `@` isn't treated as the name/version separator):
    //  @babel/preset-react@7.27.1
    //  @jest/core@29.7.0
    if let Some(rest) = value.get(1..)
        && let Some(index) = rest.find('@')
    {
        let name = &value[0..=index];
        let version = &value[index + 2..];

        // Ignore values that use URLs
        if !version.contains("://") {
            return Some((name, version));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    mod parse_name_and_version {
        use super::*;

        // bun ids have no parents, so an empty delimiter is passed.
        mod bun {
            use super::*;

            #[test]
            fn parses_name_and_version() {
                assert_eq!(
                    parse_name_and_version("csstype@3.1.3", ""),
                    Some(("csstype", "3.1.3"))
                );
            }

            #[test]
            fn preserves_scope() {
                assert_eq!(
                    parse_name_and_version("@babel/core@7.28.3", ""),
                    Some(("@babel/core", "7.28.3"))
                );
            }

            #[test]
            fn ignores_url_versions() {
                assert_eq!(
                    parse_name_and_version("foo@https://example.com/foo.tgz", ""),
                    None
                );
                assert_eq!(
                    parse_name_and_version("foo@git+ssh://git@github.com/foo/bar.git", ""),
                    None
                );
            }
        }

        // deno separates a package from its parents with `_`:
        //  @babel/preset-react@7.27.1_@babel+core@7.28.3
        mod deno {
            use super::*;

            #[test]
            fn parses_name_and_version() {
                assert_eq!(
                    parse_name_and_version("@babel/core@7.28.3", "_"),
                    Some(("@babel/core", "7.28.3"))
                );
            }

            #[test]
            fn strips_parents() {
                assert_eq!(
                    parse_name_and_version("@babel/preset-react@7.27.1_@babel+core@7.28.3", "_"),
                    Some(("@babel/preset-react", "7.27.1"))
                );
            }
        }

        // pnpm separates a package from its parents with `(`:
        //  @jest/core@29.7.0(@babel/types@7.26.10)
        mod pnpm {
            use super::*;

            #[test]
            fn parses_name_and_version() {
                assert_eq!(
                    parse_name_and_version("typescript@5.9.2", "("),
                    Some(("typescript", "5.9.2"))
                );
            }

            #[test]
            fn preserves_scope() {
                assert_eq!(
                    parse_name_and_version("@jest/core@29.7.0", "("),
                    Some(("@jest/core", "29.7.0"))
                );
            }

            #[test]
            fn strips_parents() {
                assert_eq!(
                    parse_name_and_version("@jest/core@29.7.0(@babel/types@7.26.10)", "("),
                    Some(("@jest/core", "29.7.0"))
                );
            }

            #[test]
            fn strips_peer_parents() {
                assert_eq!(
                    parse_name_and_version("seroval-plugins@1.3.2(seroval@1.3.2)", "("),
                    Some(("seroval-plugins", "1.3.2"))
                );
            }
        }

        #[test]
        fn returns_none_when_pkg_urls() {
            assert_eq!(
                parse_name_and_version(
                    "@dxos/echo@https://pkg.pr.new/dxos/dxos/@dxos/echo@728b08e",
                    ""
                ),
                None
            );
        }

        #[test]
        fn returns_none_when_no_version() {
            assert_eq!(parse_name_and_version("react", ""), None);
        }

        #[test]
        fn returns_none_for_scope_without_version() {
            assert_eq!(parse_name_and_version("@babel/core", ""), None);
        }

        #[test]
        fn returns_none_for_empty_value() {
            assert_eq!(parse_name_and_version("", ""), None);
        }
    }
}
