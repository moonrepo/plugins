# Changelog

## Unreleased

#### 🐞 Fixes

- Fixed the shared globals dir not always being available for lookup.

## 0.17.6

#### 🚀 Updates

- Replaced `pre_run` hook with `activate_environment` function.
- Updated `shared-globals-dir` to also work with `proto activate` and not just `proto run`.
- Now sets `pnpm_config_global_dir` and `pnpm_config_global_bin_dir` environment variables.

## 0.17.5

#### 🚀 Updates

- Respect `.editorconfig` when writing `package.json` files.

## 0.17.4

#### 🐞 Fixes

- Fixed an issue where `package.json` properties would be re-ordered while saving.

## 0.17.3

#### 🚀 Updates

- Updated to support proto v0.55 release.
- Added a `registry-url` config setting.
- Added `pin_version` and `unpin_version` support, which maps to `package.json` `devEngines.packageManager`.

#### 🐞 Fixes

- Fixed some `package.json` version parsing issues.

## 0.17.2

#### 🐞 Fixes

- Fixed an issue where major versions only (`18`) would not parse correctly.

## 0.17.1

#### 🚀 Updates

- Added `package.json` `devEngines.packageManager` support for version detection.

## 0.17.0

#### 🚀 Updates

- Added an internal shims feature that resolves issues when npm/pnpm/yarn binaries are ran in isolation (from the install directory).
  - These files are in the `shims` directory, relative from the install directory, and replace the `bin` directory.
  - Before this would fail with "Could not determine Node.js install directory" errors because it was unable to determine the correct file paths.

## 0.16.5

#### 🚀 Updates

- Updated to support proto v0.53 release.

## 0.16.4

#### 🚀 Updates

- Added a globals lookup directory for `~/.proto/tools/node/<version>/bin`. However, the node version may not always be available.

#### 🐞 Fixes

- Fixed some `package.json` parsing issues.

## 0.16.3

#### 🚀 Updates

- Removed the tool directory from `exes_dirs`.

## 0.16.2

#### 🚀 Updates

- Updated to support proto v0.52 release.

## 0.16.1

#### ⚙️ Internal

- Enabled experimental trace logging.
- Updated dependencies.

## 0.16.0

#### 🚀 Updates

- Improved `package.json` parsing.

## 0.15.2

#### ⚙️ Internal

- Updated dependencies.

## 0.15.1

#### 🚀 Updates

- Updated to support proto v0.47 release.

## 0.15.0

#### 🚀 Updates

- Updated to support proto v0.46 release.

## 0.14.2

#### 🚀 Updates

- Updated dependencies.

## 0.14.1

#### 🚀 Updates

- Added `node` as a required plugin for this plugin to function correctly.

## 0.14.0

#### 🚀 Updates

- Added support for `volta.extends`: https://docs.volta.sh/advanced/workspaces
- Updated `volta` to take precedence over `engines` in `package.json`.
- Updated to support proto v0.42 release.

## 0.13.1

#### 🚀 Updates

- Updated shared globals injection to work for all npm commands and not just add/remove.

## 0.13.0

#### 🚀 Updates

- Updated to support proto v0.40 release.

## 0.12.0

#### 🚀 Updates

- Added a `dist-url` config setting, allowing the download host to be customized.

## 0.11.6

#### 🚀 Updates

- Migrated to a new repository: https://github.com/moonrepo/tools

#### 🐞 Fixes

- Fixed the shared globals directory not resolving correctly.

## 0.11.5

#### 🐞 Fixes

- Fixed version parsing for versions that contain `.x`.

## 0.11.4

#### 🚀 Updates

- Updated to support proto v0.37 release.

## 0.11.3

#### 🚀 Updates

- Updated dependencies.

## 0.11.2

#### 🐞 Fixes

- Fixed yarn "2.4.3" not resolving or downloading correctly (it was published to the wrong package).

## 0.11.1

#### 🚀 Updates

- Updated to support proto v0.36 release.

## 0.11.0

#### 🚀 Updates

- Updated to support proto v0.35 release.

## 0.10.3

#### 🐞 Fixes

- Fixed yarn "latest" alias pointing to the v1 latest, instead of v4 (berry) latest.

## 0.10.2

#### 🚀 Updates

- Added a `dist-url` config setting, allowing the download host to be customized.

#### 🐞 Fixes

- Fixed `.nvmrc` and `.node-version` parsing when they contain comments.

## 0.10.1

#### 🚀 Updates

- Updated to support proto v0.32 release.

## 0.10.0

#### 💥 Breaking

- Removed `install_global`, use `npm/pnpm/yarn` instead.
- Removed `uninstall_global`, use `npm/pnpm/yarn` instead.
- Removed the `intercept-globals` config setting.

#### 🚀 Updates

- Added a new `shared-globals-dir` setting, which injects args/env vars into npm/pnpm/yarn commands when they attemp to install global packages.
- Updated to support proto v0.31 release.
- Updated dependencies.

## 0.9.1

#### 🚀 Updates

- Added version detection support for `volta` in `package.json`.

## 0.9.0

#### 💥 Breaking

- Changed the `bundled-npm` and `intercept-globals` settings to be `false` by default (instead of `true`).

#### 🚀 Updates

- Updated to support proto v0.29 release.

## 0.8.0

#### 💥 Breaking

- Removed deprecated functions: `locate_bins`, `create_shims`

#### 🚀 Updates

- Updated to support proto v0.28 release.
- Updated to extism-pdk v1.

## 0.7.0

#### 💥 Breaking

- Will no longer symlink binaries (`~/.proto/bin`) for all package managers.
  - You'll need to rely on shims for proper functonality.
  - And you'll most likely need to delete any old bins manually.

#### 🚀 Updates

- Updated to support proto v0.26 release.

#### ⚙️ Internal

- Updated dependencies.

## 0.6.1

#### 🚀 Updates

- Added `lts` and `lts-latest` as supported remote aliases.

## 0.6.0

#### 🚀 Updates

- Added 2 new settings: `intercept-globals` and `bundled-npm`.
- Updated to support proto v0.24 release.

#### ⚙️ Internal

- Updated dependencies.

## 0.5.3

#### 🐞 Fixes

- Fixed an incorrect globals directory on Windows.

#### ⚙️ Internal

- Updated dependencies.
- Updated globals install to use a `--prefix` arg instead of `PREFIX` env var.

## 0.5.2

#### 🚀 Updates

- Updated to support proto v0.23 release.
- Will now ignore detecting versions from `node_modules` paths.

## 0.5.1

#### 🐞 Fixes

- Fixed Yarn >= v1.22.20 not unpacking correctly.

## 0.5.0

#### 💥 Breaking

- Updated the `npm` tool to create the `npx` shim instead of the `node` tool.
- Updated symlinked binaries to use the shell scripts instead of the source `.js` files (when applicable).

#### 🚀 Updates

- Updated to support proto v0.22 release.

#### ⚙️ Internal

- Updated dependencies.

## 0.4.3

#### 🐞 Fixes

- Temporarily fixed an issue where Yarn would fail to parse the npm registry response and error with "control character (\u0000-\u001F) found while parsing a string".

## 0.4.2

#### 🚀 Updates

- Support Yarn v4.

#### 🐞 Fixes

- Temporarily fixed an issue where calling `node` as a child process may fail.

## 0.4.1

#### 🐞 Fixes

- Potentially fixed a WASM memory issue.

## 0.4.0

#### 🚀 Updates

- Updated to support proto v0.20 release.

#### ⚙️ Internal

- Updated dependencies.

## 0.3.2

#### 🐞 Fixes

- Now strips the corepack hash from `packageManager` when parsing versions.

## 0.3.1

#### ⚙️ Internal

- Updated dependencies.

## 0.3.0

#### 🚀 Updates

- Added support for installing the canary release (when applicable).
- Brought back support for detecting a version from `package.json` engines.
- Updated to support proto v0.17 release.

## 0.2.1

#### 🚀 Updates

- Updated to support proto v0.16 release.

## 0.2.0

#### 🚀 Updates

- Added support for `install_global` and `uninstall_global`.
- Added `post_install` hook for installing the bundled npm.
- Updated to support proto v0.15 release.

#### 🐞 Fixes

- **npm**
  - Will no longer crash when parsing an invalid `package.json`.

## 0.1.0

#### 💥 Breaking

- Will no longer check `engines` in `package.json` when detecting a version.

#### 🚀 Updates

- Updated to support proto v0.14 release.

## 0.0.2

#### 🐞 Fixes

- **npm**
  - Improved version resolution for "bundled" alias.

## 0.0.1

#### 🎉 Release

- Initial release!
