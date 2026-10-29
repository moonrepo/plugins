# Changelog

## Unreleased

#### 💥 Breaking

- Replaced `--prefix` (which was ignored) with `--components`, which strips the provided number of leading directories from unpacked files.

#### 🐞 Fixes

- Fixed failed unpacks (invalid archives, etc) being reported as successful.
- Fixed existing files not being overwritten when unpacking zip archives.
- Fixed zip archives not being detected when the URL has a query string, or the extension is uppercase.
- Fixed zip archives requiring `unzip` on Windows. Will now use the built-in `tar` when `unzip` is not available.
- Fixed absolute `--src` and `--dest` paths failing.

## 1.0.4

#### 🚀 Updates

- Updated to support moon v2.6 release.

## 1.0.3

#### 🚀 Updates

- Updated to support moon v2.5 release.

## 1.0.2

#### 🚀 Updates

- Updated with latest moon v2 plugin APIs.

## 1.0.1

#### 🚀 Updates

- Updated with moon v2 plugin APIs.

## 1.0.0

#### 🚀 Updates

- Official major release for moon v2.
- Updated to use `unzip` and `tar` commands.

## 0.0.7

#### ⚙️ Internal

- Enabled experimental trace logging.
- Updated dependencies.

## 0.0.6

#### ⚙️ Internal

- Updated dependencies.

## 0.0.5

#### 🚀 Updates

- Updated dependencies.

## 0.0.4

#### 🚀 Updates

- Updated dependencies.

## 0.0.3

#### 🚀 Updates

- Added `register_extension` API.

## 0.0.2

#### 🚀 Updates

- Updated dependencies.

## 0.0.1

#### 🚀 Updates

- Initial release!
