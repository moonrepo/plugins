# Changelog

## 1.0.5

#### 🚀 Updates

- Updated `Cargo.lock` parsing dependencies.

## 1.0.4

#### 🚀 Updates

- Updated with latest moon v2 plugin APIs.

## 1.0.3

#### 🐞 Fixes

- API compatibility.

## 1.0.2

#### 🚀 Updates

- Updated with moon v2 plugin APIs.

## 1.0.1

#### 🐞 Fixes

- Fixed an issue where dependency self-cycles (using `path = "."`) would be included in graph extension.

## 1.0.0

#### 🚀 Updates

- Official major release for moon v2.

## 0.3.0

#### 🚀 Updates

- Updated to support moon v1.41.

## 0.2.4

#### 🚀 Updates

- Removed globals directory injection as this happens in moon directly.

## 0.2.3

#### 🚀 Updates

- Updated manifest parsing to extract `path` and `git` values.

#### 🐞 Fixes

- Fixed invalid versions when creatin the Docker image name.

## 0.2.2

#### 🐞 Fixes

- Fixed a "wasm `unreachable` instruction executed" error.

## 0.2.1

#### ⚙️ Internal

- Enabled experimental trace logging.
- Updated dependencies.

## 0.2.0

#### 🚀 Updates

- Cached the globals bin directory when extending task commands/scripts.
- Task hashing now includes the host OS, arch, and libc.

#### ⚙️ Internal

- Updated dependencies.

## 0.1.2

#### 🐞 Fixes

- Fixed `cargo-binstall` failing in CI when the binary already exists.

## 0.1.1

#### 🐞 Fixes

- Fixed Cargo/Rustup home directory detection.

## 0.1.0

#### 🚀 Updates

- Initial release!
