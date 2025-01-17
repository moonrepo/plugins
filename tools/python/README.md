# Python plugin (experimental)

[Python](https://www.python.org/) WASM plugin for [proto](https://github.com/moonrepo/proto).

## Installation

```shell
proto install python
```

This plugin is built-in to proto, but if you want to override it with an explicit version, add the following to `.prototools`.

```toml
[plugins]
python = "https://github.com/moonrepo/plugins/releases/download/python_tool-vX.Y.Z/python_tool.wasm"
```

## Configuration

Python plugin does not support configuration.

## Hooks

Python plugin does not support hooks.

## Caveats

This will install a pre-built version from [astral-sh/python-build-standalone](https://github.com/astral-sh/python-build-standalone), which doesn't support all versions, only Python 3.

Building from source directly (with `python-build`), and supporting Python 2, will be fully supported in the future.

## Contributing

Build the plugin:

```shell
cargo build --target wasm32-wasip1
```

Test the plugin by running `proto` commands.

```shell
proto install python-test
proto list-remote python-test
```
