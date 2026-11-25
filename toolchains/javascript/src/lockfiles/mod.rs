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

use moon_pdk_api::VersionSpec;
use toolchain_common::debug;

// ADR-0008.5: Skip a version that can't be parsed, instead of failing the whole lockfile
pub(super) fn parse_version_spec<T: AsRef<str>>(name: &str, version: T) -> Option<VersionSpec> {
    let version = version.as_ref();

    if version.is_empty() || version.contains('#') || version.contains(':') {
        return None;
    }

    match VersionSpec::parse(version) {
        Ok(spec) => Some(spec),
        Err(error) => {
            debug!("Skipping the locked version {version} of {name}: {error}");

            None
        }
    }
}

pub(super) fn parse_name_and_version<'a>(
    value: &'a str,
    delimiter: &str,
) -> Option<(&'a str, &'a str)> {
    // Split on the version's `@`, skipping the first character, so that
    // a scope's `@` isn't one, like in `@babel/core@7.28.3`
    let index = value.get(1..)?.find('@')? + 1;
    let name = &value[0..index];
    let mut version = &value[index + 1..];

    // Remove parents, which follow the version, as names may contain the
    // delimiter, like Deno's `string_decoder@1.3.0`:
    //  @babel/preset-react@7.27.1_@babel+core@7.28.3
    //  @jest/core@29.7.0(@babel/types@7.26.10)
    if !delimiter.is_empty()
        && let Some(end) = version.find(delimiter)
    {
        version = &version[0..end];
    }

    // Ignore values that use URLs
    if version.contains("://") {
        return None;
    }

    Some((name, version))
}

#[cfg(test)]
mod tests {
    use super::*;

    mod parse_version_spec {
        use super::*;

        #[test]
        fn parses_versions() {
            assert!(parse_version_spec("a", "1.2.3").is_some());
        }

        #[test]
        fn skips_invalid_versions() {
            assert!(parse_version_spec("a", "1..2").is_none());
        }
    }

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
                assert_eq!(
                    parse_name_and_version(
                        "ts-node@10.9.2_@types+node@26.6.6_typescript@7.0.2",
                        "_"
                    ),
                    Some(("ts-node", "10.9.2"))
                );
            }

            #[test]
            fn keeps_names_with_the_delimiter() {
                assert_eq!(
                    parse_name_and_version("string_decoder@1.3.0", "_"),
                    Some(("string_decoder", "1.3.0"))
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
