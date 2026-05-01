# npm plugin

[npm](https://docs.npmjs.com) backend WASM plugin for [proto](https://github.com/moonrepo/proto), that will install CLIs from [npmjs.com](https://npmjs.com).

## Installation

This plugin is built-in to proto, but if you want to override it with an explicit version, add the following to `.prototools`.

```toml
[plugins.backends]
npm = "https://github.com/moonrepo/plugins/releases/download/npm_backend-vX.Y.Z/npm_backend.wasm"
```

## Configuration

npm plugin can be configured with a `.prototools` file.

### For backend

- `bun` - Use `bun` for installing and executing packages instead of `npm`/`node`.

```toml
"npm:<id>" = "1.2.3"

[backends.npm]
bun = true
```
