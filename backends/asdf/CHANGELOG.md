# Changelog

## Unreleased

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
