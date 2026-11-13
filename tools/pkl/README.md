# Pkl plugin

[Pkl](https://pkl-lang.org) WASM plugin for [proto](https://github.com/moonrepo/proto).

## Installation

```shell
proto install pkl
```

This plugin is built-in to proto, but if you want to override it with an explicit version, add the following to `.prototools`.

```toml
[plugins.tools]
pkl = "https://github.com/moonrepo/plugins/releases/download/pkl_tool-vX.Y.Z/pkl_tool.wasm"
```

## Configuration

Pkl plugin does not support configuration.

## Hooks

Pkl plugin does not support hooks.

## Contributing

Build the plugins:

```shell
cargo build --target wasm32-wasip1
```

Test the plugins by running `proto` commands.

```shell
proto install pkl-test
proto versions pkl-test
```
