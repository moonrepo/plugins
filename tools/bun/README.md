# Bun plugin

[Bun](https://bun.sh/) WASM plugin for [proto](https://github.com/moonrepo/proto).

## Installation

This plugin is built-in to proto, but if you want to override it with an explicit version, add the following to `.prototools`.

```toml
[plugins]
bun = "https://github.com/moonrepo/plugins/releases/download/bun_tool-vX.Y.Z/bun_tool.wasm"
```

## Configuration

Bun plugin can be configured with a `.prototools` file.

- `dist-url` (string) - The distribution URL to download Bun archives from. Supports `{version}` and `{file}` tokens.

```toml
[tools.bun]
dist-url = "https://..."
```

## Hooks

Bun plugin does not support hooks.

## Caveats

Bun supports a `bunx` executable, which is a shortcut for `bun x` (note the space). It achieves this through some file name magic, as the `bunx` file is exactly the same as the `bun` file, jsut the exec functionality is toggled on/off based on the file name (derived from `args[0]`).

Supporting this in proto is tricky. For symlinked binaries, we can easily support this, as we symlink the `bun` binary to `~/.proto/bin/bunx`. However, for the shim `~/.proto/shims/bunx`, we can't do this, as we execute `bun` as a child process, as there's no `bunx` file to execute (Bun doesn't provide it), and the file name magic doesn't happen. So for the shim, we execute `bun x` under the hood instead.

This caveat only really matters in regards to the `--bun` flag, which works for bins, _but not_ for shims.

## Contributing

Build the plugin:

```shell
cargo build --target wasm32-wasip1
```

Test the plugin by running `proto` commands.

```shell
proto install bun-test
proto versions bun-test
```
