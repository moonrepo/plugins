# moon plugin

moon WASM plugin for [proto](https://github.com/moonrepo/proto).

## Installation

```shell
proto install moon
```

This plugin is built-in to proto, but if you want to override it with an explicit version, add the following to `.prototools`.

```toml
[plugins]
moon = "https://github.com/moonrepo/plugins/releases/download/moon_tool-vX.Y.Z/moon_tool.wasm"
```

## Configuration

moon plugin does not support configuration.

## Hooks

moon plugin does not support hooks.

## Contributing

Build the plugins:

```shell
cargo build --target wasm32-wasip1
```

Test the plugins by running `proto` commands.

```shell
proto install moon-test
proto list-remote moon-test
```
