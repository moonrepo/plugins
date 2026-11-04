use moon_pdk_api::{HostArch, HostOS, Version};
use rust_toolchain::binstall::{get_binstall_archive, get_binstall_url};

mod binstall {
    use super::*;

    #[test]
    fn archive_for_supported_hosts() {
        for (os, arch, archive) in [
            (
                HostOS::Linux,
                HostArch::X64,
                "cargo-binstall-x86_64-unknown-linux-musl.tgz",
            ),
            (
                HostOS::Linux,
                HostArch::Arm64,
                "cargo-binstall-aarch64-unknown-linux-musl.tgz",
            ),
            (
                HostOS::MacOS,
                HostArch::Arm64,
                "cargo-binstall-aarch64-apple-darwin.zip",
            ),
            (
                HostOS::Windows,
                HostArch::X64,
                "cargo-binstall-x86_64-pc-windows-msvc.zip",
            ),
        ] {
            assert_eq!(get_binstall_archive(os, arch).as_deref(), Some(archive));
        }
    }

    #[test]
    fn no_archive_for_unsupported_hosts() {
        assert_eq!(get_binstall_archive(HostOS::FreeBSD, HostArch::X64), None);
        assert_eq!(get_binstall_archive(HostOS::Linux, HostArch::Arm), None);
    }

    #[test]
    fn url_for_latest_and_pinned() {
        assert_eq!(
            get_binstall_url("cargo-binstall-aarch64-apple-darwin.zip", None),
            "https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-aarch64-apple-darwin.zip"
        );
        assert_eq!(
            get_binstall_url(
                "cargo-binstall-aarch64-apple-darwin.zip",
                Some(&Version::new(1, 26, 2))
            ),
            "https://github.com/cargo-bins/cargo-binstall/releases/download/v1.26.2/cargo-binstall-aarch64-apple-darwin.zip"
        );
    }
}
