# Changelog

## Unreleased

#### 🚀 Updates

- `installArgs` now documents that it also applies to `uv pip install`, when the Python toolchain's
  `packageManager` is `uv-pip`.
- Deprecated the `version` setting, which never did anything, as the venv uses the pip that ships
  with Python.
- Panics now report their message, instead of an `unreachable` WASM trap error.

## 0.1.5

#### 🚀 Updates

- Updated to support moon v2.6 release.

## 0.1.4

#### 🚀 Updates

- Ensures that Python is installed before setting up this toolchain.

## 0.1.3

#### 🚀 Updates

- Updated to support moon v2.5 release.

## 0.1.2

#### 🚀 Updates

- Updated with latest moon v2 plugin APIs.

## 0.1.1

#### 🚀 Updates

- Updated with moon v2 plugin APIs.

## 0.1.0

#### 🚀 Updates

- Initial release!
