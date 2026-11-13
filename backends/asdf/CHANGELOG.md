# Changelog

## Unreleased

#### 💥 Breaking

- Requires proto v0.64 or later.

#### 🚀 Updates

- Updated to support proto v0.64 release.
- Versions with a 4th number, like `1.2.3.4`, are now listed, as proto supports a revision. They
  were skipped.
- Improved the error when a tool is not in the asdf shortname index, to point to the `shortname` and `repository` settings.
- Panics now report their message, instead of an `unreachable` WASM trap error.

#### 🐞 Fixes

- Fixed the `latest` version resolving to a pre-release, as it was not set and proto used the highest version.
- Fixed `bin/latest-stable` not receiving an (empty) filter query argument like asdf, which failed scripts that read `$1` (asdf-golang).
- Fixed only the first `bin/list-bin-paths` directory being scanned for executables (asdf-golang's `go` was missing), and directories being returned as executables.
- Fixed `.tool-versions` lines with fallback versions or extra whitespace failing to parse. The first version is used, and lines are also matched by the `shortname` setting.
- Fixed `bin/exec-env` multi-line variables being split into bogus variables. The script is now always sourced with bash, like asdf.
- Fixed `ASDF_CONCURRENCY` overriding the variable when already set in the environment.
- Fixed a version file outside of the directories that proto can access panicking instead of returning an error.
- Fixed a failed `nproc` or `sysctl` setting an empty `ASDF_CONCURRENCY`. It now falls back to 1.

#### ⚙️ Internal

- Version files are now read from their path, as the `content` input is deprecated.

## 0.3.5

#### 🚀 Updates

- Updated to support proto v0.60 release.

## 0.3.4

#### 🚀 Updates

- Updated to support proto v0.59 release.

## 0.3.3

#### 🚀 Updates

- Updated to support proto v0.55 release.

## 0.3.2

#### 🚀 Updates

- Updated to support proto v0.52 release.

## 0.3.1

#### ⚙️ Internal

- Enabled experimental trace logging.
- Updated dependencies.

## 0.3.0

#### 🚀 Updates

- Updated the backend ID/path to `asdf/<tool>` instead of `asdf-<tool>`.
- Updated scripts to extract the command/shell to execute with from its shebang.

#### ⚙️ Internal

- Updated dependencies.

## 0.2.1

#### ⚙️ Internal

- Updated dependencies.

## 0.2.0

#### 🚀 Updates

- Added `exec-env` experimental support. Runs as a `pre-run` hook to extract any set environment variables.
- Added `latest-stable` script support when the alias "stable" is used for a version.
- Reduced the amount of calls made for converting `/proto/backends` virtual paths into a real path.

#### 🐞 Fixes

- Ensure an executable is always returned, even if invalid.

## 0.1.2

#### 🐞 Fixes

- Fixed an issue where non-executable bins were being returned. We do our best to filter this list.

## 0.1.1

#### 🐞 Fixes

- Fixed some issues when the plugin is used as "asdf" directly.

## 0.1.0

#### 🎉 Release

- Initial release!
