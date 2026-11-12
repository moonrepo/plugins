# ZLS plugin

## Unreleased

#### 💥 Breaking

- `minimum_zig_version` in `build.zig.zon` now resolves to a ZLS within the same minor version
  (`~0.14`) instead of any newer version (`>=0.14`), matching the Zig plugin.

#### 🚀 Updates

- Updated to support proto v0.64 release.

#### 🐞 Fixes

- Fixed comments in `.zig-version` and `.zigversion`, like `0.15.2 # stable`, failing to parse.
  Everything after `#` is now ignored.
- Removed the archive prefix from downloads, as ZLS archives have no top-level directory.
- Fixed partial versions and aliases in Zig version files, like `0.15` or `latest` in
  `.zig-version`, failing to parse and stopping ZLS from installing.

#### ⚙️ Internal

- Version files are now read from their path, as the `content` input is deprecated.

## 0.1.0

#### 🎉 Release

- Added an initial ZLS command-line plugin.
- Added stable releases from the official Zigtools release index.
- Added SHA-256 verification and official prebuilt archives for Linux, macOS, and Windows.
- Added compatible ZLS version detection from `.zig-version`, `.zigversion`, and `build.zig.zon` files.
