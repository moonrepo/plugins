# Changelog

## Unreleased

#### 🚀 Updates

- `CARGO_INSTALL_ROOT` and `CARGO_HOME` are now used as-is for the globals directories when they
  already are a `bin` directory, instead of appending another `bin`.
- **Behavior change:** When `version` is a fully-qualified version or channel, tasks now run with
  that toolchain by setting `RUSTUP_TOOLCHAIN`, as rustup ignores moon's `PROTO_RUST_VERSION`. This
  overrides `rust-toolchain.toml` and `rustup override` for moon tasks; enable
  `syncToolchainConfig` to keep editors and ad-hoc `cargo` in step. A `RUSTUP_TOOLCHAIN` set in
  your shell still takes precedence, but one set in a task's `env` is overwritten.
- **Behavior change:** Bare task commands are no longer rewritten to `cargo <command>` when the
  command exists on its own. For example, `make` stays `make` when `cargo-make` is installed, while
  `nextest` still becomes `cargo nextest`.
- `components` and `targets` are now installed for the configured toolchain with `--toolchain`.
- `syncToolchainConfig` now also merges `components` and `targets` into `rust-toolchain.toml`, so
  rustup installs them for editors and ad-hoc `cargo` too. Existing entries are kept.
- `cargo-binstall` is now installed from its prebuilt GitHub release instead of being compiled with
  `cargo install`, which took minutes on cold CI runners. It falls back to `cargo install` when the
  download or unpacking (with `tar`) fails, or on hosts other than x64 and arm64 Linux, macOS, and
  Windows.
- Panics now report their message, instead of an `unreachable` WASM trap error.

#### 🐞 Fixes

- Fixed `addMsrvConstraint` and `syncToolchainConfig` writing values that Cargo and rustup reject,
  like `rust-version = "stable"` or `channel = "~1.80"`. Versions that can't be written are now
  skipped with a warning, including partial versions like `1.80` for `syncToolchainConfig`, as
  rustup treats them as a separate toolchain.
- Fixed `addMsrvConstraint` only setting `package.rust-version` when the root `Cargo.toml` has both
  `[workspace]` and `[package]`.
- Fixed project relationships not being inferred from `workspace = true`, renamed
  (`package = "..."`), and `[target.*]` path dependencies.
- Fixed `docker prune` deleting binaries in target triple and custom profile directories, and
  ignoring `CARGO_TARGET_DIR` and `build.target-dir`. A separate `build-dir` is now removed too.
- Fixed the default Docker image for channels, as `rust:stable` and others don't exist. Stable and
  beta now use `rust:latest`, and nightly uses `rustlang/rust:nightly`.
- Fixed `.cargo/config` (without an extension) not being detected as a config file.
- Fixed `CARGO_HOME/bin` not being added to `PATH` when `CARGO_INSTALL_ROOT` is set.
- Fixed bare dependency requirements (`1.2.3`) being parsed as exact versions instead of caret
  requirements, so task hashes with `hasher.optimization: accuracy` now track the resolved version
  from `Cargo.lock`. `[target.*]` dependencies are now included as well.
- Includes `rust_tool` fixes for toolchain file precedence and Windows ARM64.

## 1.0.12

#### 🐞 Fixes

- Fixed `docker prune` failing with "unknown variant `custom-build`" when a crate has a build
  script, example, bench, or other unsupported target kind.
- Fixed `docker scaffold` leaving an empty `src/main.rs` in library crates, which failed builds
  with "`main` function not found". Only the crate's existing lib and bin targets (including
  `src/bin`) are now stubbed.

## 1.0.11

#### 🚀 Updates

- Added `bins.*.args` support, which are passed to `cargo binstall`. Bins with args are installed in separate commands.

## 1.0.10

#### 🚀 Updates

- Updated to support moon v2.6 release.

## 1.0.9

#### 🐞 Fixes

- Fixed configured `bins` not being reinstalled when their binaries were
  uninstalled or deleted outside of moon. Only missing binaries are now
  installed.

## 1.0.8

#### 🚀 Updates

- Updated to support moon v2.5 release.

## 1.0.7

#### 🐞 Fixes

- Fixed an issue where Docker scaffolding would leave behind empty `lib.rs` or `main.rs` files.

## 1.0.6

#### 🐞 Fixes

- Updated `cargo-binstall` installation to use `--locked`.

## 1.0.5

#### 🚀 Updates

- Updated `Cargo.lock` parsing dependencies.

## 1.0.4

#### 🚀 Updates

- Updated with latest moon v2 plugin APIs.

## 1.0.3

#### 🐞 Fixes

- API compatibility.

## 1.0.2

#### 🚀 Updates

- Updated with moon v2 plugin APIs.

## 1.0.1

#### 🐞 Fixes

- Fixed an issue where dependency self-cycles (using `path = "."`) would be included in graph extension.

## 1.0.0

#### 🚀 Updates

- Official major release for moon v2.

## 0.3.0

#### 🚀 Updates

- Updated to support moon v1.41.

## 0.2.4

#### 🚀 Updates

- Removed globals directory injection as this happens in moon directly.

## 0.2.3

#### 🚀 Updates

- Updated manifest parsing to extract `path` and `git` values.

#### 🐞 Fixes

- Fixed invalid versions when creatin the Docker image name.

## 0.2.2

#### 🐞 Fixes

- Fixed a "wasm `unreachable` instruction executed" error.

## 0.2.1

#### ⚙️ Internal

- Enabled experimental trace logging.
- Updated dependencies.

## 0.2.0

#### 🚀 Updates

- Cached the globals bin directory when extending task commands/scripts.
- Task hashing now includes the host OS, arch, and libc.

#### ⚙️ Internal

- Updated dependencies.

## 0.1.2

#### 🐞 Fixes

- Fixed `cargo-binstall` failing in CI when the binary already exists.

## 0.1.1

#### 🐞 Fixes

- Fixed Cargo/Rustup home directory detection.

## 0.1.0

#### 🚀 Updates

- Initial release!
