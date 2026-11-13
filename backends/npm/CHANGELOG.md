# Changelog

## 0.2.0

#### 💥 Breaking

- Requires proto v0.64 or later.

#### 🚀 Updates

- Updated to support proto v0.64 release.
- Panics now report their message, instead of an `unreachable` WASM trap error.

#### 🐞 Fixes

- Fixed the `latest` version resolving to a pre-release when the `latest` dist-tag points to one. It now falls back to the highest stable version.
- Fixed extensionless bins that are native binaries or non-Node scripts (like `esbuild` after its postinstall) being run with `node`/`bun`. The shebang is now checked.
- Fixed the package's install scripts not running with npm v12, which blocks them by default. The package is now passed to `--allow-scripts`.
- Fixed a cryptic JSON error when loading versions fails (for example, an unknown package). The npm/bun error is now shown.

## 0.1.4

#### 🐞 Fixes

- Fixed loading versions failing with npm v12, as `npm view --json` now wraps objects in an array.

## 0.1.3

#### 🚀 Updates

- Updated to support proto v0.60 release.

## 0.1.2

#### 🚀 Updates

- Updated to support proto v0.59 release.

## 0.1.1

#### 🐞 Fixes

- Fixed extensionless bins not running via `node`.

## 0.1.0

#### 🎉 Release

- Initial release!
