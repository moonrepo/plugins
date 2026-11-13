# Swift plugin

## Unreleased

#### 💥 Breaking

- Requires proto v0.64 or later.
- The `swift-tools-version` in `Package.swift` is now a minimum requirement (`>=5.9`) instead of a
  caret range (`^5.9`), as it declares the oldest toolchain that can build the package, and newer
  toolchains build older manifests. Pin a version in `.swift-version` to stay on a major version.
- When `linux-platform` isn't configured, the Linux distribution is now detected from
  `/etc/os-release`, like swiftly, instead of always using `ubuntu-24.04`. Other distributions were
  installing an Ubuntu toolchain that can fail with missing or mismatched libraries. Unsupported
  distributions still fall back to `ubuntu-24.04`, with a warning.

#### 🚀 Updates

- Updated to support proto v0.64 release.
- Versions are now loaded with proto's `load_tool_versions` host function, so another Git
  repository or index can be configured with `[tools.swift.versions-source]`.
- The Linux distribution is now read from the host details that proto provides, instead of
  running `cat /etc/os-release`. Locking another platform uses `ubuntu-24.04`, unless
  `linux-platform` is configured.
- Added `debian-13`, `redhat-ubi-10`, and `ubuntu-26.04` Linux platforms, available from Swift 6.4.

#### 🐞 Fixes

- Fixed a lockfile shared by machines with different Linux distributions failing checksum
  verification, as each downloads a different toolchain. Distributions other than `ubuntu-24.04`
  are now recorded as a variant in the lockfile (for example, `debian-12`).
- Fixed comments in `.swift-version`, like `6.1.2 # stable`, failing to parse. Everything after
  `#` is now ignored.
- Fixed downloads of 6.4.0 and later `.0` releases, which Swift.org names `swift-6.4.0-RELEASE`
  instead of `swift-6.4-RELEASE`.
- Fixed the globals directory, which pointed at the toolchain's own `usr/bin` instead of
  `~/.swiftpm/bin`, where `swift package experimental-install` writes executables.
- Fixed `Package.swift` version detection, which failed on trailing settings like
  `// swift-tools-version: 6.3;(experimentalCGen)`, and ignored the spacing and casing that SwiftPM
  allows, like `//swift-tools-version:5.10`.

#### ⚙️ Internal

- Version files are now read from their path, as the `content` input is deprecated.

## 0.1.0

#### 🚀 Updates

- Added an initial Swift language plugin with Linux prebuilt downloads.
- Added configurable Linux distributions for every platform listed by Swift.org.
- Added GPG verification metadata for Linux prebuilt downloads.
- Added macOS universal toolchain downloads using Swift.org `.pkg` installers.
