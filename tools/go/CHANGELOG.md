# Changelog

## Unreleased

#### 🚀 Updates

- Updated dependencies.

## 0.14.0

#### 🚀 Updates

- Updated to support proto v0.42 release.

## 0.13.0

#### 🚀 Updates

- Updated to support proto v0.40 release.

## 0.12.0

#### 💥 Breaking

- Changed the `gobin` setting to `false` by default.

## 0.11.4

#### 🚀 Updates

- Migrated to a new repository: https://github.com/moonrepo/tools

#### 🐞 Fixes

- Fixed `go.mod`/`go.work` version detection/parsing on Windows.

## 0.11.3

#### 🚀 Updates

- Updated dependencies.

## 0.11.2

#### 🚀 Updates

- Updated to support proto v0.36 release.

## 0.11.1

#### 🚀 Updates

- Added `gofmt` as a secondary shim/binary.
- Updated `go.mod` version parsing to use better ranges.

## 0.11.0

#### 🚀 Updates

- Updated to support proto v0.35 release.

## 0.10.2

#### 🚀 Updates

- Added a `dist-url` config setting, allowing the download host to be customized.

## 0.10.1

#### 🚀 Updates

- Updated to support proto v0.32 release.

## 0.10.0

#### 💥 Breaking

- Removed `install_global`, use `go install` instead.
- Removed `uninstall_global`.

#### 🚀 Updates

- Updated to support proto v0.31 release.
- Updated dependencies.

## 0.9.0

#### 🚀 Updates

- Updated to support proto v0.29 release.

## 0.8.0

#### 💥 Breaking

- Removed deprecated functions: `locate_bins`

#### 🚀 Updates

- Updated to support proto v0.28 release.
- Updated to extism-pdk v1.

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

#### 🐞 Fixes

- Fixed an issue where parsing `go.mod` or `go.work` would fully expand a partial version. For example, `1.19` would be fixed to `1.19.0` instead of `1.19.*`.

## 0.3.0

#### 🚀 Updates

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
- Updated to support Go's new versioning scheme starting with v1.21.0.

## 0.0.3

#### 🚀 Updates

- Improved OS/arch detection and combination logic.

## 0.0.2

#### 🐞 Fixes

- Fixed some edge cases with version parsing/formatting.

## 0.0.1

#### 🎉 Release

- Initial release!
