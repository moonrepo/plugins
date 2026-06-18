# Java plugin

[Java](https://www.java.com/en/) WASM plugin for [proto](https://github.com/moonrepo/proto).

## Installation

```shell
proto install java
```

This plugin is built-in to proto, but if you want to override it with an explicit version, add the following to `.prototools`.

```toml
[plugins.tools]
java = "https://github.com/moonrepo/plugins/releases/download/java_tool-vX.Y.Z/java_tool.wasm"
```

## Configuration

Java plugin can be configured with a `.prototools` file.

- `api-url` (string) - The Foojay Disco API URL to load Java prebuilts from.
- `vendor` (string) - The Java distribution vendor to install. Defaults to `temurin`.
- `image-type` (string) - The Java image type to install. Defaults to `jdk`.
- `release-type` (string) - The Java release type to load. Defaults to `ga`.

```toml
[tools.java]
api-url = "https://api.foojay.io/disco/v3.0"
vendor = "temurin"
image-type = "jdk"
release-type = "ga"
```

## Hooks

Java plugin does not support hooks.

## Contributing

Build the plugin:

```shell
cargo build --target wasm32-wasip1
```

Test the plugin by running `proto` commands.

```shell
proto install java-test
proto versions java-test
```
