use extism_pdk::*;
use proto_pdk::*;
use std::path::PathBuf;

#[host_fn]
extern "ExtismHost" {
    fn get_env_var(name: String) -> String;
    fn to_virtual_path(input: String) -> String;
}

fn get_home_env(key: &str) -> Result<Option<VirtualPath>, Error> {
    match get_host_env_var(key)? {
        Some(value) => {
            if value.is_empty() {
                return Ok(None);
            }

            let path = PathBuf::from(value);

            // Variable returns a real path, so convert to virtual
            let path = if path.is_absolute() {
                into_virtual_path(path)?
            } else {
                into_virtual_path("/cwd")?.join(path)
            };

            Ok(Some(path))
        }
        None => Ok(None),
    }
}

pub fn get_cargo_home(env: &HostEnvironment) -> Result<VirtualPath, Error> {
    Ok(get_home_env("CARGO_HOME")?.unwrap_or_else(|| env.home_dir.join(".cargo")))
}

pub fn get_rustup_home(env: &HostEnvironment) -> Result<VirtualPath, Error> {
    // Cargo sets the RUSTUP_HOME env var when running tests,
    // which causes a ton of issues, so intercept it here!
    if let Some(test_env) = get_test_environment()? {
        return Ok(test_env.sandbox.join(".home/.rustup"));
    }

    Ok(get_home_env("RUSTUP_HOME")?.unwrap_or_else(|| env.home_dir.join(".rustup")))
}

pub fn get_channel_from_version(spec: &VersionSpec) -> String {
    if spec.is_canary() {
        "nightly".to_owned()
    } else {
        spec.to_string()
    }
}

pub fn is_non_version_channel(spec: &UnresolvedVersionSpec) -> bool {
    match spec {
        UnresolvedVersionSpec::Canary => true,
        UnresolvedVersionSpec::Alias(value) => {
            value == "stable"
                || value == "beta"
                || value == "nightly"
                || value.starts_with("nightly")
        }
        _ => false,
    }
}
