use go_toolchain::go_sum::*;
use moon_config::VersionSpec;
use starbase_sandbox::create_sandbox;
use std::collections::BTreeMap;
use std::fs;

mod go_sum {
    use super::*;

    #[test]
    fn parses_basic() {
        let sandbox = create_sandbox("sum-files");
        let go_sum =
            GoSum::parse(fs::read_to_string(sandbox.path().join("basic.sum")).unwrap()).unwrap();

        assert_eq!(
            go_sum.dependencies,
            BTreeMap::from_iter([
                (
                    "github.com/atotto/clipboard".into(),
                    vec![GoSumDependency {
                        checksum: "h1:EH0zSVneZPSuFR11BlR9YppQTVDbh5+16AmcJi4g1z4=".into(),
                        version: VersionSpec::parse("0.1.4").unwrap()
                    }]
                ),
                (
                    "github.com/charmbracelet/bubbles".into(),
                    vec![GoSumDependency {
                        checksum: "h1:9TdC97SdRVg/1aaXNVWfFH3nnLAwOXr8Fn6u6mfQdFs=".into(),
                        version: VersionSpec::parse("0.21.0").unwrap()
                    }]
                ),
                (
                    "github.com/charmbracelet/bubbletea".into(),
                    vec![GoSumDependency {
                        checksum: "h1:JAMNLTbqMOhSwoELIr0qyP4VidFq72/6E9j7HHmRKQc=".into(),
                        version: VersionSpec::parse("1.3.5").unwrap()
                    }]
                )
            ])
        );
    }

    #[test]
    fn parses_basic_without_trailing() {
        let sandbox = create_sandbox("sum-files");

        GoSum::parse(
            fs::read_to_string(sandbox.path().join("basic.sum"))
                .unwrap()
                .trim(),
        )
        .unwrap();
    }

    #[test]
    fn parses_advanced() {
        let sandbox = create_sandbox("sum-files");
        let go_sum =
            GoSum::parse(fs::read_to_string(sandbox.path().join("advanced.sum")).unwrap()).unwrap();

        assert_eq!(
            go_sum.dependencies,
            BTreeMap::from_iter([
                (
                    "github.com/atotto/clipboard".into(),
                    vec![GoSumDependency {
                        checksum: "h1:EH0zSVneZPSuFR11BlR9YppQTVDbh5+16AmcJi4g1z4=".into(),
                        version: VersionSpec::parse("0.1.4").unwrap()
                    }]
                ),
                (
                    "github.com/charmbracelet/bubbles".into(),
                    vec![GoSumDependency {
                        checksum: "h1:9TdC97SdRVg/1aaXNVWfFH3nnLAwOXr8Fn6u6mfQdFs=".into(),
                        version: VersionSpec::parse("0.21.0").unwrap()
                    }]
                ),
                (
                    "github.com/charmbracelet/bubbletea".into(),
                    vec![GoSumDependency {
                        checksum: "h1:JAMNLTbqMOhSwoELIr0qyP4VidFq72/6E9j7HHmRKQc=".into(),
                        version: VersionSpec::parse("1.3.5-abc+123").unwrap()
                    }]
                )
            ])
        );
    }

    #[test]
    fn parses_advanced_without_trailing() {
        let sandbox = create_sandbox("sum-files");

        GoSum::parse(
            fs::read_to_string(sandbox.path().join("advanced.sum"))
                .unwrap()
                .trim(),
        )
        .unwrap();
    }

    #[test]
    fn keeps_every_version() {
        let go_sum = GoSum::parse(
            "example.com/a v1.0.0 h1:old=\nexample.com/a v1.0.0/go.mod h1:oldmod=\nexample.com/a v1.1.0 h1:new=\n",
        )
        .unwrap();

        assert_eq!(
            go_sum.dependencies["example.com/a"],
            [
                GoSumDependency {
                    checksum: "h1:old=".into(),
                    version: VersionSpec::parse("1.0.0").unwrap()
                },
                GoSumDependency {
                    checksum: "h1:new=".into(),
                    version: VersionSpec::parse("1.1.0").unwrap()
                }
            ]
        );
    }

    #[test]
    fn keeps_hashes_containing_double_slashes() {
        let go_sum = GoSum::parse(
            "golang.org/x/net v0.10.0 h1:X2//UzNDwYmtCLn7To6G58Wr6f5ahEAQgKNzv9Y951M= // Comment\n",
        )
        .unwrap();

        assert_eq!(
            go_sum.dependencies["golang.org/x/net"][0].checksum,
            "h1:X2//UzNDwYmtCLn7To6G58Wr6f5ahEAQgKNzv9Y951M="
        );
    }

    #[test]
    fn skips_malformed_lines_and_handles_crlf() {
        let go_sum = GoSum::parse(
            "example.com/a v1.0.0\r\nexample.com/b v1.0.0 h1:b= extra\r\nexample.com/c v1.0.0 h1:c=\r\n",
        )
        .unwrap();

        assert_eq!(
            go_sum.dependencies.keys().collect::<Vec<_>>(),
            ["example.com/c"]
        );
    }
}
