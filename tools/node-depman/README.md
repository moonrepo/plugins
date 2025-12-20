# Node.js package manager plugins

npm, pnpm, and yarn WASM plugins for [proto](https://github.com/moonrepo/proto).

## Installation

```shell
proto install npm|pnpm|yarn
```

This plugin is built-in to proto, but if you want to override it with an explicit version, add the following to `.prototools`.

```toml
[plugins]
npm|pnpm|yarn = "https://github.com/moonrepo/plugins/releases/download/node_depman_tool-vX.Y.Z/node_depman_tool.wasm"
```

## Configuration

All plugins can be configured with a `.prototools` file.

- `registry-url` (string) - The registry URL to download archives from.
- `dist-url` (string) - The distribution archives URL format. Supports `{registry}`, `{package}`, `{package_without_scope}`, `{version}`, and `{file}` tokens.
- `shared-globals-dir` (bool) - EXPERIMENTAL: Global npm, pnpm, or yarn packages are installed to a shared location: `~/.proto/tools/node/globals`. Defaults to `false`.

```toml
[tools.npm]
registry-url = "https://..."
dist-url = "{registry}/{package}/-/{package_without_scope}-{version}.tgz"
shared-globals-dir = true

# [tools.pnpm]
# [tools.yarn]
```

> To execute the shared globals, you'll need to add `~/.proto/tools/node/globals/bin` to `PATH` in your shell.

## Hooks

### Pre-run

Before a npm/pnpm/yarn command is ran and `shared-globals-dir` is enabled, this hook will modify the arguments or environment variables of the command when installing/removing/etc a global package. Is a no-op for other commands.

npm and yarn will set the `PREFIX` environment variable, while pnpm will set `--global-dir` and `--global-bin-dir` arguments.

## Contributing

Build the plugins:

```shell
cargo build --target wasm32-wasip1
```

Test the plugins by running `proto` commands.

```shell
proto install npm-test
proto versions npm-test
```
