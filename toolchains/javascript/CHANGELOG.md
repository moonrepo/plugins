# Changelog

## Unreleased

#### 🚀 Updates

- Panics now report their message, instead of an `unreachable` WASM trap error.
- Lockfile parse errors now name the lockfile.
- Workspace dependencies declared with a version range, like `"b": "^1.0.0"`, are now project
  dependencies for npm, Yarn and Bun when the workspace package's version satisfies the range, as
  those package managers link them. These new edges can expose dependency cycles.
- `workspace:` ranges like `workspace:^1` and `workspace:>=1.0.0` are now supported.
- `syncProjectWorkspaceDependencies` now replaces a synced `version`, `version-caret`,
  `version-tilde` or peer range once the workspace package's version no longer satisfies it.
- `syncProjectWorkspaceDependencies` now syncs packages without a `version` for the formats that
  don't need one, and syncs their peers as `*`.
- `rootPackageDependenciesOnly` now allows local workspace dependencies in projects, like the ones
  that `syncProjectWorkspaceDependencies` writes.
- `docker scaffold` now copies the `package.json` of workspace members that aren't moon projects,
  and of local `file:`, `link:` and `portal:` dependencies (or the tarball), into the configs
  skeleton.
- `docker prune` now also deletes the `node_modules` of the focused projects and their dependencies,
  as moon v1 did, when `deleteVendorDirectories` is enabled.
- Inferred npm tasks now end with `--`, so passthrough arguments reach the script instead of npm.
  This changes their task hashes once.
- `npm:` and `jsr:` alias dependencies are now hashed by their range, and dist tags like `latest`
  by the first locked version.
- Package manager detection now also reads `devEngines.packageManager`.
- Version-gated behavior now treats a `latest` or `stable` version as the newest release, and a
  prerelease, like `6.0.0-rc.1`, like its release.

#### 🐞 Fixes

- Fixed `package.json` fields that npm accepts but the toolchain couldn't parse failing every moon
  command, like `"version": "1.0"`, `"bundleDependencies": true` or an `engines` array. Those
  fields are now skipped.
- Fixed `sync_project` rewriting every dependency in a field it changed, which could turn a range
  like `>=1.0.0 <2.0.0` into `>=1.0.0, <2.0.0`, which npm rejects. Only the synced entries are now
  written.
- Fixed `sync_project` failing every task in a JavaScript project without a `package.json`.
- Fixed synced peer ranges for 0.x packages being `^0.0.0`. They're now `^<version>`.
- Fixed `rootPackageDependenciesOnly` failing on empty dependency fields.
- Fixed inferred tasks failing the project graph for a script that can't be a task ID, like a
  `"//"` comment, or that a shell couldn't split. They're now skipped, or split on whitespace.
- Fixed inferred tasks being disabled in CI when the script contained `-w` anywhere, like in
  `--max-warnings`, `--write` or `build-web.js`. Watch, help and version options are now matched
  as whole arguments, and `-w` is ignored for package managers, where it selects a workspace.
- Fixed scripts named like lifecycle hooks, such as `prettier`, `preview` and `postcss`, not being
  inferred. Only `pre<name>` and `post<name>` scripts with a `<name>` script are skipped, and
  `deno.json` tasks, which have no hooks, are no longer filtered by prefix.
- Fixed root scripts that run `moon` being inferred, which made moon run itself recursively.
- Fixed scripts whose names clean to the same task ID, like `build:js` and `build-js`, replacing
  each other. The first one is kept.
- Fixed projects that share a package name, like examples, being left out of the project graph
  except for one. The first one is still the target for dependencies.
- Fixed a leftover lockfile from another package manager overriding the configured one's in task
  hashes. It's now ignored when the configured package manager's own lockfile exists.
- Fixed a full plugin cache failing `locate_dependencies_root`.
- Fixed `initialize_toolchain` failing on an unknown `packageManager`, and linking to a page that
  doesn't exist.
- Fixed lockfile parse errors showing a virtual path instead of the real one.

- Fixed an empty workspace entry in `bun.lock` panicking while parsing the lockfile.
- Fixed a failed `bun bun.lockb` being parsed as an empty lockfile. It now returns an error.
- Fixed a version that can't be parsed in `bun.lock`, `deno.lock`, `package-lock.json`,
  `pnpm-lock.yaml`, `yarn.lock` or `package.json` failing every task hash. The version is now
  skipped.
- Fixed `bun.lock` entries with an unknown shape failing to parse the lockfile. They're now skipped.
- Fixed `package-lock.json` files with a `lockfileVersion` above 3 returning no dependencies.
  They're now parsed like version 3.

## 1.3.2

#### 🚀 Updates

- Updated to support moon v2.6 release.

## 1.3.1

#### 🚀 Updates

- Updated Bun support for the v1.4 release:
  - Dependencies are now installed with `bun ci` in CI when a `bun.lock` exists
    (v1.2.20+), as Bun does not enable frozen lockfiles in CI automatically.
  - Dependencies are now deduped with `bun dedupe` when the
    `dedupeOnLockfileChange` setting is enabled (v1.4+).
  - Focused installs now include workspace dependencies, by passing `...`
    dependency relations to `--filter` (v1.4+).

#### 🐞 Fixes

- Updated `package.json` version parsing to support custom protocols (`example:`).
- Fixed `bun.lock` parsing failing on nested overrides (lockfile v3).
- Fixed Git/GitHub `bun.lock` entries without an integrity hash recording the
  Bun tag as the hash.

## 1.3.0

#### 🚀 Updates

- Added unstable support for [Nub](https://nubjs.com/) as a package manager:
  - Natively uses `nub.lock` (pnpm lockfile format), but will locate dependency
    roots using other package manager lockfiles that nub can operate on.
  - Reads workspace members and catalogs from `pnpm-workspace.yaml` when
    present, otherwise from `package.json`.
  - Does not require the Node.js toolchain, as nub is a standalone binary.

#### 🐞 Fixes

- Fixed `bun.lock` parsing failing on Git/GitHub dependencies that include both
  package metadata (`dependencies`, `bin`, etc) and an integrity hash.

## 1.2.2

#### 🚀 Updates

- Updated to support moon v2.5 release.

## 1.2.1

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
