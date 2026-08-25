# Zig plugin

[Zig](https://ziglang.org/) WASM plugin for [proto](https://github.com/moonrepo/proto).

Supports the official prebuilt Zig archives for Linux, macOS, Windows, FreeBSD, NetBSD, and OpenBSD. Requires proto v0.61 or newer.

## Installation

```shell
proto install zig
```

This plugin is built-in to proto, but if you want to override it with an explicit version, add the following to `.prototools`.

```toml
[plugins.tools]
zig = "https://github.com/moonrepo/plugins/releases/download/zig_tool-vX.Y.Z/zig_tool.wasm"
```

Install the latest development build with proto's `canary` version, or the Zig-native `master` alias.

```shell
proto install zig canary
proto install zig master
```

## Configuration

Zig plugin can be configured with a `.prototools` file.

- `index-url` (string) - The URL of a Zig-compatible download index. Defaults to Zig's official index.

```toml
[tools.zig]
index-url = "https://ziglang.org/download/index.json"
```

## Version detection

The plugin detects exact versions from `.zig-version` and `.zigversion`. It also detects `minimum_zig_version` from `build.zig.zon` as a minimum version requirement.

## Contributing

Build the plugin:

```shell
cargo build --target wasm32-wasip1
```

Test the plugin by running `proto` commands.

```shell
proto install zig-test
proto versions zig-test
```
