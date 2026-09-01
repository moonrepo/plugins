# Node.js plugin

Node.js WASM plugin for [proto](https://github.com/moonrepo/proto).

## Installation

```shell
proto install node
```

This plugin is built-in to proto, but if you want to override it with an explicit version, add the following to `.prototools`.

```toml
[plugins.tools]
node = "https://github.com/moonrepo/plugins/releases/download/node_tool-vX.Y.Z/node_tool.wasm"
```

## Configuration

All plugins can be configured with a `.prototools` file.

- `bundled-npm` (bool) - When `node` is installed, also install `npm` with the version of npm that came bundled with Node.js. Defaults to `false`.
- `dist-url` (string) - The distribution URL to download Node.js archives from. Supports `{channel}`, `{version}`, and `{file}` tokens.
- `dist-url-unofficial` (string) - The distribution URL to download [unofficial Node.js builds](https://github.com/nodejs/unofficial-builds) from, like musl. Supports `{channel}`, `{version}`, and `{file}` tokens.
- `index-url` (string) - The URL of a Node.js-compatible versions index. Supports the `{channel}` token.

```toml
[tools.node]
bundled-npm = true
dist-url = "https://nodejs.org/download/{channel}/v{version}/{file}"
dist-url-unofficial = "https://unofficial-builds.nodejs.org/download/{channel}/v{version}/{file}"
index-url = "https://nodejs.org/download/{channel}/index.json"
```

> The `{channel}` token is replaced with `release`, or `nightly` when installing a canary version.

## Hooks

### Post-install

After Node.js is installed and `bundled-npm` is enabled, the version of npm that came bundled with Node.js will also be installed. This functionality can also be skipped by passing `--no-bundled-npm` during installation.

```shell
proto install node -- --no-bundled-npm
```

## Contributing

Build the plugins:

```shell
cargo build --target wasm32-wasip1
```

Test the plugins by running `proto` commands.

```shell
proto install node-test
proto versions node-test
```
