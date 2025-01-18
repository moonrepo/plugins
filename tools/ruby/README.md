# Ruby plugin

Ruby WASM plugin for [proto](https://github.com/moonrepo/proto).

## Installation

```shell
proto install ruby
```

This plugin is built-in to proto, but if you want to override it with an explicit version, add the following to `.prototools`.

```toml
[plugins]
ruby = "https://github.com/moonrepo/plugins/releases/download/ruby_tool-vX.Y.Z/ruby_tool.wasm"
```

## Configuration

Ruby plugin does not support configuration.

## Hooks

Ruby plugin does not support hooks.

## Contributing

Build the plugins:

```shell
cargo build --target wasm32-wasip1
```

Test the plugins by running `proto` commands.

```shell
proto install ruby-test
proto versions ruby-test
```
