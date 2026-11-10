# Changelog

## Unreleased

#### 💥 Breaking

- `packageManager` (`bun@x`) in `package.json` now takes precedence over `engines.bun`, like
  setup-bun, as an exact version should win over a range.

#### 🚀 Updates

- `$BUN_INSTALL` is now used as-is for the globals directory when it already is a `bin` directory,
  instead of appending another `bin`.
- Added support for FreeBSD (x64 and arm64), which Bun added in v1.3.14.
- AVX2 support is now detected from the x86-64 level that proto detects (level 3 and above),
  instead of running `grep` or `sysctl`. Older versions of proto still run them.
- The `-baseline` build is now recorded as the `baseline` variant in the lockfile, so machines with
  and without AVX2 each verify their own build's checksum, instead of failing the other's.

#### 🐞 Fixes

- When proto locks another platform (`proto lock --platform`), the regular build is now used for x64,
  instead of checking AVX2 support on the machine running proto, which recorded the `-baseline`
  build when locking `x64-linux` from a machine without AVX2, like an arm64 Mac.
- Fixed pinning when `.bumrc` or `.bun-version` exists, which overrode the `package.json` it wrote
  to. The existing version file is now updated (and removed when unpinning). `package.json` pins now
  only write `devEngines.runtime`, as also writing `devEngines.packageManager` turned another package
  manager's entry into a list.
- Fixed pinning an alias, like `lts` or `latest`, writing it to `devEngines` in `package.json`, which
  only supports versions and ranges. It now errors instead.
- Fixed a comment after the version in `.bun-version` or `.bumrc`, like `1.3 # stable`, failing
  to parse. Everything after `#` is now ignored.
- Fixed Intel macOS machines without AVX2 (and x64 under Rosetta) installing a Bun build that
  crashes on start. They now install the baseline build, like Bun's own installer. Only affects
  versions before v1.4, where every x64 build became baseline.
- Fixed globals directories not including `$BUN_INSTALL_BIN`, `$BUN_INSTALL/bin`, and
  `$XDG_CACHE_HOME/.bun/bin`, which `bun add --global` uses when set.

#### ⚙️ Internal

- Version files are now read from their path, as the `content` input is deprecated.

## 0.16.11

#### 🚀 Updates

- Added support for Windows arm64, which Bun added in v1.3.10.
- Added support for musl based Linux, which Bun added in v1.1.35. Older versions
  will now error instead of installing a glibc build that can't run.

#### 🐞 Fixes

- Fixed the Windows support check, which would exclude future major versions,
  like v2.0.

## 0.16.10

#### 🚀 Updates

- Updated to support proto v0.60 release.

## 0.16.9

#### 🚀 Updates

- Updated to support proto v0.59 release.

## 0.16.8

#### 🚀 Updates

- Respect `.editorconfig` when writing `package.json` files.

## 0.16.7

#### 🐞 Fixes

- Fixed an issue where `package.json` properties would be re-ordered while saving.

## 0.16.6

#### 🚀 Updates

- Updated to support proto v0.55 release.
- Added `pin_version` and `unpin_version` support, which maps to `package.json` `devEngines`.

#### 🐞 Fixes

- Fixed some `package.json` version parsing issues.

## 0.16.5

#### 🐞 Fixes

- Fixed an issue where major versions only (`18`) would not parse correctly.

## 0.16.4

#### 🚀 Updates

- Added `package.json` `devEngines.runtime` support for version detection.

## 0.16.3

#### 🚀 Updates

- Updated to support proto v0.53 release.

## 0.16.2

#### 🐞 Fixes

- Fixed some `package.json` parsing issues.

## 0.16.1

#### ⚙️ Internal

- Enabled experimental trace logging.
- Updated dependencies.

## 0.16.0

#### 🚀 Updates

- Added detection sources: `.bumrc`, `.bun-version`, `package.json` (volta, engines, packageManager)

## 0.15.2

#### ⚙️ Internal

- Updated dependencies.

## 0.15.1

#### 🚀 Updates

- Updated to support proto v0.47 release.

## 0.15.0

#### 🚀 Updates

- Updated to support proto v0.46 release.

## 0.14.1

#### 🚀 Updates

- Updated dependencies.

## 0.14.0

#### 🚀 Updates

- Updated to support proto v0.42 release.

## 0.13.0

#### 🚀 Updates

- Updated to support proto v0.40 release.

## 0.12.3

#### 🚀 Updates

- Migrated to a new repository: https://github.com/moonrepo/tools

## 0.12.2

#### 🚀 Updates

- Updated dependencies.

## 0.12.1

#### 🚀 Updates

- Updated to support proto v0.36 release.

## 0.12.0

#### 🚀 Updates

- Updated to support proto v0.35 release.

## 0.11.1

#### 🚀 Updates

- Added a `dist-url` config setting, allowing the download host to be customized.

## 0.11.0

#### 🚀 Updates

- Added Windows support.
- Will now use the baseline build on x64 Linux when available.

## 0.10.1

#### 🚀 Updates

- Updated to support proto v0.32 release.

## 0.10.0

#### 💥 Breaking

- Removed `install_global`, use `bun add --global` instead.
- Removed `uninstall_global`, use `bun remove --global` instead.

#### 🚀 Updates

- Updated to support proto v0.31 release.
- Updated dependencies.

## 0.9.0

#### 🚀 Updates

- Updated to support proto v0.29 release.

## 0.8.0

#### 💥 Breaking

- Removed deprecated functions: `locate_bins`, `create_shims`

#### 🚀 Updates

- Updated to support proto v0.28 release.
- Updated to extism-pdk v1.

## 0.7.0

#### 🚀 Updates

- Updated to support proto v0.26 release.
- Will now symlink a `bunx` binary to `~/.proto/bin`.
- The shim will continue to use `bun x` under the hood (note the space).

#### ⚙️ Internal

- Updated dependencies.

## 0.6.0

#### 🚀 Updates

- Updated to support proto v0.24 release.

#### ⚙️ Internal

- Updated dependencies.

## 0.5.0

#### 🚀 Updates

- Updated to support proto v0.22 release.

#### ⚙️ Internal

- Updated dependencies.

## 0.4.0

#### 🚀 Updates

- Updated to support proto v0.20 release.

#### ⚙️ Internal

- Updated dependencies.

## 0.3.1

#### ⚙️ Internal

- Updated dependencies.

## 0.3.0

#### 🚀 Updates

- Added support for installing the [canary release](https://github.com/oven-sh/bun/releases/tag/canary).
- Updated to support proto v0.17 release.

## 0.2.1

#### 🚀 Updates

- Updated to support proto v0.16 release.

## 0.2.0

#### 🚀 Updates

- Added support for `install_global` and `uninstall_global`.
- Updated to support proto v0.15 release.

## 0.1.0

#### 🚀 Updates

- Updated to support proto v0.14 release.

## 0.0.3

#### 🚀 Updates

- Improved OS/arch detection and combination logic.

## 0.0.2

#### 🚀 Updates

- Updated `load_versions` to use `git` instead of the GitHub API.
- Requires proto >= v0.12.1.

## 0.0.1

#### 🎉 Release

- Initial release!
