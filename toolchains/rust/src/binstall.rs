use moon_pdk_api::{HostArch, HostOS, Version};

/// The prebuilt archive published to cargo-binstall's GitHub releases
/// for the host. Unsupported hosts return `None`.
pub fn get_binstall_archive(os: HostOS, arch: HostArch) -> Option<String> {
    let arch = match arch {
        HostArch::X64 => "x86_64",
        HostArch::Arm64 => "aarch64",
        _ => return None,
    };

    // Linux uses the static musl build, like binstall's own installer
    let (target, ext) = match os {
        HostOS::Linux => ("unknown-linux-musl", "tgz"),
        HostOS::MacOS => ("apple-darwin", "zip"),
        HostOS::Windows => ("pc-windows-msvc", "zip"),
        _ => return None,
    };

    Some(format!("cargo-binstall-{arch}-{target}.{ext}"))
}

pub fn get_binstall_url(archive: &str, version: Option<&Version>) -> String {
    match version {
        Some(version) => format!(
            "https://github.com/cargo-bins/cargo-binstall/releases/download/v{version}/{archive}"
        ),
        None => format!(
            "https://github.com/cargo-bins/cargo-binstall/releases/latest/download/{archive}"
        ),
    }
}
