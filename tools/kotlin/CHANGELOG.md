# Kotlin plugin

## Unreleased

#### 🚀 Updates

- Updated to support proto v0.64 release.

#### ⚙️ Internal

- Version files are now read from their path, as the `content` input is deprecated.

## 0.1.0

#### 🚀 Updates

- Added a Kotlin language plugin using the portable compiler distribution on Linux, macOS, and Windows.
- Added version detection from `.kotlin-version` and `.sdkmanrc`.
- Support historical compiler releases, numeric fixes, and previews with their original download names.
- Added compiler and runner shims, and `KOTLIN_HOME` activation.
