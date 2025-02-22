# Python Poetry plugin

Poetry WASM plugin for [proto](https://github.com/moonrepo/proto).

## Installation

```shell
proto install poetry
```

This plugin is built-in to proto, but if you want to override it with an explicit version, add the following to `.prototools`.

```toml
[plugins]
poetry = "https://github.com/moonrepo/plugins/releases/download/python_poetry_tool-vX.Y.Z/python_poetry_tool.wasm"
```

## Configuration

Poetry plugin does not support configuration.

## Hooks

Poetry plugin does not support hooks.

## Contributing

Build the plugins:

```shell
cargo build --target wasm32-wasip1
```

Test the plugins by running `proto` commands.

```shell
proto install poetry-test
proto versions poetry-test
```
