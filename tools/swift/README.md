# Swift plugin

[Swift](https://www.swift.org/) WASM plugin for [proto](https://github.com/moonrepo/proto).

Supports Swift.org prebuilt toolchains for Linux and macOS. Requires proto v0.61 or newer.

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
- `linux-platform` (enum) - The Linux distribution to download Swift for. When not configured, it's detected from `/etc/os-release` (RHEL derivatives like Rocky and Alma use the `redhat-ubi-*` builds), falling back to `ubuntu-24.04` for distributions that Swift doesn't publish builds for. Supports `amazon-linux-2`, `amazon-linux-2023`, `debian-12`, `debian-13`, `fedora-39`, `fedora-41`, `redhat-ubi-9`, `redhat-ubi-10`, `ubuntu-20.04`, `ubuntu-22.04`, `ubuntu-24.04`, and `ubuntu-26.04`. Not every Swift version is published for every platform.

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
