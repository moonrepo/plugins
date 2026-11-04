use moon_pdk_api::UnresolvedVersionSpec;
use rust_toolchain::version::{to_channel, to_msrv};

fn msrv(value: &str) -> Option<String> {
    to_msrv(&UnresolvedVersionSpec::parse(value).unwrap())
}

fn channel(value: &str) -> Option<String> {
    to_channel(&UnresolvedVersionSpec::parse(value).unwrap())
}

mod version {
    use super::*;

    #[test]
    fn msrv_from_versions() {
        assert_eq!(msrv("1.80.0").as_deref(), Some("1.80.0"));
        assert_eq!(msrv("1.80.1").as_deref(), Some("1.80.1"));
    }

    #[test]
    fn msrv_from_requirements() {
        assert_eq!(msrv("1.80").as_deref(), Some("1.80"));
        assert_eq!(msrv("~1.80").as_deref(), Some("1.80"));
        assert_eq!(msrv("^1.80.1").as_deref(), Some("1.80.1"));
        assert_eq!(msrv(">=1.80").as_deref(), Some("1.80"));
    }

    #[test]
    fn msrv_ignores_invalid() {
        assert_eq!(msrv("stable"), None);
        assert_eq!(msrv("beta"), None);
        assert_eq!(msrv("nightly"), None);
        assert_eq!(msrv("nightly-2024-01-01"), None);
        assert_eq!(msrv("latest"), None);
        assert_eq!(msrv("1"), None);
        assert_eq!(msrv("<1.80"), None);
        assert_eq!(msrv(">=1.80, <2"), None);
    }

    #[test]
    fn channel_from_aliases() {
        assert_eq!(channel("stable").as_deref(), Some("stable"));
        assert_eq!(channel("beta").as_deref(), Some("beta"));
        assert_eq!(channel("nightly").as_deref(), Some("nightly"));
        assert_eq!(channel("canary").as_deref(), Some("nightly"));
    }

    #[test]
    fn channel_from_dated() {
        assert_eq!(
            channel("nightly-2024-01-01").as_deref(),
            Some("nightly-2024-01-01")
        );
        assert_eq!(
            channel("beta-2024-01-01").as_deref(),
            Some("beta-2024-01-01")
        );
    }

    #[test]
    fn channel_from_versions() {
        assert_eq!(channel("1.80.0").as_deref(), Some("1.80.0"));
    }

    #[test]
    fn channel_ignores_partial_and_invalid() {
        assert_eq!(channel("1.80"), None);
        assert_eq!(channel("~1.80"), None);
        assert_eq!(channel("^1.80.1"), None);
        assert_eq!(channel("latest"), None);
        assert_eq!(channel(">=1.80, <2"), None);
    }
}
