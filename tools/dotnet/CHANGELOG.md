# Changelog

## Unreleased

#### 🚀 Updates

- Initial release of the .NET proto plugin.
  - Installs .NET SDKs from Microsoft's published archives, verified against the
    SHA512 in the official release metadata.
  - Resolves versions from the release metadata rather than git tags, so every
    published SDK is available, previews included.
  - `lts`, `sts` and `preview` aliases follow .NET's release cadence, and are
    resolved from the release index alone rather than by listing every channel.
  - Detects `global.json`, mapping `sdk.version` and `rollForward` onto a
    version requirement, and supports `proto pin`/`unpin` writing `sdk.version`
    back into it.
  - Exports `DOTNET_ROOT` for `proto activate`, so MSBuild and the SDK
    resolvers find the SDK in a proto-activated shell.
  - Warns when a channel pin such as `8.0` spans SDK feature bands, since the
    highest is selected, and points at the compound requirement that pins a
    single band.
