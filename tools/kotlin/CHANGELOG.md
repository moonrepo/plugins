# Kotlin plugin

## 0.2.0

#### 💥 Breaking

- Requires proto v0.64 or later.

#### 🚀 Updates

- Updated to support proto v0.64 release.
- Versions are now loaded with proto's `load_tool_versions` host function, so another Git
  repository or index can be configured with `[tools.kotlin.versions-source]`.
- Panics now report their message, instead of an `unreachable` WASM trap error.

#### ⚙️ Internal

- Version files are now read from their path, as the `content` input is deprecated.

## 0.1.0

#### 🚀 Updates

- Added a Kotlin language plugin using the portable compiler distribution on Linux, macOS, and Windows.
- Added version detection from `.kotlin-version` and `.sdkmanrc`.
- Support historical compiler releases, numeric fixes, and previews with their original download names.
- Added compiler and runner shims, and `KOTLIN_HOME` activation.
