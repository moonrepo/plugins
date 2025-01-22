# Changelog

## 0.13.0

#### 🚀 Updates

- Added build from source support.

## 0.12.3

#### 🚀 Updates

- Updated dependencies.

## 0.12.2

- Switched to `astral-sh/python-build-standalone` from `indygreg/python-build-standalone` for pre-built images.

## 0.12.1

#### 🐞 Fixes

- Fixed an issue where our bin linking strategy would point to an invalid executable path.

## 0.12.0

#### 💥 Breaking

- Removed `python<major>` and `pip<major>` executables. Use the new proto bins feature in v0.42 instead.

#### 🚀 Updates

- Added `~/.local/bin` as a globals lookup directory.
- Updated to support proto v0.42 release.

## 0.11.0

#### 🚀 Updates

- Updated to support proto v0.40 release.

## 0.10.5

#### 🚀 Updates

- Migrated to a new repository: https://github.com/moonrepo/tools

## 0.10.4

#### 🚀 Updates

- Updated to support proto v0.37 release.

## 0.10.3

#### 🚀 Updates

- Updated dependencies.

## 0.10.2

#### 🚀 Updates

- Will now create a pip shim that includes the major version, for example, `pip3`.

## 0.10.1

#### 🚀 Updates

- Updated to support proto v0.36 release.

## 0.10.0

#### 🚀 Updates

- Updated to support proto v0.35 release.

## 0.9.0

#### 🚀 Updates

- Will now create a secondary executable that includes the major version in the file name, for example, `python3`.
- Updated to support proto v0.32 release.

## 0.8.0

#### 💥 Breaking

- Removed `install_global`, use `pip install` instead.
- Removed `uninstall_global`, use `pip uninstall` instead.

#### 🚀 Updates

- Updated to support proto v0.31 release.
- Updated dependencies.

## 0.7.0

#### 🚀 Updates

- Updated to support proto v0.29 release.

## 0.6.0

#### 💥 Breaking

- Removed deprecated functions: `locate_bins`, `create_shims`

#### 🚀 Updates

- Added support for Python 3.12 pre-builts.
- Updated to support proto v0.28 release.
- Updated to extism-pdk v1.

## 0.5.0

#### 🚀 Updates

- Updated to support proto v0.26 release.
- Improved error messages when a pre-built is not available.

#### ⚙️ Internal

- Updated dependencies.

## 0.4.0

#### 🚀 Updates

- Updated to support proto v0.24 release.

#### ⚙️ Internal

- Updated dependencies.

## 0.3.0

#### 💥 Breaking

- Removed `--user` from global package installation via `proto install-global`. Packages are now installed into the tool directory for the current Python version: `.proto/tools/python/3.12.0/install/bin`.

#### ⚙️ Internal

- Updated dependencies.

## 0.2.0

#### 🚀 Updates

- Updated to support proto v0.22 release.

#### ⚙️ Internal

- Updated dependencies.

## 0.1.2

#### ⚙️ Internal

- Temporarily disabling `python-build` functionality.

## 0.1.1

#### 🐞 Fixes

- Potentially fixed a WASM memory issue.

## 0.1.0

#### 🚀 Updates

- Updated to support proto v0.20 release.

#### ⚙️ Internal

- Updated dependencies.

## 0.0.2

#### ⚙️ Internal

- Updated dependencies.

## 0.0.1

#### 🎉 Release

- Initial release!
