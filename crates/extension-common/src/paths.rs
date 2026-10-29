use moon_pdk::{VirtualPath, VirtualPathExt};

/// Return the provided path as an absolute virtual path, relative to the
/// working directory. Real paths are converted to virtual paths, otherwise
/// they can't be accessed by WASI.
pub fn get_absolute_path(working_dir: &VirtualPath, path: &str) -> VirtualPath {
    VirtualPath::create(path).unwrap_or_else(|_| working_dir.join(path))
}
