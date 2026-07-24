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

## Vendors

Java is distributed by many vendors, with prebuilts provided by the [foojay Disco API](https://github.com/foojayio/discoapi). A specific vendor can be chosen by prefixing the version with the vendor's scope prefix, in the format of `<vendor>-<version>`.

```toml
java = "temurin-21"
```

Versions without a prefix resolve against the default `openjdk` distribution. Vendor prefixes are also supported in `.java-version` files, while `.sdkmanrc` files use SDKMAN's own `<version>-<vendor>` format, whose vendor codes map to the aliases below.

| Vendor                             | Scope prefix        | Aliases                     |
| ---------------------------------- | ------------------- | --------------------------- |
| AdoptOpenJDK                       | `aoj`               |                             |
| AdoptOpenJDK OpenJ9                | `aoj-openj9`        |                             |
| Huawei BiSheng                     | `bisheng`           |                             |
| Amazon Corretto                    | `corretto`          | `amzn`, `amazon`            |
| Debian OpenJDK                     | `debian`            |                             |
| Alibaba Dragonwell                 | `dragonwell`        | `albba`, `alibaba`, `dragon` |
| Gluon GraalVM                      | `gluon-graalvm`     | `gluon`                     |
| Oracle GraalVM                     | `graalvm`           | `graal`                     |
| GraalVM Community Edition (JDK 8)  | `graalvm-ce8`       |                             |
| GraalVM Community Edition (JDK 11) | `graalvm-ce11`      |                             |
| GraalVM Community Edition (JDK 16) | `graalvm-ce16`      |                             |
| GraalVM Community Edition (JDK 17) | `graalvm-ce17`      |                             |
| GraalVM Community Edition (JDK 19) | `graalvm-ce19`      |                             |
| GraalVM Community Edition (JDK 20) | `graalvm-ce20`      |                             |
| GraalVM Community                  | `graalvm-community` | `graalce`                   |
| JetBrains Runtime                  | `jetbrains`         |                             |
| Tencent Kona                       | `kona`              | `tencent`                   |
| BellSoft Liberica                  | `liberica`          | `librca`                    |
| BellSoft Liberica Native Image Kit | `liberica-native`   | `nik`                       |
| Red Hat Mandrel                    | `mandrel`           |                             |
| Microsoft Build of OpenJDK         | `microsoft`         | `ms`, `msoft`               |
| OJDKBuild                          | `ojdk-build`        |                             |
| OpenLogic OpenJDK                  | `open-logic`        |                             |
| Oracle OpenJDK (default)           | `openjdk`           | `open`                      |
| Oracle JDK                         | `oracle`            |                             |
| Red Hat Build of OpenJDK           | `redhat`            |                             |
| SapMachine                         | `sap-machine`       | `sap`, `sapmchn`            |
| IBM Semeru                         | `semeru`            | `sem`                       |
| IBM Semeru Certified Edition       | `semeru-certified`  |                             |
| Eclipse Temurin                    | `temurin`           | `tem`                       |
| Trava OpenJDK                      | `trava`             |                             |
| Azul Zulu                          | `zulu`              |                             |
| Azul Zulu Prime                    | `zulu-prime`        |                             |

> Aliases exist primarily for SDKMAN compatibility and are recognized when parsing `.sdkmanrc` files. Use the scope prefix when pinning versions in `.prototools` or `.java-version` files.

## Configuration

Java plugin can be configured with a `.prototools` file.

- `api-url` (string) - The Foojay Disco API URL to load Java prebuilts from.
- `release-type` (string) - The Java release type to load. Defaults to `ga`.

```toml
[tools.java]
api-url = "https://api.foojay.io/disco/v3.0"
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
