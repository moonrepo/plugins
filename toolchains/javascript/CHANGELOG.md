# Changelog

## Unreleased

#### 🚀 Updates

- Added Deno support.
  - Can customize `packageManager` with `deno`.
  - Will parse `deno.json` and `deno.jsonc` manifest files.
  - Will parse `deno.lock` lock files.
  - Will install dependencies with `deno install`.
- Added workspace member caching to reduce fs operations.
- Updated `install_dependencies` and `setup_environment` to take project toolchain configuration into account.
- Updated to support moon v1.41.

## 0.1.3

#### 🐞 Fixes

- Fixed `pnpm-lock.yaml` parsing issues.

## 0.1.2

#### 🐞 Fixes

- Fixed `package.json` dependency version parsing issues.

## 0.1.1

#### 🚀 Updates

- Removed globals directory injection as this happens in moon directly.

## 0.1.0

#### 🚀 Updates

- Initial release!
