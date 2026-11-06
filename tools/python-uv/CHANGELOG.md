# Changelog

## Unreleased

#### 🚀 Updates

- Added Windows arm64 and x86, and Linux x86, arm, ppc64le, s390x, and riscv64 support.

#### 🐞 Fixes

- Fixed `uv self update` not being blocked, as `uv self upgrade` was blocked instead.
- Fixed `required-version = "===X.Y.Z"` resolving to the latest `X.Y` patch, instead of exactly
  `X.Y.Z`, which uv then refused to run.

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
