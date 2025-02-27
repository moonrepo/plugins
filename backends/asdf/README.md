# asdf plugin

[asdf](https://asdf-vm.com/) backend WASM plugin for [proto](https://github.com/moonrepo/proto).

## Unsupported

The `exec-path`, `post-*`, `pre-*`, and `help.*` asdf scripts are currently not supported by this plugin.

## Installation

This plugin is built-in to proto, but if you want to override it with an explicit version, add the following to `.prototools`.

```toml
[plugins]
asdf = "https://github.com/moonrepo/plugins/releases/download/asdf_backend-vX.Y.Z/asdf_backend.wasm"
```

## Configuration

asdf plugin can be configured with a `.prototools` file.

- `asdf-shortname` (string) - The name of the [asdf plugin](https://github.com/asdf-vm/asdf-plugins) if different than the configured ID.
- `asdf-repository` (string) - The Git repository URL in which to locate [scripts](https://asdf-vm.com/plugins/create.html#scripts-overview). If not defined, is extracted from the shortname plugin index.
- `exes` (string[]) - List of executable file names (relative from `bin`) to be linked as a shim/bin. If not defined, we'll automatically scan the `bin` directory.

```toml
<id> = "asdf:1.2.3"

[tools.<id>]
asdf-shortname = "..."
```

## Hooks

asdf plugin does not support hooks.

## Contributing

Build the plugin:

```shell
cargo build --target wasm32-wasip1
```

Test the plugin by running `proto` commands.

```shell
proto install asdf-test
proto versions asdf-test
```
