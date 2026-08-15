# Swift plugin

[Swift](https://www.swift.org/) WASM plugin for [proto](https://github.com/moonrepo/proto).

## Installation

```shell
proto install swift
```

This plugin is built-in to proto, but if you want to override it with an explicit version, add the following to `.prototools`.

```toml
[plugins.tools]
swift = "https://github.com/moonrepo/plugins/releases/download/swift_tool-vX.Y.Z/swift_tool.wasm"
```

## Configuration

Swift plugin can be configured with a `.prototools` file.

- `dist-url` (string) - The distribution URL to download Swift archives from. Supports `{release}`, `{platform}`, `{folder}`, and `{file}` tokens.
- `linux-platform` (string) - The platform directory in Swift.org download URLs. Defaults to `ubuntu2404`.
- `linux-archive-suffix` (string) - The platform suffix in Swift.org archive names. Defaults to `ubuntu24.04`.

```toml
[tools.swift]
dist-url = "https://download.swift.org/{release}/{platform}/{folder}/{file}"
linux-platform = "ubuntu2404"
linux-archive-suffix = "ubuntu24.04"
```

## Contributing

Build the plugin:

```shell
cargo build --target wasm32-wasip1
```

Test the plugin by running `proto` commands.

```shell
proto install swift-test
proto versions swift-test
```
