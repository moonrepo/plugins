# Changelog

## Unreleased

#### 🐞 Fixes

- Fixed the `latest` version resolving to a pre-release when the `latest` dist-tag points to one. It now falls back to the highest stable version.

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
