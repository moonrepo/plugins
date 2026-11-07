# Zig plugin

## Unreleased

#### 🐞 Fixes

- Fixed comments in `.zig-version` and `.zigversion`, like `0.15.2 # stable`, failing to parse.
  Everything after `#` is now ignored.
- Fixed the `master` alias (`proto install zig master`, or `master` in `.zig-version`) failing to
  resolve, as the master build wasn't in the version list.
- No longer creates a `~/.proto/bin/zig.exe` on Windows. It's a copy of the executable, which can't
  find Zig's `lib` directory and fails on every run. Use the `zig` shim instead.

## 0.1.0

#### 🎉 Release

- Added an initial Zig language plugin.
- Added stable and master releases from Zig's official download index.
- Added SHA-256 verification and official prebuilt archives for Linux, macOS, Windows, FreeBSD, NetBSD, and OpenBSD.
- Added version detection from `.zig-version`, `.zigversion`, and `build.zig.zon` files.
