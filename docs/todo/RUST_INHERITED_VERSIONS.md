# Rust toolchain: resolve inherited dependency versions

Plan for `toolchains/rust` (`rust_toolchain` 1.0.12), written 2026-11-04. Follows moon PR
[moonrepo/moon#2799](https://github.com/moonrepo/moon/pull/2799), which fixed the task hasher side.
This builds on `RUST.md` #4 (`workspace = true` path resolution) and #12 (`parse_manifest`
fidelity). There are no plugin API changes: every field used here already exists.

## Testing

Build the WASM first. The test sandbox loads the prebuilt `.wasm`, so rebuild after every source
change; `moon run :build` can replay a stale cached build.

```bash
cargo build -p rust_toolchain --target wasm32-wasip1 --release
```

```bash
cargo nextest run -p rust_toolchain --no-default-features
```

In agent sessions, unset `AI_AGENT` and `CLAUDECODE` first, or wrapper banners can break JSON tests.

## Decisions needed

| Decision                                        | Recommendation                             |
| ----------------------------------------------- | ------------------------------------------ |
| D1: keep `inherited: true` on resolved entries? | Yes, see **Compatibility** under Change    |

Only moon's task hasher reads the flag. Keeping it is accurate, and moon with #2799 uses the
resolved version first. Dropping it (emitting a plain version) would also fix moon hosts older than
#2799, as they use a non-inherited entry's version directly, but it misreports the manifest.

## Problem

`parse_manifest` (`tier2.rs:402`) returns `workspace = true` dependencies without a version:
`ManifestDependency::Inherited(true)`, or `Config { inherited: true, features }` when the entry has
features (`tier2.rs:419-447`). moon then looks the version up in the parsed workspace manifest,
where `[workspace.dependencies]` is merged into `dependencies` (`tier2.rs:468-470`) before the
package's own tables, which overwrite it (`tier2.rs:473-481`). Two cases break:

1. **Inherited versions are lost when the root package inherits the same dependency.** In a
   workspace root that's also a package, `[workspace.dependencies] serde = "1"` and
   `[dependencies] serde = { workspace = true }` share the `serde` key, and the inherited marker
   replaces the version. That loses the version for the root package and for every member that
   inherits `serde`. moon #2799 falls back to hashing every locked `serde` version, which has two
   gaps:
   - With `hasher.optimization: performance`, no lockfile is read, so the dependency isn't hashed
     at all.
   - With `accuracy`, a workspace version change is missed when the old and new versions both stay
     locked. For example, `syn = "1"` becomes `"2"`, and both are in `Cargo.lock` through other
     crates. The locked set is the same, so the hash doesn't change, and tasks can reuse stale
     outputs. Members are most exposed, as their task inputs don't include the root `Cargo.toml`.
     (Copilot's second review comment on #2799.)
2. **Members can inherit the wrong version.** When the root package declares its own
   `[dependencies] foo = "0.5"` next to `[workspace.dependencies] foo = "1"`, the merged map only
   holds `^0.5`, so moon resolves a member's `foo = { workspace = true }` to `^0.5`.

The plugin reads the real `[workspace.dependencies]` table, so it can fix both. The JS toolchain
already does the same for `catalog:` versions (`load_catalogs` in `javascript/src/tier2.rs`).

## Change

- [ ] Resolve inherited entries in `parse_manifest`
- [ ] Merge features like Cargo
- [ ] Fixtures and tests
- [ ] Changelog

**Resolve:** in `parse_manifest`, if any dependency table (including `[target.*]`) has a
`Dependency::Inherited`, find the workspace's dependencies with `locate_workspace_deps`
(`tier2.rs:135`), like `extend_project_graph` does. It walks up from the manifest's own directory,
so a root package finds its own file, and members find the root. Pass a local cache map, as each
call parses a single manifest (moon caches `parse_manifest` results per input for the process).

For each inherited entry whose name is in `[workspace.dependencies]`, emit
`ManifestDependency::Config(ManifestDependencyConfig { inherited: true, .. })` with:

- `version`: the workspace entry's version (a simple string, or a detailed `version`), through
  `parse_requirement` (`tier2.rs:488`), so `1.2.3` becomes `^1.2.3` like other entries.
- `features`: the workspace entry's features, then the member's, deduplicated. Cargo's features on
  a `workspace = true` entry are additive with the workspace's.
- `path` and `url` (from `git`) from the workspace entry, so a path-only or git dependency is still
  described accurately, even without a version.

Keep today's output when the name isn't in `[workspace.dependencies]` or no workspace is found.
Cargo rejects that manifest anyway.

**Keep:** merging `[workspace.dependencies]` into the workspace manifest's `dependencies`. Moon
hosts before #2799 look inherited versions up there, so removing it would regress them. It also
means a root package's production dependencies include every workspace dependency, which hashes
more than it uses; that's out of scope here.

**Compatibility:** with D1's recommendation, moon hosts before #2799 ignore a version on an
inherited entry, and keep their current (scope-only) lookup, so nothing changes for them. With
#2799, the resolved version is used first, which fixes both cases above in both hasher modes. In
accuracy mode, the lockfile is matched against the real requirement instead of the
every-locked-version fallback, so a requirement change that selects another locked version changes
the hash.

## Tests

In `tier2_test.rs`, `mod parse_manifest`:

- **Update `parses_package`** (`__fixtures__/files`): `c.workspace = true` becomes
  `Config { inherited: true, version: ^7.8.9, features: ["on"] }`. `d` isn't in the fixture's
  `[workspace.dependencies]`, so it stays `Config { inherited: true, features: ["off"] }`, which
  covers the unresolved case.
- **New fixture `__fixtures__/root-package`:** a workspace root with a `[package]`, where
  `[workspace.dependencies]` has `serde = "1"`, `foo = "1"`, `core = { path = "core" }` and a git
  entry, and the root's own `[dependencies]` has `serde = { workspace = true }` and `foo = "0.5"`. A
  `member/Cargo.toml` inherits `serde` under `[dependencies]`, `foo`, `core` and the git entry under
  `[dev-dependencies]`, plus something under `[target.'cfg(unix)'.build-dependencies]`. Assert:
  - The root's and the member's `serde` are both `^1` (case 1).
  - The member's `foo` is `^1`, not `^0.5` (case 2).
  - `core` has its `path` and no version, the git entry has its `url`, and the target-table entry is
    resolved.
  - Member features are merged after the workspace's.

## Outside this repo (moon)

- [x] moon #2799 (`crates/task-hasher/src/task_hashing.rs`): look inherited versions up in the
      workspace's production dependencies first, reuse a root package's own manifest, fall back to
      locked versions, and use an inherited entry's own version first. This replaces the moon item
      for `apply_toolchain_dependencies_by_scope` under "Outside this repo (moon)" in `RUST.md`.
- [x] moon #2799 commit `2a58332b7a` (Copilot's first review comment): the lookup stops at the first
      workspace entry with the name, even if it has no version. Before, a root package's own
      `[dev-dependencies] serde = "=0.9.0"` was used for a member's inherited dev `serde` when the
      root package also inherited `serde`. It now goes to the locked-version fallback.
- Not done in moon: when an inherited version can't be resolved, also hash a digest of the root
  `Cargo.toml`. It would close both gaps in case 1 without this change, but any edit to the root
  manifest would invalidate those tasks. This change makes it unnecessary.
- [ ] When #2799 is carried to `develop-3.0`, add an end-to-end hashing test there. The `tc-tier2`
      test plugin on 3.0 parses manifests and lockfiles; `master`'s doesn't.

## Release

Ship it with the next `rust_toolchain` release, with a 🐞 Fixes changelog entry. Task hashes change
for projects with inherited dependencies, so their tasks miss the cache once after upgrading.
