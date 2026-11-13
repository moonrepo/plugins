---
status: accepted
date: 2026-11-10
applies-to: [backends/**, crates/**, extensions/**, toolchains/**, tools/**]
---

# Parse other tools' formats leniently

## Context

Plugins parse formats that other tools own:

- Lockfiles, manifests and version files.
- The output of commands like `cargo metadata` and `npm view`.
- Responses from registries like nodejs.org and foojay.

Those tools change their formats between releases by adding fields, entry shapes and values. When a
plugin's types don't match, the plugin fails on a file that the tool itself accepts. Users can't
fix it, and have to wait for a plugin release.

The changelogs list many of these failures:

- The JavaScript toolchain failed on `bun.lock` files with nested overrides (lockfile v3). It also
  failed on Git dependencies with both package metadata and an integrity hash, and panicked on an
  empty workspace entry. It failed on pnpm v10's multi-document `pnpm-lock.yaml` files too
  ([#132](https://github.com/moonrepo/plugins/pull/132)).
- The Go toolchain failed on `go.mod` files where `tool` is a list, and on `tool ()`.
- The Rust toolchain's `docker prune` failed with "unknown variant `custom-build`" when a crate has a
  build script.
- The npm backend failed to load versions with npm v12, which wraps `npm view --json` output in an
  array ([#198](https://github.com/moonrepo/plugins/issues/198)).
- Several tools, including Deno, Java and Poetry, failed on comments in version files, like
  `.dvmrc` and `.java-version`.

These failures stop work:

- moon's task hasher returns any error from a toolchain's `parse_manifest`. It does the same for
  `parse_lock` when `hasher.optimization` is `accuracy`. So a lockfile that the plugin can't parse
  fails every task that hashes it.
- A failed `load_versions` fails `proto install`.

Most types already lean lenient:

- Most lockfile and manifest structs set `#[serde(default)]` and only define the fields they use.
  The `bun.lock` types explain why: other fields change shape between lockfile versions.
- The Rust toolchain's `cargo metadata` types fall back with `#[serde(other)]`. The Java tool skips
  distributions it doesn't know with `lenient_distribution`.
- The Go toolchain skips a `go.mod` it can't parse, and the Poetry tool ignores a `requires-poetry`
  it can't parse. Both log a warning.
- No type for another tool's format uses `deny_unknown_fields`. Only plugin configs do.

When this ADR was written, these gaps remained:

- None of the 12 `#[serde(untagged)]` enums has a catch-all variant. Some cover every shape the
  format allows, like Deno's `lock` field. `bun.lock`'s package entries don't, since Bun adds
  shapes between releases.
- One value that can't be parsed, like a version, fails the whole call in some plugins and is
  skipped in others.
  - It fails the call in these parsers:
    - The JavaScript toolchain's `bun.lock`, `deno.lock`, `package-lock.json`, `pnpm-lock.yaml`,
      `yarn.lock` and `package.json`.
    - The Python toolchain's `poetry.lock`, `uv.lock` and `pylock.toml`.
    - The Go toolchain's `go.sum` and `go.mod`.
    - The Deno toolchain's `deno.json`.
    - The Rust toolchain's package version in `Cargo.toml`.
  - It's skipped in these:
    - The Rust toolchain's `Cargo.lock` and `Cargo.toml` dependencies.
    - The Ruby toolchain's `Gemfile.lock`.
    - The Python toolchain's requirements files and `pyproject.toml`.
- The JavaScript toolchain returns no dependencies for a `package-lock.json` with a
  `lockfileVersion` that it doesn't know, and logs nothing.

Version lists work differently, and that's intended:

- 15 tools build their versions with `LoadVersionsOutput::from`, which fails on the first value that
  `VersionSpec` can't parse. They are Bun, Deno, Go, Java, Kotlin, moon, proto, Python, Poetry, uv,
  Ruby, Rust, Swift, Zig and ZLS.
- Each tool converts the values it loads, like Git tags or PEP 440 versions, into a format that
  `VersionSpec` accepts. A value that `from` rejects is a bug in the tool's conversion.

## Decision

### Scope

1. These rules cover data in formats that other tools own:
   - Lockfiles and manifests.
   - Other tools' version and config files.
   - Command output.
   - Registry and API responses.

   The plugin's own config and the host's API types are out of scope.

### Types

2. Types for these formats SHOULD only define the fields that the plugin uses. They MUST NOT use
   `deny_unknown_fields`.
3. A field that the format may leave out MUST be an `Option` or have a default. Structs SHOULD set
   `#[serde(default)]`.
4. An enum of values that the tool may add to, like target kinds or entry shapes, MUST have one of
   these fallbacks:
   - `#[serde(other)]` on a unit variant.
   - A catch-all last variant in an untagged enum, like `Unknown(JsonValue)`.
   - A deserializer that returns `None`.

### Entries

5. Sometimes one entry can't be parsed, like a package in a lockfile or a dependency in a manifest.
   Plugin code MUST then skip that entry, or skip the field it can't parse, like a dependency's
   version. It MUST NOT fail the whole call. Rule 7 covers version lists.
6. Plugin code SHOULD log a skipped entry with `debug!`, not `host_log!`. The message names the file
   or source, the entry and the parser's error.
7. A tool MUST convert the versions it loads into a format that `VersionSpec` accepts before it
   calls `LoadVersionsOutput::from`. It MUST skip values that it can't convert. `from` then fails
   on any value it can't parse, since that's a bug in the tool's conversion, not in the data.

### Files

8. Plugin code MAY return an error when a file or response can't be parsed at all, like invalid
   JSON, since the tool that owns it would fail too. The error SHOULD name the file or URL, and
   include the parser's error. Where invalid files are expected, like test fixtures in Go's
   `testdata`, plugin code MAY skip the file instead, and log it with `debug!`.
9. A file may declare a format version that the plugin doesn't support, like a newer
   `lockfileVersion`. Plugin code SHOULD then parse it with the newest format it supports. It MUST
   NOT skip the file without logging it with `debug!`.

## Exceptions

- A tool's `parse_version_file` MAY return an error when the version it finds for its tool can't
  be parsed, instead of skipping it (rule 5). This covers a field in a larger file too, like
  `engines` in `package.json` or `required-version` in `uv.toml`. Skipped entries are only logged
  with `debug!`, so skipping would make proto use a different version from the one the user
  pinned, without telling them.

## Consequences

- Files and responses that their tool accepts no longer stop moon tasks or proto installs. The
  result can be less exact until the plugin supports the new shape, like a task hash without one
  dependency's version. Skipped entries only show up in debug logs, so users won't see that a
  result is less exact unless they look.
- Closing the gaps takes four kinds of change:
  - Adding catch-all variants to enums like `bun.lock`'s entries.
  - Skipping values in the parsers listed above.
  - Logging the Go toolchain's skipped `go.mod` and the Poetry tool's ignored `requires-poetry`
    with `debug!` instead of warnings.
  - Checking that the 15 tools skip every version they can't convert before
    `LoadVersionsOutput::from`.
- The Java tool's `lenient_distribution` could become a generic helper in `crates/common`, for rule
  4's deserializer ([ADR-0001.8](0001-record-architecture-decisions.md)).
- Parsers from other crates decide their own strictness. These include `cargo-lock`,
  `gomod-parser`, `package-lock-json-parser`, `yarn-lock-parser` and `deno_lockfile`. When one of
  them fails, plugins can only return its error (rule 8), and fixes go upstream.
- [ADR-0005](0005-plugin-code-does-not-panic-on-input.md) makes bad input return an error instead of
  panicking. This ADR narrows which bad input is an error at all.
- Only review enforces these rules.
