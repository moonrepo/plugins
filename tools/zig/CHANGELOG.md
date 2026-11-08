# Zig plugin

## Unreleased

#### 💥 Breaking

- `minimum_zig_version` in `build.zig.zon` now resolves within its minor version (`~0.14.0`) instead
  of to any newer version (`>=0.14.0`), as Zig changes APIs between minor versions. A 0.14 project
  was installing the latest Zig, which fails to build it. setup-zig, anyzig, and zvm install the
  exact version, while this also allows newer patches.

#### 🚀 Updates

- Panics now report their message, instead of an `unreachable` WASM trap error.

#### 🐞 Fixes

- Fixed comments in `.zig-version` and `.zigversion`, like `0.15.2 # stable`, failing to parse.
  Everything after `#` is now ignored.
- Fixed the `master` alias (`proto install zig master`, or `master` in `.zig-version`) failing to
  resolve, as the master build wasn't in the version list.
- No longer creates a `~/.proto/bin/zig.exe` on Windows. It's a copy of the executable, which can't
  find Zig's `lib` directory and fails on every run. Use the `zig` shim instead.

#### ⚙️ Internal

- Version files are now read from their path, as the `content` input is deprecated.

## 0.1.0

#### 🎉 Release

- Added an initial Zig language plugin.
- Added stable and master releases from Zig's official download index.
- Added SHA-256 verification and official prebuilt archives for Linux, macOS, Windows, FreeBSD, NetBSD, and OpenBSD.
- Added version detection from `.zig-version`, `.zigversion`, and `build.zig.zon` files.
