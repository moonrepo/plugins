# Cargo plugin

[Cargo](https://doc.rust-lang.org/cargo/) backend WASM plugin for [proto](https://github.com/moonrepo/proto), that will install CLIs from [crates.io](https://crates.io/).

## Installation

This plugin is built-in to proto, but if you want to override it with an explicit version, add the following to `.prototools`.

```toml
[plugins.backends]
cargo = "https://github.com/moonrepo/plugins/releases/download/cargo_backend-vX.Y.Z/cargo_backend.wasm"
```

## Configuration

Cargo plugin can be configured with a `.prototools` file.

- `bin` (string) - The name of an explicit binary within the package to install.
- `features` (string[]) - List of Cargo features to enable for the package.
- `no-default-features` (bool) - Disable the `default` feature of the package.
- `registry` (string) - A custom registry to install the package from.

```toml
"cargo:<id>" = "1.2.3"

[tools.<id>]
features = ["std"]
```

### For backend

- `no-binstall` (bool) - Do not use [cargo-binstall](https://crates.io/crates/cargo-binstall) for installing packages, and instead build from source.
- `registry` (string) - A custom registry to install packages from.

```toml
"cargo:<id>" = "1.2.3"

[backends.cargo]
no-binstall = true
```
