# Changelog

## 0.2.0

#### 💥 Breaking

- Requires proto v0.64 or later.

#### 🚀 Updates

- Updated to support proto v0.64 release.
- Versions are now loaded with proto's `load_tool_versions` host function, so another Git
  repository or index can be configured with `[tools.poetry.versions-source]`.
- Panics now report their message, instead of an `unreachable` WASM trap error.
- Git tags that aren't versions are now skipped when loading versions, instead of failing.
- A `requires-poetry` in `pyproject.toml` that can't be parsed is now logged at the debug level,
  instead of with a warning.

#### 🐞 Fixes

- Fixed the post-install hook writing `virtualenvs.use-poetry-python` to the global Poetry config
  for Poetry 2, overriding the user's own choice, when it's already the default. Only Poetry
  1.2-1.8 is still configured.
- Fixed the installer script being reused until Poetry is uninstalled. It's now fetched for every
  install.
- Fixed a `requires-poetry` that can't be translated, like `~=2`, failing version detection. It's now
  ignored with a warning.
- Fixed comments in `.poetry-version`, like `2.1.1 # stable`, failing to parse. Everything
  after `#` is now ignored.
- Fixed installing prereleases, like `1.2.0-rc2`, which the installer rejected as it only accepts
  PyPI's `1.2.0rc2` form.
- Fixed the post-install hook running whichever `poetry` was on `PATH` instead of the version that
  was just installed. Without proto's bins on `PATH` (a fresh CI machine, for example) the install
  failed, and through a shim it configured another version, or nothing when no version was pinned.
- Fixed `~=X.Y` in `requires-poetry` only allowing `X.Y.*`. Per PEP 440 it allows `X.*`, so `~=2.1`
  now resolves to the latest 2.x.
- Fixed `==X.Y` in `requires-poetry` resolving to the latest `X.Y` patch, instead of `X.Y.0`. Short
  versions are now zero-padded, as PEP 440 compares them.
- Fixed `!=` in `requires-poetry` failing version detection. It's now converted to a proto `!=`
  requirement, for example, `>=2.0, !=2.1.0`.
- Install failures now include the installer's output, instead of an empty error.
- Fixed a failed `poetry config` in the post-install hook being ignored. It now logs a warning.

#### ⚙️ Internal

- The installer script is now downloaded to a temporary directory for each install, which proto
  removes afterwards.
- Version files are now read from their path, as the `content` input is deprecated.

## 0.1.9

#### 🚀 Updates

- Updated to support proto v0.60 release.

## 0.1.8

#### 🚀 Updates

- Updated to support proto v0.59 release.

## 0.1.7

#### 🚀 Updates

- Will now detect a Poetry version from `tool.poetry.requires-poetry` in `pyproject.toml`.

## 0.1.6

#### 🚀 Updates

- Updated to support proto v0.55 release.

## 0.1.5

#### 🚀 Updates

- Updated to support proto v0.52 release.

## 0.1.4

#### ⚙️ Internal

- Enabled experimental trace logging.
- Updated dependencies.

## 0.1.3

#### ⚙️ Internal

- Updated dependencies.

## 0.1.2

#### 🚀 Updates

- Updated to support proto v0.47 release.

## 0.1.1

#### 🐞 Fixes

- Fixed an install error on Linux.

## 0.1.0

#### 🎉 Release

- Initial release!
