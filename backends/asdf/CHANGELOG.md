# Changelog

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
