# Changelog

## 1.1.2

#### 🐞 Fixes

- Fixed an issue where `go list` was not running in the project root.
- Fixed an issue where `go list` would add a project dependency to itself.

## 1.1.1

#### 🚀 Updates

- Added `inferRelationships` and `inferRelationshipsFromTests` settings to control `go list --deps` usage.

## 1.1.0

#### 🚀 Updates

- Will now run `go list --deps` to determine project relationships while extending the project graph.

## 1.0.3

#### 🚀 Updates

- Updated with latest moon v2 plugin APIs.

## 1.0.2

#### 🐞 Fixes

- API compatibility.

## 1.0.1

#### 🚀 Updates

- Updated with moon v2 plugin APIs.

#### 🐞 Fixes

- Fixed indirect `go.mod` dependencies being considered a project dependency.

## 1.0.0

#### 🚀 Updates

- Official major release for moon v2.

## 0.2.0

#### 🚀 Updates

- Updated to support moon v1.41.

## 0.1.6

#### 🐞 Fixes

- Fixed `go.mod` parsing failures when `tool` is a list.

## 0.1.5

#### 🚀 Updates

- Removed globals directory injection as this happens in moon directly.

## 0.1.4

#### 🐞 Fixes

- Fixed `bins` failing to install multiple in parallel.

## 0.1.3

#### 🐞 Fixes

- Fixed a "wasm `unreachable` instruction executed" error.

## 0.1.2

#### ⚙️ Internal

- Enabled experimental trace logging.
- Updated dependencies.

## 0.1.1

#### 🐞 Fixes

- Fixed `go.*` parsing failures when there was no trailing newline.

## 0.1.0

#### 🚀 Updates

- Initial release!
