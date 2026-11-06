# ZLS plugin

## Unreleased

#### 🐞 Fixes

- Removed the archive prefix from downloads, as ZLS archives have no top-level directory.
- Fixed partial versions and aliases in Zig version files, like `0.15` or `latest` in
  `.zig-version`, failing to parse and stopping ZLS from installing.

## 0.1.0

#### 🎉 Release

- Added an initial ZLS command-line plugin.
- Added stable releases from the official Zigtools release index.
- Added SHA-256 verification and official prebuilt archives for Linux, macOS, and Windows.
- Added compatible ZLS version detection from `.zig-version`, `.zigversion`, and `build.zig.zon` files.
