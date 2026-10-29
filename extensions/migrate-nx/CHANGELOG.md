# Changelog

## Unreleased

#### 🚀 Updates

- Added support for the latest Nx features:
  - `targetDefaults` array entries (unfiltered entries only).
  - `targetDefaults` keyed by executor (applied to project targets with that executor).
  - `targets.*.metadata.description`.
  - `targets.*.dependsOn[].waitFor` (maps to `wait` dependencies).
  - `json` and `workingDirectory` inputs.
  - The `...` spread token.
- Dependencies on continuous targets are now `wait` dependencies, and continuous targets enable `runInCI`, so that they're valid in moon.

#### 🐞 Fixes

- Fixed `dependsOn[].projects` being mapped to `^` when set to a project name, `{self}`, or `{dependencies}`.
- Fixed `dependsOn` wildcards (`build-*`) and `targetDefaults` globs failing or creating invalid tasks.
- Fixed `dependsOn` not cleaning project and target names (`@scope/name`, `build:prod`) like other identifiers.
- Fixed `cache: false` not disabling the cache.
- Fixed `fileset` inputs with `dependencies: true` being migrated as project inputs.

## 1.0.5

#### 🚀 Updates

- Updated to support moon v2.6 release.

## 1.0.4

#### 🚀 Updates

- Updated to support moon v2.5 release.

## 1.0.3

#### 🚀 Updates

- Updated with latest moon v2 plugin APIs.

## 1.0.2

#### 🚀 Updates

- Added support for `.config/moon` directory.

## 1.0.1

#### 🚀 Updates

- Updated with moon v2 plugin APIs.

## 1.0.0

#### 🚀 Updates

- Official major release for moon v2.
- Added support for the following `project.json` fields:
  - `targets.*.continuous`

## 0.0.11

#### ⚙️ Internal

- Enabled experimental trace logging.
- Updated dependencies.

## 0.0.10

#### ⚙️ Internal

- Updated dependencies.

## 0.0.9

#### 🚀 Updates

- Will no longer delete Nx files by default.
- Updated dependencies.

## 0.0.8

#### 🚀 Updates

- Switched to new toolchain system.
- Updated dependencies.

## 0.0.7

#### 🚀 Updates

- Added `register_extension` API.

## 0.0.6

#### 🚀 Updates

- Added support for `defaultBase` from `nx.json`.
- Added support for `metadata` from `project.json`.
- Updated dependencies.

## 0.0.5

#### ⚙️ Internal

- Re-publish failed release.

## 0.0.4

#### 🚀 Updates

- Updated dependencies.

## 0.0.3

#### 🐞 Fixes

- Fixed invalid IDs when converting `package.json` names.

## 0.0.2

#### 🚀 Updates

- Initial release!
