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
- `linux-platform` (enum) - The Linux distribution to download Swift for. Defaults to `ubuntu-24.04`. Supports `amazon-linux-2`, `amazon-linux-2023`, `debian-12`, `fedora-39`, `fedora-41`, `red-hat-ubi-9`, `ubuntu-20.04`, `ubuntu-22.04`, and `ubuntu-24.04`.

```toml
[tools.swift]
dist-url = "https://download.swift.org/{release}/{platform}/{folder}/{file}"
linux-platform = "amazon-linux-2"
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
