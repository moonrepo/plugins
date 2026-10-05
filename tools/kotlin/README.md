# Kotlin plugin

[Kotlin](https://kotlinlang.org/) compiler WASM plugin for [proto](https://github.com/moonrepo/proto).

## Installation

Register the plugin in `.prototools` using a published plugin version:

```toml
[plugins.tools]
kotlin = "https://github.com/moonrepo/plugins/releases/download/kotlin_tool-vX.Y.Z/kotlin_tool.wasm"
```

Install Java and Kotlin:

```shell
proto install java
proto install kotlin
```

This plugin installs the portable command-line compiler ZIP from JetBrains. It supports Linux, macOS, and Windows with a compatible Java runtime. Kotlin/Native and the experimental compiler native image are separate distributions and are not installed by this plugin.

Java is a separate dependency. The upstream launchers use `JAVA_HOME` when set, otherwise `java` from `PATH`. Configure Java through proto or provide your own compatible JDK. Activating a proto-managed Java version sets `JAVA_HOME` through the Java plugin.

Choose a JDK compatible with the selected Kotlin release; historical compilers may not run on current Java versions.

## Versions

Pin Kotlin through `.prototools`:

```toml
kotlin = "2.4.20"
```

Exact versions, partial versions, ranges, and `latest` are supported. The latest alias selects a stable release; preview releases such as `2.4.20-RC` can be requested explicitly. Versions have no vendor scope.

Historical releases are supported, including numeric fixes (`1.1.4-3`), lowercase previews (`1.4.0-rc`), and EAP builds (`1.3.70-eap-274`). Two-part release names use a zero patch component: request `1.1.0` for the original `1.1` release, or `1.2.0-M1` for `1.2-M1`. Where preview tags and compiler build versions differ, both are available (for example, `1.3.0-rc4` and `1.3.0-rc-190`). The pre-1.0 four-part build `0.11.91.1` is represented as `0.11.91+1`.

The plugin also detects versions from these files, in order:

- `.kotlin-version`: the first nonempty, noncomment version specification.
- `.sdkmanrc`: the `kotlin=<version>` entry. Other SDK entries are ignored.

SHA-256 verification is enabled for Kotlin 1.9.0-RC and newer. Older releases do not provide compiler checksum files.

The installed CLI compiler is independent of the Kotlin compiler version configured in Gradle or Maven. Installing this tool does not update project build files.

## Executables and environment

The plugin exposes `kotlin` and `kotlinc`, plus `kotlinr`, `kotlinc-jvm`, `kotlinc-js`, `kotlinc-wasm`, and `kapt` when present in the installed release.

The `kotlin` shim uses the `kotlinr` runner when available, falling back to the older `kotlin` launcher. Windows uses the distribution's `.bat` launchers. All commands use proto shims. Activation adds the distribution's `bin` directory to `PATH` and sets `KOTLIN_HOME` to the installation directory.

For early distributions that have no runner, `kotlin` invokes the compiler. If `kotlinc` is also absent, both commands use `kotlinc-jvm`.

## Configuration

- `dist-url` (string): download URL template supporting `{version}` (proto version), `{tag}` (upstream Git tag), and `{file}` (actual archive filename). Use `{tag}` when mirroring GitHub's layout, as historical tags do not always equal `v{version}`. A mirror should also serve adjacent `.sha256` files for releases that provide checksums.

```toml
[tools.kotlin]
dist-url = "https://github.com/JetBrains/kotlin/releases/download/{tag}/{file}"
```

## Hooks

Kotlin does not implement installation or run hooks.

## Contributing

Build and test the plugin from the repository root:

```shell
cargo build -p kotlin_tool --target wasm32-wasip1 --release
cargo nextest run -p kotlin_tool --no-default-features
```

For local CLI testing, register the built artifact:

```toml
[plugins.tools]
kotlin-test = "file://./target/wasm32-wasip1/release/kotlin_tool.wasm"
```

```shell
proto install kotlin-test
proto versions kotlin-test
```
