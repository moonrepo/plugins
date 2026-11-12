# Changelog

## Unreleased

#### 💥 Breaking

- The `canary` version now errors. moon stopped publishing canary builds in v2, and the old `canary`
  release on GitHub only has a moon v1 build from December 2025, which was installed instead.

#### 🚀 Updates

- Updated to support proto v0.64 release.
- `moon upgrade` is no longer blocked by proto. moon v2.2.2+ detects that proto installed it, and
  upgrades with `proto install moon latest --pin local`.

#### 🐞 Fixes

- Fixed building moon v1 from source, which tried to build a `moonx` binary that only exists in v2.
- Fixed building moon v2.2+ from source without `protoc`, which it requires. It's now installed with
  apt or Homebrew when missing.
- moon v2.0 and v2.1 now error as unsupported on macOS x64, instead of failing to download a build
  that was never published.

## 0.4.3

#### 🚀 Updates

- Updated to support proto v0.60 release.

## 0.4.2

#### 🚀 Updates

- Updated to support proto v0.59 release.

## 0.4.1

#### 🚀 Updates

- Updated to support proto v0.55 release.

## 0.4.0

#### 🚀 Updates

- Added support for moon v2+'s new distribution format.
  - Binaries are now distributed as archives.
  - Supports checksum files.
  - Supports the `moonx` binary.

## 0.3.3

#### ⚙️ Internal

- Enabled experimental trace logging.
- Updated dependencies.

## 0.3.2

#### ⚙️ Internal

- Updated dependencies.

## 0.3.1

#### 🚀 Updates

- Updated to support proto v0.47 release.

## 0.3.0

#### 🐞 Fixes

- Fixed the wrong binary name being used when building from source.

## 0.2.0

#### 🚀 Updates

- Updated to support proto v0.46 release.

## 0.1.0

#### 🚀 Updates

- Added build from source support.

## 0.0.2

#### 🎉 Release

- Initial release!
