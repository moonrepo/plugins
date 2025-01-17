# Go plugin

[Go](https://go.dev/) WASM plugin for [proto](https://github.com/moonrepo/proto).

## Installation

```shell
proto install go
```

This plugin is built-in to proto, but if you want to override it with an explicit version, add the following to `.prototools`.

```toml
[plugins]
go = "https://github.com/moonrepo/plugins/releases/download/go_tool-vX.Y.Z/go_tool.wasm"
```

## Configuration

Go plugin can be configured with a `.prototools` file.

- `dist-url` (string) - The distribution URL to download Go archives from. Supports `{version}` and `{file}` tokens.
- `gobin` (bool) - When enabled, will inject a `GOBIN` environment variable into your shell. Defaults to `false`.

```toml
[tools.go]
dist-url = "https://..."
gobin = false
```

## Hooks

### Post-install

After installation, we'll inject a `GOBIN` environment variable into your shell, pointing to
`~/go/bin`, if it does not already exist. This variable will be used to locate Go binaries across
all installed versions. This functionality can be skipped by passing `--no-gobin` during
installation, or setting the `gobin` configuration to `false`.

```shell
proto install go -- --no-gobin
```

## Contributing

Build the plugin:

```shell
cargo build --target wasm32-wasip1
```

Test the plugin by running `proto` commands.

```shell
proto install go-test
proto list-remote go-test
```
