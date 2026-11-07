# Node.js package manager plugins

npm, pnpm, yarn, and nub WASM plugins for [proto](https://github.com/moonrepo/proto).

## Installation

```shell
proto install npm|pnpm|yarn|nub
```

This plugin is built-in to proto, but if you want to override it with an explicit version, add the following to `.prototools`.

```toml
[plugins.tools]
npm|pnpm|yarn|nub = "https://github.com/moonrepo/plugins/releases/download/node_depman_tool-vX.Y.Z/node_depman_tool.wasm"
```

## Configuration

All plugins can be configured with a `.prototools` file.

- `registry-url` (string) - The registry URL to download archives from.
- `dist-url` (string) - The distribution archives URL format. Supports `{registry}`, `{package}`, `{package_without_scope}`, `{version}`, and `{file}` tokens.
- `shared-globals-dir` (bool) - EXPERIMENTAL: Global npm, pnpm, yarn, or nub packages are installed to a shared location: `~/.proto/tools/node/globals`. Defaults to `false`.

```toml
[tools.npm]
registry-url = "https://..."
dist-url = "{registry}/{package}/-/{package_without_scope}-{version}.tgz"
shared-globals-dir = true

# [tools.pnpm]
# [tools.yarn]
```

> To execute the shared globals, you'll need to add `~/.proto/tools/node/globals/bin` to `PATH` in your shell.

## Environment

When `shared-globals-dir` is enabled, the environment of npm/pnpm/yarn/nub commands (via `proto run`, shims, or `proto activate`) is modified so that global packages are installed into the shared location.

npm and yarn will set the `PREFIX` environment variable, pnpm v11+ will set `pnpm_config_global_dir` and `pnpm_config_global_bin_dir`, while pnpm v10 and below, and nub, will set `npm_config_global_dir` and `npm_config_global_bin_dir`.

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
