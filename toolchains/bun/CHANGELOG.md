# Changelog

## Unreleased

#### 🚀 Updates

- Panics now report their message, instead of an `unreachable` WASM trap error.
- `.bumrc` and `.bun-version` are now config files, so they're copied into the Docker configs
  skeleton.

#### 🐞 Fixes

- Fixed a Bun in `~/.bun/bin`, where Bun's installer and `setup-bun` put it, running moon's
  installs and tasks instead of the configured `version`.
- Fixed the default Docker image for a version bound being the oldest allowed version, like
  `oven/bun:1.2.0` for `>=1.2.0`, or one that doesn't exist, like `oven/bun:2` for `<2`. It's now
  `oven/bun:latest`.

## 1.0.5

#### 🚀 Updates

- Updated to support moon v2.6 release.

## 1.0.4

#### 🚀 Updates

- Updated the embedded `bun` tool to support Windows arm64 (Bun v1.3.10+) and
  musl based Linux (Bun v1.1.35+).

## 1.0.3

#### 🚀 Updates

- Updated to support moon v2.5 release.

## 1.0.2

#### 🚀 Updates

- Updated with latest moon v2 plugin APIs.

## 1.0.1

#### 🚀 Updates

- Updated with moon v2 plugin APIs.

#### 🐞 Fixes

- Fixed some `package.json` version parsing issues.

## 1.0.0

#### 🚀 Updates

- Official major release for moon v2.

## 0.2.0

#### 🚀 Updates

- Updated to support moon v1.41.

## 0.1.1

#### 🚀 Updates

- Removed globals directory injection as this happens in moon directly.

## 0.1.0

#### 🚀 Updates

- Initial release!
