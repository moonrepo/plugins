# ZLS plugin

[ZLS](https://zigtools.org/zls/) (Zig Language Server) WASM plugin for [proto](https://github.com/moonrepo/proto).

Supports official prebuilt ZLS archives for Linux, macOS, and Windows. Requires proto v0.61 or newer.

## Installation

Add the plugin to `.prototools`, then install ZLS.

```toml
[plugins.tools]
zls = "https://github.com/moonrepo/plugins/releases/download/zig_ls_tool-vX.Y.Z/zig_ls_tool.wasm"
```

```shell
proto install zls
```

ZLS and Zig should use the same minor release. For example:

```shell
proto install zig 0.16
proto install zls 0.16
```

## Configuration

ZLS plugin can be configured with a `.prototools` file.

- `index-url` (string) - The URL of a ZLS-compatible download index. Defaults to the official Zigtools release index.

```toml
[tools.zls]
index-url = "https://releases.zigtools.org/v1/zls/index.json"
```

## Version detection

The plugin detects Zig versions from `.zig-version`, `.zigversion`, and `build.zig.zon`. Stable versions in plain version files are converted to a matching ZLS minor range because Zig and ZLS patch versions may differ. The `minimum_zig_version` field in `build.zig.zon` is preserved as a minimum minor requirement, while development Zig versions resolve to proto's `canary` version.

## Contributing

Build the plugin:

```shell
cargo build --target wasm32-wasip1
```

Test the plugin by running `proto` commands.

```shell
proto install zls-test
proto versions zls-test
```
