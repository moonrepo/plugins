# Changelog

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
- Added support for the following `turbo.json` fields:
  - `tags`
  - `tasks.*.env` (wildcards and negation)

## 0.1.8

#### ⚙️ Internal

- Enabled experimental trace logging.
- Updated dependencies.

## 0.1.7

#### ⚙️ Internal

- Updated dependencies.

## 0.1.6

#### 🚀 Updates

- Will no longer delete Turborepo files by default.
- Updated dependencies.

## 0.1.5

#### 🚀 Updates

- Switched to new toolchain system.
- Switched to `preset` from `local`.
- Updated dependencies.

## 0.1.4

#### 🚀 Updates

- Added `register_extension` API.

## 0.1.3

#### 🚀 Updates

- Added support for `interactive` task option.
- Updated dependencies.

## 0.1.2

#### ⚙️ Internal

- Re-publish failed release.

## 0.1.1

#### 🚀 Updates

- Added support for Turborepo v2.
- Updated dependencies.

## 0.1.0

#### 🚀 Updates

- Removed the requirement of moon's project graph. Will now scan for `turbo.json`s instead.
- Cleaned up the migration code to be more readable and maintainable.

## 0.0.2

#### 🚀 Updates

- Updated to allow a missing or empty `pipeline` in `turbo.json`.

## 0.0.1

#### 🚀 Updates

- Initial release!
- New features from moon migration:
  - Bun support behind a new `--bun` flag.
  - Runs scripts through a package manager, instead of `moon node run-script`.
  - Root-level tasks will now create a root config, instead of warning.
  - Supports `globalDotEnv`, `dotEnv`, and `outputMode`.
