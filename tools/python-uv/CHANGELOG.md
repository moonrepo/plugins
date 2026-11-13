# Changelog

## Unreleased

#### 💥 Breaking

- Requires proto v0.64 or later.
- `~=X.Y` in `required-version` now allows any `X.*` from `X.Y`, as PEP 440 and uv define it. For
  example, `~=0.5` resolves to the latest 0.x, instead of the latest 0.5.x.

#### 🚀 Updates

- Updated to support proto v0.64 release.
- Versions are now loaded with proto's `load_tool_versions` host function, so another Git
  repository or index can be configured with `[tools.uv.versions-source]`.
- A `uv.toml` without a `required-version` now stops detection in its directory, as uv ignores
  `[tool.uv]` in a `pyproject.toml` next to it.
- Added Windows arm64 and x86, and Linux x86, arm, ppc64le, s390x, and riscv64 support.
- Panics now report their message, instead of an `unreachable` WASM trap error.
- Git tags that aren't versions are now skipped when loading versions, instead of failing.

#### 🐞 Fixes

- Fixed `uv self update` not being blocked, as `uv self upgrade` was blocked instead.
- Fixed exact versions in `required-version`, like `==0.5`, `0.5`, or `===0.5.0`, resolving to the
  latest 0.5 patch instead of 0.5.0, which uv then refused to run. Short versions are now
  zero-padded, as PEP 440 compares them, which also applies to `<=`.
- Fixed `!=` in `required-version` failing version detection. It's now converted to a proto `!=`
  requirement, for example, `>=2.0, !=2.1.0`.

#### ⚙️ Internal

- Version files are now read from their path, as the `content` input is deprecated.

## 0.3.4

#### 🚀 Updates

- Updated to support proto v0.60 release.

## 0.3.3

#### 🚀 Updates

- Updated to support proto v0.59 release.

## 0.3.2

#### 🚀 Updates

- Updated to support proto v0.55 release.

## 0.3.1

#### ⚙️ Internal

- Enabled experimental trace logging.
- Updated dependencies.

## 0.3.0

#### 🚀 Updates

- Added detection sources: `uv.toml`, `pyproject.toml`

## 0.2.2

#### ⚙️ Internal

- Updated dependencies.

## 0.2.1

#### 🚀 Updates

- Updated to support proto v0.47 release.

## 0.2.0

#### 🚀 Updates

- Updated to support proto v0.46 release.

## 0.1.0

#### 🎉 Release

- Initial release!
