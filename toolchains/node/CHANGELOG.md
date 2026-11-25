# Changelog

## Unreleased

#### 🚀 Updates

- Panics now report their message, instead of an `unreachable` WASM trap error.
- The default Docker image when `version` isn't set is now `node:lts` instead of `node:latest`,
  which follows Node.js's Current release.
- `.nvmrc` and `.node-version` are now config files, so `docker scaffold` copies them into the
  configs skeleton.
- `moon toolchain add node` no longer prompts for `syncVersionManagerConfig`, which does nothing.

#### 🐞 Fixes

- Fixed the default Docker image for a version bound being the oldest allowed version, like
  `node:20` for `>=20`, or one outside the range, like `node:22` for `<22`. It's now `node:latest`.
- Fixed the default Docker image not existing for the `stable`, `lts-<codename>` and
  `lts/<codename>` aliases. They now use `node:lts` and `node:<codename>`. `canary` and
  pre-releases, which have no images, now use `node:latest`.
- Fixed an entry in nodejs.org's index that can't be parsed failing version loading. It's now
  skipped.

## 1.0.5

#### 🚀 Updates

- Updated to support moon v2.6 release.

## 1.0.4

#### 🚀 Updates

- Deprecated the `syncVersionManagerConfig` setting (it never worked correctly).

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

## 0.2.0

#### 🚀 Updates

- Updated to support moon v1.41.

## 0.1.1

#### 🚀 Updates

- Removed globals directory injection as this happens in moon directly.

## 0.1.0

#### 🚀 Updates

- Initial release!
