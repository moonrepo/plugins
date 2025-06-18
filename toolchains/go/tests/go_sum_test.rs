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
                    GoSumDependency {
                        checksum: "h1:EH0zSVneZPSuFR11BlR9YppQTVDbh5+16AmcJi4g1z4=".into(),
                        version: VersionSpec::parse("0.1.4").unwrap()
                    }
                ),
                (
                    "github.com/charmbracelet/bubbles".into(),
                    GoSumDependency {
                        checksum: "h1:9TdC97SdRVg/1aaXNVWfFH3nnLAwOXr8Fn6u6mfQdFs=".into(),
                        version: VersionSpec::parse("0.21.0").unwrap()
                    }
                ),
                (
                    "github.com/charmbracelet/bubbletea".into(),
                    GoSumDependency {
                        checksum: "h1:JAMNLTbqMOhSwoELIr0qyP4VidFq72/6E9j7HHmRKQc=".into(),
                        version: VersionSpec::parse("1.3.5").unwrap()
                    }
                )
            ])
        );
    }

    #[test]
    fn parses_advanced() {
        let sandbox = create_sandbox("sum-files");
        let go_sum =
            GoSum::parse(fs::read_to_string(sandbox.path().join("advanced.sum")).unwrap()).unwrap();

        dbg!(&go_sum);

        assert_eq!(
            go_sum.dependencies,
            BTreeMap::from_iter([
                (
                    "github.com/atotto/clipboard".into(),
                    GoSumDependency {
                        checksum: "h1:EH0zSVneZPSuFR11BlR9YppQTVDbh5+16AmcJi4g1z4=".into(),
                        version: VersionSpec::parse("0.1.4").unwrap()
                    }
                ),
                (
                    "github.com/charmbracelet/bubbles".into(),
                    GoSumDependency {
                        checksum: "h1:9TdC97SdRVg/1aaXNVWfFH3nnLAwOXr8Fn6u6mfQdFs=".into(),
                        version: VersionSpec::parse("0.21.0").unwrap()
                    }
                ),
                (
                    "github.com/charmbracelet/bubbletea".into(),
                    GoSumDependency {
                        checksum: "h1:JAMNLTbqMOhSwoELIr0qyP4VidFq72/6E9j7HHmRKQc=".into(),
                        version: VersionSpec::parse("1.3.5-abc+123").unwrap()
                    }
                )
            ])
        );
    }
}
