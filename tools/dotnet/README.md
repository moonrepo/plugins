# .NET plugin

[.NET](https://dotnet.microsoft.com/) WASM plugin for [proto](https://github.com/moonrepo/proto).

Installs .NET SDKs from Microsoft's published archives. Requires proto v0.60 or newer.

## Installation

This plugin is not built-in to proto. Add it to `.prototools`.

```toml
[plugins.tools]
dotnet = "https://github.com/moonrepo/plugins/releases/download/dotnet_tool-vX.Y.Z/dotnet_tool.wasm"
```

```shell
proto install dotnet
```

An SDK archive is self-contained, so it unpacks into proto's version directory as-is: muxer, `host/fxr`, matching shared runtimes, the `sdk/<band>` directory, packs and templates. The `dotnet` executable is exposed as a shim only, with no `bin` symlink, because the muxer resolves `host/fxr` and `shared/` relative to its own location and fails when run through a symlink.

## Versions

Versions come from Microsoft's release metadata rather than git tags. Tags do not cover every published SDK (8.0.125, 8.0.201 and 9.0.101 have none) and some that do exist have no downloadable archive.

`lts`, `sts` and `preview` aliases follow .NET's release cadence.

### Feature bands

The hundreds digit of the patch version is the SDK's feature band. Bands are parallel product lines, not a sequence: 8.0.404 and 8.0.130 are both current, and neither is newer than the other.

A channel pin resolves to the highest band, which is not always the one you want.

```toml
dotnet = "8.0"     # highest band, 4xx today
dotnet = "8.0.404" # exactly this SDK
```

To follow a single band, pin the range.

```toml
dotnet = ">=8.0.400 && <8.0.500"
```

## Detection

`global.json` is parsed structurally rather than read as a bare version, so `sdk.rollForward` is honored alongside `sdk.version`. For a pinned `8.0.404`:

| `rollForward`                 | Requirement                |
| ----------------------------- | -------------------------- |
| `disable`                     | `8.0.404`                  |
| `patch`, `latestPatch`, unset | `>=8.0.404 && <8.0.500`    |
| `feature`, `latestFeature`    | `~8.0.404`                 |
| `minor`, `latestMinor`        | `^8.0.404`                 |
| `major`, `latestMajor`        | `>=8.0.404`                |

Unrecognized values are treated as the default, since new modes get added over time.

`allowPrerelease` is not read. Ranges exclude pre-releases by semver convention and a pinned pre-release resolves to itself, which is close enough to the SDK's own behavior.

Writing a version back into `global.json` edits an existing file and never creates one, since the file also carries `msbuild-sdks` and `projects`. Removing one drops the `sdk` object only when nothing else is left in it.

## Configuration

.NET plugin can be configured with a `.prototools` file.

- `metadata-url` (string) - Base URL for release metadata, for mirrors and air-gapped networks. `releases-index.json` and `{channel}/releases.json` hang off it.
- `dist-url` (string) - Overrides the archive URL from the release metadata. Supports `{version}`, `{rid}` and `{extension}` tokens. Archives fetched this way are not checksum verified, because the metadata's hashes describe Microsoft's archives and not a mirror's.

```toml
[tools.dotnet]
metadata-url = "https://builds.dotnet.microsoft.com/dotnet/release-metadata"
dist-url = "https://mirror.example.com/dotnet/{version}/dotnet-sdk-{version}-{rid}.{extension}"
```

## Hooks

.NET plugin does not support hooks.

`proto activate` exports `DOTNET_ROOT`. The muxer does not need it, but MSBuild, the SDK resolvers, and any `dotnet` reached other than through this install do.

## Contributing

Build the plugin:

```shell
cargo build --target wasm32-wasip1
```

Test the plugin by running `proto` commands.

```shell
proto install dotnet-test
proto versions dotnet-test
```
