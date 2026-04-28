# Changelog

## Unreleased

#### 🐞 Fixes

- Fixed an issue where the wrong arguments were passed to `uv sync` depending on whether proto is managing the Python version.
- Fixed an issue where venv paths were not available to commands ran through the toolchain, like `uv sync`.

## 0.1.7

#### 🐞 Fixes

- Fixed this toolchain depending on pip/uv, when it should be reversed.

## 0.1.6

#### 🐞 Fixes

- Fixed install/venv args being passed incorrectly in some situations.

## 0.1.5

#### 🐞 Fixes

- Fixed an issue where install commands didn't have access to venv bins.
- Fixed an issue where project dependencies were not being inferred correctly when the dependency contains extras metadata.

## 0.1.4

#### 🚀 Updates

- Normalized package/dependency names to PEP 503 during graph extending.

#### 🐞 Fixes

- Fixed an issue where package manager toolchain settings were not being inherited correctly.

## 0.1.3

#### 🚀 Updates

- Added `extend_project_graph` support. Will now read `pyproject.toml` dependencies to determine project relationships.

## 0.1.2

#### 🚀 Updates

- Updated with latest moon v2 plugin APIs.

## 0.1.1

#### 🚀 Updates

- Updated with moon v2 plugin APIs.

## 0.1.0

#### 🚀 Updates

- Initial release!
