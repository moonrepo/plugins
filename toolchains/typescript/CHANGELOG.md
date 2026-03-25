# Changelog

## Unreleased

#### 🚀 Updates

- Added TypeScript v6 support.

## 1.1.0

#### 🚀 Updates

- Added a `pruneProjectReferences` setting that prunes non-moon managed project references when syncing.

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

## 0.3.0

#### 🚀 Updates

- Updated to support moon v1.41.

## 0.2.3

#### 🚀 Updates

- When `includeSharedTypes` and `syncProjectReferences` are both enabled, and the shared types folder contains a `tsconfig.json`, it will also be synced as a project reference.

## 0.2.2

#### 🚀 Updates

- Temporarily disabled `hash_task_contents` and exe detection.

## 0.2.1

#### ⚙️ Internal

- Enabled experimental trace logging.
- Updated dependencies.

## 0.2.0

#### 🚀 Updates

- Support new toolchain APIs.

#### ⚙️ Internal

- Updated dependencies.

## 0.1.4

#### ⚙️ Internal

- Updated dependencies.

## 0.1.3

#### 🐞 Fixes

- Updated tsconfig parsing to not fail if an `extends` file is missing in the chain.

## 0.1.2

#### 🐞 Fixes

- Fixed some issues when tsconfig paths don't end with `.json`.

## 0.1.1

#### 🚀 Updates

- Added `initialize_toolchain` support.

## 0.1.0

#### 🚀 Updates

- Initial release!
