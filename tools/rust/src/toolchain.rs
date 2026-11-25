use std::path::Path;

/// Whether the rustup toolchain in `dir` has `cargo`, the binary that proto locates. A `bin`
/// folder alone is not enough: proto creates the toolchain's folder before it installs it, and
/// rustup then treats the folder as an installed toolchain, so when a `rust-toolchain.toml`
/// selects it, rustup only adds that file's components and targets.
pub fn has_cargo(dir: &Path, cargo_exe: &str) -> bool {
    dir.join("bin").join(cargo_exe).exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn has_cargo_when_bin_contains_it() {
        let dir = starbase_sandbox::create_empty_sandbox();
        fs::create_dir_all(dir.path().join("bin")).unwrap();
        fs::write(dir.path().join("bin/cargo"), "").unwrap();

        assert!(has_cargo(dir.path(), "cargo"));
    }

    #[test]
    fn has_no_cargo_when_bin_has_only_components() {
        let dir = starbase_sandbox::create_empty_sandbox();
        fs::create_dir_all(dir.path().join("bin")).unwrap();
        fs::write(dir.path().join("bin/cargo-clippy"), "").unwrap();
        fs::write(dir.path().join("bin/clippy-driver"), "").unwrap();

        assert!(!has_cargo(dir.path(), "cargo"));
    }
}
