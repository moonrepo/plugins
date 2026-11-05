# Ruby plugin

Ruby WASM plugin for [proto](https://github.com/moonrepo/proto).

## Installation

```shell
proto install ruby
```

This plugin is built-in to proto, but if you want to override it with an explicit version, add the following to `.prototools`.

```toml
[plugins.tools]
ruby = "https://github.com/moonrepo/plugins/releases/download/ruby_tool-vX.Y.Z/ruby_tool.wasm"
```

## Configuration

Ruby plugin can be configured with a `.prototools` file.

- `use-latest-build` (bool) - When a fully-qualified version without a build is requested, like
  `3.4.5`, resolve it to the latest available pre-built build, like `3.4.5+4`. Defaults to `false`.

```toml
[tools.ruby]
use-latest-build = true
```

A `.ruby-version` file may contain the `ruby-` prefix used by RVM and chruby, like `ruby-3.4.5`.
Files for other engines, like `jruby-9.4.5.0`, are ignored.

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

Update the cached list of portable Ruby binaries after jdx publishes new releases:

```shell
node scripts/generateRubyReleases.mjs
```
