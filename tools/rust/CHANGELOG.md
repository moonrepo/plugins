# Changelog

## Unreleased

#### 🚀 Updates

- Updated to support proto v0.59 release.

## 0.13.8

#### 🚀 Updates

- Updated to support proto v0.55 release.

## 0.13.7

#### 🚀 Updates

- Updated to support proto v0.53 release.

## 0.13.6

#### 🚀 Updates

- Updated to support proto v0.52 release.

## 0.13.5

#### ⚙️ Internal

- Enabled experimental trace logging.
- Updated dependencies.

## 0.13.4

#### 🐞 Fixes

- Fixed Cargo/Rustup home directory detection.

## 0.13.3

#### ⚙️ Internal

- Updated dependencies.

## 0.13.2

#### 🚀 Updates

- Updated to support proto v0.47 release.

## 0.13.1

#### 🐞 Fixes

- Use the correct temporary directory when installing `rustup`.

## 0.13.0

#### 🚀 Updates

- Updated to support proto v0.46 release.

## 0.12.1

#### 🚀 Updates

- Updated dependencies.

## 0.12.0

#### 🚀 Updates

- Updated to support proto v0.42 release.

## 0.11.0

#### 🚀 Updates

- Updated to support proto v0.40 release.

## 0.10.6

#### 🚀 Updates

- Migrated to a new repository: https://github.com/moonrepo/tools

## 0.10.5

#### 🚀 Updates

- Updated to support proto v0.37 release.

## 0.10.4

#### 🐞 Fixes

- Updated our "automatic rustup installation" to respect the `CARGO_HOME` environment variable.

## 0.10.3

#### 🚀 Updates

- Updated `RUSTUP_HOME` to support relative paths.

## 0.10.2

#### 🚀 Updates

- Updated dependencies.

## 0.10.1

#### 🚀 Updates

- Updated to support proto v0.36 release.

## 0.10.0

#### 🚀 Updates

- Updated to support proto v0.35 release.

## 0.9.1

#### 🚀 Updates

- Updated to support proto v0.32 release.

## 0.9.0

#### 💥 Breaking

- Removed `install_global`, use `cargo install` instead.
- Removed `uninstall_global`, use `cargo uninstall` instead.

#### 🚀 Updates

- Updated to support proto v0.31 release.
- Updated dependencies.

## 0.8.1

#### 🐞 Fixes

- Use the full triple target when installing and uninstalling toolchains.

## 0.8.0

#### 🚀 Updates

- Updated to support proto v0.29 release.

## 0.7.1

#### 🐞 Fixes

- When auto-installing rustup, will now update `PATH` on the host to find the new binaries.

## 0.7.0

#### 💥 Breaking

- Removed deprecated functions: `locate_bins`, `create_shims`

#### 🚀 Updates

- Updated to support proto v0.28 release.
- Updated to extism-pdk v1.

#### 🐞 Fixes

- Fixed manifest syncing referencing an invalid path.

## 0.6.0

#### 🚀 Updates

- Updated to support proto v0.26 release.

#### ⚙️ Internal

- Updated dependencies.

## 0.5.0

#### 🚀 Updates

- Updated to support proto v0.24 release.

#### 🐞 Fixes

- Fixed auto-install of rustup not working on Windows.

#### ⚙️ Internal

- Updated dependencies.

## 0.4.0

#### 🚀 Updates

- Updated to support proto v0.22 release.

#### ⚙️ Internal

- Updated dependencies.

## 0.3.1

#### 🐞 Fixes

- Fixed an issue where `RUSTUP_HOME` would not respect virtual paths.

## 0.3.0

#### 🚀 Updates

- Will now attempt to install `rustup` if it does not exist on the current machine.
- Updated to support proto v0.20 release.

#### 🐞 Fixes

- Will now respect the `RUSTUP_HOME` environment variable when locating the `.rustup` store.
- Fixed an issue where `sync_manifest` would fail if we didn't have read access to the rustup store.

#### ⚙️ Internal

- Updated dependencies.

## 0.2.3

#### 🐞 Fixes

- Fixed the "bin not found" errors by uninstalling the toolchain before installing it, if we've detected that it's in a broken state.

## 0.2.2

#### 🐞 Fixes

- Temporary hack for "bin not found" errors.

## 0.2.1

#### 🐞 Fixes

- Switched to `cargo` from `rustc` for bin detection.
- Slightly improved logic that detects an installation.

#### ⚙️ Internal

- Updated dependencies.

## 0.2.0

#### 🚀 Updates

- Added support for installing the canary release (via nightly).
- Updated to support proto v0.17 release.

## 0.1.2

#### 🚀 Updates

- Updated to support proto v0.16 release.

## 0.1.1

#### 🐞 Fixes

- Fixed `rustup` detection on Windows.

## 0.1.0

#### 🚀 Updates

- Added uninstall support (uses `rustup toolchain uninstall`).
- Added support for `install_global` and `uninstall_global`.
- Updated to support proto v0.15 release.

#### 🐞 Fixes

- Fixed an issue where the globals directory for `CARGO_INSTALL_ROOT` was incorrect.

## 0.0.3

#### 🐞 Fixes

- Fixed an issue where the `rustc` binary wasn't properly located on Windows.

## 0.0.2

#### 🚀 Updates

- Supports automatic manifest syncing (keeps track of installed versions).

## 0.0.1

#### 🎉 Release

- Initial release!
