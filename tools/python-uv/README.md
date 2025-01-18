# Python uv plugin

uv WASM plugin for [proto](https://github.com/moonrepo/proto).

## Installation

```shell
proto install uv
```

This plugin is built-in to proto, but if you want to override it with an explicit version, add the following to `.prototools`.

```toml
[plugins]
ub = "https://github.com/moonrepo/plugins/releases/download/python_uv_tool-vX.Y.Z/python_uv_tool.wasm"
```

## Configuration

uv plugin does not support configuration.

## Hooks

uv plugin does not support hooks.

## Contributing

Build the plugins:

```shell
cargo build --target wasm32-wasip1
```

Test the plugins by running `proto` commands.

```shell
proto install uv-test
proto versions uv-test
```
