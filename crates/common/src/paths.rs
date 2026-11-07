use proto_pdk::{AnyResult, get_host_env_var};
use std::path::{Path, PathBuf};

fn is_bin_dir(path: &Path) -> bool {
    // Host paths may be Windows paths, which WASM doesn't split on `\`
    path.to_string_lossy()
        .trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        == Some("bin")
}

/// Append a `bin` directory to the path, unless its last component already is one,
/// like a `DENO_INSTALL_ROOT` of `~/.deno/bin`.
pub fn append_bin_dir(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();

    if is_bin_dir(path) {
        path.to_path_buf()
    } else {
        path.join("bin")
    }
}

/// Like [`append_bin_dir`], but for the directory in an environment variable,
/// returning a `$NAME` or `$NAME/bin` lookup that proto expands on the host.
pub fn append_env_bin_dir(name: &str) -> AnyResult<String> {
    Ok(
        if get_host_env_var(name)?.is_some_and(|dir| is_bin_dir(Path::new(&dir))) {
            format!("${name}")
        } else {
            format!("${name}/bin")
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_bin() {
        assert_eq!(append_bin_dir("/opt/deno"), PathBuf::from("/opt/deno/bin"));
        assert_eq!(
            append_bin_dir("/opt/binary"),
            PathBuf::from("/opt/binary/bin")
        );
        assert_eq!(append_bin_dir("C:\\deno"), PathBuf::from("C:\\deno/bin"));
    }

    #[test]
    fn keeps_existing_bin() {
        assert_eq!(
            append_bin_dir("/opt/deno/bin"),
            PathBuf::from("/opt/deno/bin")
        );
        assert_eq!(
            append_bin_dir("/opt/deno/bin/"),
            PathBuf::from("/opt/deno/bin/")
        );
        assert_eq!(
            append_bin_dir("C:\\deno\\bin"),
            PathBuf::from("C:\\deno\\bin")
        );
        assert_eq!(append_bin_dir("bin"), PathBuf::from("bin"));
    }
}
