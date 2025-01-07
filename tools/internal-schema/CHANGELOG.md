# Changelog

## Unreleased

#### 🚀 Updates

- Updated dependencies.

## 0.16.2

#### 🚀 Updates

- Added `metadata.requires`, for defining plugin requirements.

## 0.16.1

#### 🚀 Updates

- Added deprecation messages for old settings.
- Added support for custom `deprecations`.

#### 🐞 Fixes

- Brought back `platform.*.bin_path` temporarily.

## 0.16.0

#### 💥 Breaking

- Removed `platform.*.bin_path` support (use `install.exes` instead).
- Deprecated `install.primary` and `install.secondary` (use `install.exes` instead).

#### 🚀 Updates

- Added `install.exes` for defining primary and secondary executables.
- Updated to support proto v0.42 release.

## 0.15.1

#### 🚀 Updates

- Added `{versionMajor}`, `{versionMajorMinor}`, `{versionYear}`, `{versionYearMonth}`, `{versionPrerelease}`, and `{versionBuild}` tokens.

## 0.15.0

#### 🚀 Updates

- Updated to support proto v0.40 release.

## 0.14.1

#### 🚀 Updates

- Migrated to a new repository: https://github.com/moonrepo/tools

## 0.14.0

#### 🚀 Updates

- Added `platform.*.exe_dir`, which points to a relative dir that contains all pre-installed executables.
- Renamed `platform.*.bin_path` to `exe_path`.
- Updated to support proto v0.37 release.

## 0.13.1

#### 🚀 Updates

- Updated `resolve.aliases` to support ranges, requirements, and aliases.
- Updated `resolve.version-pattern` and `resolve.git-tag-pattern` to support year/month/day regex group names (for calver support).
- Updated dependencies.

## 0.13.0

#### 🚀 Updates

- Added `resolve.aliases` and `resolve.versions` settings, allowing an explicit list of aliases and versions to be defined. The versions list will be merged with other git/manifest extracted versions.
- Updated to support proto v0.36 release.

## 0.12.0

#### 🚀 Updates

- Updated to support proto v0.35 release.

## 0.11.0

#### 🚀 Updates

- Added `install.primary` setting, for advanced configuring the primary bin/shim itself. Supports the same settings as the WASM plugin.
- Added `install.secondary` setting, for configuring secondary bins/shims. Supports the same settings as the WASM plugin.
- Updated `platform.*.bin-path` to support interpolation.
- Updated to support proto v0.32 release.

## 0.10.1

#### 🚀 Updates

- Added an `install.libc` setting, to customize the libc wording used.

## 0.10.0

#### 💥 Breaking

- Removed `globals` setting. Use native installation methods instead.
- Removed deprecated `shim` setting, use `install` setting instead.

#### 🚀 Updates

- Added a `platform.*.archs` setting, to limit what architectures are supported for that OS.
- Added a `packages` setting for managing global packages. Supports `globals_lookup_dirs` and `globals_prefix` sub-settings.
- Updated to support proto v0.31 release.
- Updated dependencies.

#### ⚙️ Internal

- Updated arch/os value parsing to be more strict.

## 0.9.1

#### 🚀 Updates

- Updated our regex to support Perl syntax like `\d`.

## 0.9.0

#### 🚀 Updates

- Updated to support proto v0.29 release.
- Added `globals.bin` setting, for customizing the bin used for installs/uninstalls.

## 0.8.0

#### 💥 Breaking

- Removed deprecated functions: `locate_bins`, `create_shims`

#### 🚀 Updates

- Updated to support proto v0.28 release.
- Updated to extism-pdk v1.

## 0.7.1

#### 🚀 Updates

- Added `resolve.version-pattern` and improved regex handling.
  - Now supports named captures: `major`, `minor`, `patch`, `pre`, `build`
  - Will construct the version from the above captures.
- Deprecated `resolve.git-tag-pattern` (use the above instead).

## 0.7.0

#### 🚀 Updates

- Updated to support proto v0.26 release.

#### ⚙️ Internal

- Updated dependencies.

## 0.6.0

#### 🚀 Updates

- Updated to support proto v0.24 release.

#### ⚙️ Internal

- Updated dependencies.

## 0.5.0

#### 🚀 Updates

- Added `install.no_bin` and `install.no_shim` fields.
- Updated to support proto v0.22 release.
- Deprecated undocumented `shim` setting.

#### ⚙️ Internal

- Updated dependencies.

## 0.4.1

#### 🐞 Fixes

- Potentially fixed a WASM memory issue.

## 0.4.0

#### 🚀 Updates

- Added `install.checksum_public_key` for defining the public key used to verify checksums.
- Added `metadata.self_upgrade_commands` for defining which sub-commands should be blocked for self-upgrades.
- Updated to support proto v0.20 release.

#### ⚙️ Internal

- Updated dependencies.

## 0.3.3

#### 🐞 Fixes

- Fixed `archive-prefix` not being interpolated.

#### ⚙️ Internal

- Updated dependencies.

## 0.3.2

#### ⚙️ Internal

- Updated dependencies.

## 0.3.1

#### 🐞 Fixes

- Fixed an invalid regex pattern.

## 0.3.0

#### 🚀 Breaking

- We updated the schema internally to be represented as JSON instead of TOML, which may cause breakages depending on a version mismatch between proto and the plugin.

#### 🐞 Fixes

- Fixed version parsing from tags to be more accurate. Will now properly include prerelease/build metadata.

## 0.2.0

#### 🚀 Updates

- Added support for installing canary releases with the `install.checksum_url_canary` and `install.download_url_canary` settings.
- Updated to support proto v0.17 release.

## 0.1.1

#### 🚀 Updates

- Updated to support proto v0.16 release.

## 0.1.0

#### 🚀 Updates

- Added support for `install_global` and `uninstall_global`.
- Updated to support proto v0.15 release.

## 0.0.1

#### 🎉 Release

- Initial release!
