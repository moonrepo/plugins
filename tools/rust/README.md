# Rust plugin

[Rust](https://www.rust-lang.org/) WASM plugin for [proto](https://github.com/moonrepo/proto).

## Installation

```shell
proto install rust
```

This plugin is built-in to proto, but if you want to override it with an explicit version, add the following to `.prototools`.

```toml
[plugins]
rust = "https://github.com/moonrepo/plugins/releases/download/rust_tool-vX.Y.Z/rust_tool.wasm"
```

## Configuration

Rust plugin does not support configuration.

## Hooks

Rust plugin does not support hooks.

## Caveats

If you're familiar with Rust, you most likely use [rustup](https://rustup.rs), a Rust specific toolchain manager. This overlaps heavily with how proto works, so instead of proto reinventing the wheel here, we simply call `rustup` under the hood. Because of this, be aware of the following when using Rust in proto:

- The `~/.cargo/bin` directory must be in your `PATH`.
- We don't install Rust to `~/.proto/tools/rust` but instead reference `~/.rustup/toolchains`.
- We don't create shims for `cargo`, `rustup`, etc.

Since we don't create shims for `cargo`, `rustup`, etc, we can't detect Rust versions at runtime. However, `rustup` supports this through the
[`rust-toolchain.toml`](https://rust-lang.github.io/rustup/overrides.html#the-toolchain-file) file. We suggest using this file.

## Contributing

Build the plugin:

```shell
cargo build --target wasm32-wasip1
```

Test the plugin by running `proto` commands.

```shell
proto install rust-test
proto versions rust-test
```
