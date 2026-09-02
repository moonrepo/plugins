use extism_pdk::*;
use proto_pdk::*;

/// Platform identifier and archive extension for the host, in Microsoft's
/// RID vocabulary. The archive URL is
/// `.../Sdk/{version}/dotnet-sdk-{version}-{rid}.{extension}`.
pub fn target_rid(env: &HostEnvironment) -> FnResult<(String, &'static str)> {
    let rid = match env.os {
        HostOS::Windows => match env.arch {
            HostArch::X64 => "win-x64",
            HostArch::X86 => "win-x86",
            HostArch::Arm64 => "win-arm64",
            _ => return Err(unsupported_arch(env)),
        },
        HostOS::MacOS => match env.arch {
            HostArch::X64 => "osx-x64",
            HostArch::Arm64 => "osx-arm64",
            _ => return Err(unsupported_arch(env)),
        },
        HostOS::Linux => {
            // Alpine and other musl distributions get their own builds; the
            // glibc archives will not run there.
            let prefix = if matches!(env.libc, HostLibc::Musl) {
                "linux-musl"
            } else {
                "linux"
            };

            let arch = match env.arch {
                HostArch::X64 => "x64",
                HostArch::Arm64 => "arm64",
                HostArch::Arm => "arm",
                _ => return Err(unsupported_arch(env)),
            };

            return Ok((format!("{prefix}-{arch}"), "tar.gz"));
        }
        _ => {
            return Err(plugin_err!(PluginError::UnsupportedOS {
                tool: "dotnet".into(),
                os: env.os.to_string(),
            }));
        }
    };

    let extension = if env.os.is_windows() { "zip" } else { "tar.gz" };

    Ok((rid.to_owned(), extension))
}

fn unsupported_arch(env: &HostEnvironment) -> WithReturnCode<Error> {
    plugin_err!(PluginError::UnsupportedArch {
        tool: "dotnet".into(),
        arch: env.arch.to_string(),
    })
}
