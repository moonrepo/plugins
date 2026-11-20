# Changelog

## Unreleased

#### 🚀 Updates

- Panics now report their message, instead of an `unreachable` WASM trap error.
- `pn` and `pnx`, which pnpm 11 and 12 ship, are now pnpm executables.
- `nubr`, which nub ships since v0.9.3, is now a Nub executable.
- `.yarnrc`, Yarn 1's config file, is now a Yarn config file.
- Deprecated the Yarn `plugins` setting, which never ran in moon v2. Yarn 4 and later bundle the
  official plugins. For Yarn 2 and 3, commit `.yarn/plugins` and `.yarnrc.yml`.
- Yarn's `installArgs` now documents that `yarn workspaces focus` doesn't get them.
- Includes `node_depman_tool`'s Yarn changes: `yarn.version` wins over `yarnPath`, Yarn 1–5 come
  ahead of `~/.yarn/bin` on `PATH`, and Yarn 6 no longer runs Yarn Switch.

#### 🐞 Fixes

- Fixed `initialize_toolchain` linking to a page that doesn't exist.
- Fixed the Nub config's docs link.

## 1.1.1

#### 🚀 Updates

- Updated to support moon v2.6 release.

## 1.1.0

#### 🚀 Updates

- Added unstable support for [Nub](https://nubjs.com/).

## 1.0.4

#### 🚀 Updates

- Updated to support moon v2.5 release.

## 1.0.3

#### 🚀 Updates

- Updated to always include the "shared globals" directory.

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
