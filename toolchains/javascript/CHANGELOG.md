# Changelog

## Unreleased

#### 🐞 Fixes

- Fixed some issues around version parsing when parsing lockfiles.

## 1.2.0

#### 🚀 Updates

- Added support for [Deno v2.9](https://deno.com/blog/v2.9):
  - Will support other package manager lockfiles for seeding.

## 1.1.0

#### 🚀 Updates

- Added support for [Deno v2.8](https://deno.com/blog/v2.8):
  - Will use `deno ci` for installs in CI when `deno.lock` exists and the
    configured Deno version is >= v2.8.
  - Will pass `--prod` to `deno install` for production installs when the
    configured Deno version is >= v2.8.
  - Will resolve `catalog:` references in `package.json` files using
    catalogs declared in a root `deno.json`.

## 1.0.8

#### 🐞 Fixes

- Fixed `pnpm-lock.yaml` parsing for pnpm v10's multi-document lockfiles, which
  are written when `managePackageManagerVersions` is enabled (the default).

## 1.0.7

#### 🚀 Updates

- Will now remove `node_modules` during Docker prune, as some package managers don't do this automatically.
- Removed `*.config.*` files from the Docker scaffold process.

## 1.0.6

#### 🚀 Updates

- Updated `deno.lock` and `yarn.lock` parsing dependencies.

## 1.0.5

#### 🚀 Updates

- Added support for `*` versions when determining project relationships.

#### 🐞 Fixes

- Reworked `pnpm dedupe` and `yarn dedupe` detection logic.

## 1.0.4

#### 🚀 Updates

- Updated with latest moon v2 plugin APIs.

## 1.0.3

#### 🚀 Updates

- Reduced memory consumption.

#### 🐞 Fixes

- Fixed more `package.json` version parsing issues.

## 1.0.2

#### 🚀 Updates

- Updated with moon v2 plugin APIs.

#### 🐞 Fixes

- Fixed some `package.json` version parsing issues.

## 1.0.1

#### 🐞 Fixes

- Fixed some `bun.lock` parsing issues.

## 1.0.0

#### 🚀 Updates

- Official major release for moon v2.
- Added support for Yarn v4.10 catalogs.

#### 🐞 Fixes

- Fixed an issue where implicit dependencies would sometimes not resolve.

## 0.2.2

#### 🚀 Updates

- Added support for Bun v1.3 `package.json` catalogs.
- Updated `parse_manifest` to resolve versions from applicable catalogs.

## 0.2.1

#### 🐞 Fixes

- Fixed some version parsing issues that contain ".x" and other variants.

## 0.2.0

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
