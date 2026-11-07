# Python plugin

[Python](https://www.python.org/) WASM plugin for [proto](https://github.com/moonrepo/proto).

## Installation

```shell
proto install python
```

This plugin is built-in to proto, but if you want to override it with an explicit version, add the following to `.prototools`.

```toml
[plugins.tools]
python = "https://github.com/moonrepo/plugins/releases/download/python_tool-vX.Y.Z/python_tool.wasm"
```

## Configuration

Python plugin can be configured with a `.prototools` file.

- `use-latest-build` (bool) - When a fully-qualified version without a build is requested, like
  `3.12.0`, resolve it to the latest available pre-built build, like `3.12.0+20231002`. Defaults to
  `false`.

```toml
[tools.python]
use-latest-build = true
```

A `.python-version` file is read like pyenv and uv read it: the first line that isn't blank or a
comment. Files for other implementations, like `pypy3.10-7.3.12`, or `system`, are ignored.

## Hooks

Python plugin does not support hooks.

## Caveats

This will install a pre-built version from [astral-sh/python-build-standalone](https://github.com/astral-sh/python-build-standalone), which doesn't support all versions, only Python 3.

## Contributing

Build the plugin:

```shell
cargo build --target wasm32-wasip1
```

Test the plugin by running `proto` commands.

```shell
proto install python-test
proto versions python-test
```
