# pnpm toolchain fix plan

Fixes from the pnpm toolchain audit (2026-11-17, against moon v2.6.5 source, live-checked with moon
2.6.4 and the released `javascript_toolchain` 1.3.2 and `node_depman_toolchain` 1.1.1, on Node
24.22.3 with pnpm 10.33.4; tool behavior was checked directly with pnpm 8.15.9, 9.15.9, and the
latest of each current line, 10.34.9, 11.28.12 and 12.21.0). Covers the pnpm parts of:

- `toolchains/node-depman` (`node_depman_toolchain` 1.1.1, toolchain ID `pnpm`), which registers
  pnpm's metadata and config, and re-exports `node_depman_tool` 0.21.0 (`tools/node-depman`) as its
  tier 3 API. The tool was audited on 2026-11-06, and is only covered here where the toolchain
  depends on it.
- `toolchains/javascript` (`javascript_toolchain` 1.3.2, ID `javascript`), whose `Pnpm` arms install
  dependencies, read `pnpm-workspace.yaml` and parse `pnpm-lock.yaml`. Nub shares the workspace and
  lockfile code (`nub.lock` is pnpm's format), so several fixes reach `NUB.md` too.

Item numbers (#1–#12) match the audit findings.

## Testing

Build the WASM first. The test sandbox loads the prebuilt `.wasm`, so rebuild after every source
change; `moon run :build` can replay a stale cached build.

```bash
cargo build -p javascript_toolchain -p node_depman_toolchain --target wasm32-wasip1 --release
```

```bash
cargo nextest run -p javascript_toolchain -p node_depman_toolchain --no-default-features
```

In agent sessions, unset `AI_AGENT` and `CLAUDECODE` first, or wrapper banners can break JSON tests.

The install tests mock `load_toolchain_config` with `|_, _|`, which ignores the requested ID. The
pnpm arms pass `pnpm`, which is right, but new tests for config loading should assert the ID.

Findings marked **live** were reproduced with `~/.proto/tools/moon/2.6.4/moon` in a scratch git
repo: a root `package.json`, `packages/{a,b,c,ignored}`, and a `pnpm-workspace.yaml` with
`packages: ['packages/*', '!packages/ignored']`, a top-level `catalog`, a named `legacy` catalog and
a `patchedDependencies` entry from `pnpm patch` / `pnpm patch-commit`. `a` depends on catalog
entries, `b` (`workspace:*`), a GitHub dependency, a registry tarball URL, an npm alias and `file:`
/ `link:` directories; `c` injects `b`. The toolchains config was:

```yaml title=".moon/toolchains.yml"
javascript:
  packageManager: pnpm
node:
  version: '24.22.3'
pnpm:
  version: '10.33.4'
```

Write `pnpm: {}` rather than `pnpm:` with no value, which loads no toolchains at all (see "Outside
this repo"). Isolate pnpm with `pnpm_config_store_dir`, `npm_config_store_dir`, `PNPM_HOME` and the
`XDG_*` directories pointing at a scratch dir. A `javascript_toolchain` built from `master` loads in
moon 2.6.4 with
`javascript: { plugin: 'file://<CARGO_TARGET_DIR>/wasm32-wasip1/release/javascript_toolchain.wasm' }`,
since it has no tier 3; #2, #5 and #6 were re-checked that way. `node_depman_toolchain` from
`master` doesn't load (see "Release"), but its pnpm code matches 1.1.1.

`moon docker setup` was emulated by copying `.moon/docker/configs` to a fresh git repo, and
`docker prune` by copying `.moon/docker/sources` over it, with stdin from `/dev/null` as in a Docker
build.

**Checked, no change needed:**

- `vendor_dir_name` (`node_modules`): the root `node_modules` exists and isn't empty after an
  install with pnpm 10.33.4, 11.28.12 and 12.21.0, for `nodeLinker` `isolated`, `hoisted` and `pnp`,
  for `enableGlobalVirtualStore: true`, for pnpm 12's experimental `nodeLinker: { type: loaded }`,
  and for a custom `modulesDir` (pnpm 11), so moon doesn't treat installs as first installs.
- Frozen installs in CI: with `CI=true`, pnpm 10, 11 and 12 fail on a stale lockfile with
  `ERR_PNPM_OUTDATED_LOCKFILE` and install normally without one, so the plain `pnpm install`
  (`toolchains/javascript/src/tier2.rs:547-566`) needs no flag. pnpm 11's `pnpm ci` also deletes
  `node_modules` first, and moon already skips CI installs when `node_modules` exists.
- `pnpm install --prod --filter-prod <name>...` installs the focused projects, their workspace
  dependencies and the root's production dependencies, drops dev dependencies (including the
  dependencies' dev dependencies), and leaves the lockfile untouched, on pnpm 10.33.4, 11 and 12.
  `--filter-prod` and `pnpm dedupe` exist in every version from 8 to 12.
- Lockfiles: pnpm 10.34.9, 11.28.12 and 12.21.0 write byte-identical v9 lockfiles for the scratch
  workspace. Peer suffixes (`react-dom@18.3.1(react@18.3.1)`) only appear in `snapshots`, npm
  aliases are keyed by the real name, and the env document that pnpm 10+ writes for
  `packageManagerDependencies` and `configDependencies` parses with the multi-document loop
  (`lockfiles/pnpm.rs:16-20`). Catalog dependencies resolve through `catalog` and named `catalogs`
  into task hashes (live).
- `pnpm-workspace.yaml`: without `packages`, pnpm 10–12 treat only the root as a project, as the
  plugin does; an empty or comment-only file works; negated member globs are handled by moon's
  `GlobSet` (moon `crates/toolchain-plugin/src/toolchain_plugin.rs:179-183`). A numeric catalog
  value, which pnpm 12 accepts, parses, as serde_yaml reads scalars as strings.
- `.pnpmfile.mjs` (pnpm 11+) is covered by the `.pnpmfile.*` config glob. pnpm 11 reads only auth
  and registry settings from `.npmrc`, which the plugin never reads.
- Injected dependencies (`dependenciesMeta.<name>.injected`): pnpm 9+ symlinks them when peers match
  (`dedupeInjectedDeps`), as it did in the scratch repo, and their `workspace:` edge is already in
  the graph.

## Decisions needed

| Decision                                                                                                | Recommendation                                                                  |
| ------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| D1 (#8): write `workspace:` specifiers for pnpm when `dependencyVersionFormat` is `star` or `version*`? | Yes. pnpm 9+ only links `workspace:` specifiers by default                      |
| D2 (#9): with `devEngines.packageManager` present, sync its version instead of `packageManager`?        | Yes, with `insert_dev_engine_version`, and never add `packageManager` beside it |
| D3 (#10): fix installs that don't re-run after `pnpm-workspace.yaml` or `pnpm.*` changes in moon?       | Yes, in moon's install fingerprint. The plugin can't add to it                  |

None are decided yet.

## Phase 1: clear-cut fixes

### #1 Docker: copy pnpm patches into the skeleton

- [ ] Add `define_docker_metadata` with `patches/**/*`
- [ ] Test

**Problem:** `pnpm patch-commit` writes `patches/<name>@<version>.patch` and records it in
`patchedDependencies` (in `pnpm-workspace.yaml` since pnpm 10, in `package.json`'s `pnpm` field
before). pnpm reads every listed patch during install, frozen or not. The JavaScript toolchain has
no `define_docker_metadata` or `scaffold_docker` (`toolchains/javascript/src/tier1.rs`), so moon
only copies the toolchains' config files, lockfiles and manifests (moon
`crates/toolchain-plugin/src/toolchain_plugin.rs:305-319`). **Live:** `moon docker scaffold a`
copied no `patches/`, and `moon docker setup` in the configs skeleton failed with
`ENOENT: no such file or directory, open '…/patches/is-number@7.0.0.patch'` (pnpm 10.33.4). Copying
the directory in got past the error. The sources phase doesn't copy it either, so `docker prune`'s
install fails the same way.

**Change:** add `define_docker_metadata`, returning `scaffold_globs: ["patches/**/*"]`. moon applies
scaffold globs at the workspace root and in every project root, in both phases (moon
`crates/app/src/commands/docker/scaffold.rs:76-124`). `patches/` is also the default of `bun patch`
and patch-package, so return it for every package manager.

**Not covered:** a custom `patchesDir`, or patch paths outside `patches/`. Reading
`patchedDependencies` in a `scaffold_docker` root call would cover them, and `JAVASCRIPT.md` #12
adds `scaffold_docker` anyway.

This item also fixes `NPM.md` #4 and `BUN.md` #4, which reproduced the same failure live with
`npm patch` (npm 12.2.2, `EPATCHNOTFOUND`) and `bun patch` (Bun 1.4.0, "Couldn't find patch
file"). Both point here.

**Tests** (`tier1_test.rs`): `define_docker_metadata` returns the glob, and a scaffold with a pnpm,
an npm 12 and a Bun `patchedDependencies` entry copies each patch in the configs phase.

### #2 `workspace:` ranges fail the whole project graph

**Done (2026-11-17, JavaScript pass):** upstream as `WorkspaceProtocol::Requirement { alias, req }`,
with `VersionProtocol` deserializing anything else to `Unknown`. Until `nodejs_package_json` 0.5.0 is
published, the root `Cargo.toml` patches it locally (`JAVASCRIPT.md` "Release").

- [x] Accept any range after `workspace:` in `nodejs_package_json`
- [x] Bump the crate and map the new variant
- [x] Tests

**Problem:** pnpm accepts any semver range after `workspace:`, like `workspace:^1`, `workspace:~1.2`
or `workspace:>=1.0.0`. `nodejs_package_json` 0.4.11's `WorkspaceProtocol::from_str` only accepts
`*`, `^` or `~` alone or followed by a full version, a path, or a full version
(`src/protocols/workspace.rs:59-100`, all through `Version::parse`). `VersionProtocol` deserializes
with `try_from = "String"`, so one such dependency fails the whole `package.json`, and
`extend_project_graph` loads every project's manifest with `?` (`tier2.rs:35`). **Live:** with
`"b": "workspace:^1"` in `packages/c/package.json`, pnpm 10.33.4, 11.28.12 and 12.21.0 install it
(`specifier: workspace:^1`, `version: link:../b`), while `moon run c:hi` fails with "Failed to parse
JSON file /workspace/packages/c/package.json … Failed to parse version: unexpected end of input
while parsing major version number", with the released plugin and a `master` build. Catalog values
in `pnpm-workspace.yaml` go through the same parser.

**Change:** fix it upstream (`milesj/rust-js-ecosystem`), as ADR-0008 leaves other crates'
strictness to them: parse what follows `workspace:` (after an alias) as a requirement when it isn't
`*`, `^`, `~`, a path or a full version, in a new variant like
`WorkspaceProtocol::Requirement(VersionReq)`, and return `VersionProtocol::Unknown` instead of an
error for anything else (ADR-0008.4). Then bump the crate, and map the new variant to
`parse_manifest_version` in `create_manifest_dependency` (`tier2.rs:772-780`).
`extend_project_graph` already links every `Workspace(_)` (`tier2.rs:87`).

**Tests:** in the crate, `workspace:^1`, `workspace:~1.2`, `workspace:1`, `workspace:>=1.0.0` and
`workspace:foo@^1`. In `tier2_test.rs`, `extend_project_graph` links a `workspace:^1` dependency,
and `parse_manifest` reports its requirement.

### #3 Production installs fail without a TTY when `node_modules` is kept

- [ ] Pass `--config.confirmModulesPurge=false` to production installs
- [ ] Tests

**Problem:** `docker prune` calls `prune_docker`, which deletes the root `node_modules` only when
`docker.prune.deleteVendorDirectories` is on, and then runs
`pnpm install --prod --filter-prod <name>...` (moon
`crates/app/src/commands/docker/prune.rs:141-209`, `tier2.rs:547-566`). When `node_modules` holds
dev dependencies, pnpm has to remove and recreate it, and asks first. Docker builds have no TTY and
usually no `CI`, so pnpm 10 and 11 abort. **Live:** with `deleteVendorDirectories: false`,
`moon docker setup` followed by `moon docker prune` failed with
`ERR_PNPM_ABORTED_REMOVE_MODULES_DIR_NO_TTY  Aborted removal of modules directory due to no TTY`
(pnpm 10.33.4). Run directly, pnpm 10.33.4 and 11.28.12 fail the same way, pnpm 8.15.9 and 9.15.9
take the prompt's default, and 12.21.0 doesn't ask.

**Change:** add `--config.confirmModulesPurge=false` when `input.production` is set. pnpm 8, 9,
10.33.4, 11 and 12 accept it, and complete the install. Keep it off for normal installs, where a
user at a terminal should still be asked.

**Not covered:** with `deleteVendorDirectories: false`, dev packages stay in `node_modules/.pnpm`
after the production install (pnpm 10.33.4, 11 and 12 all kept `is-odd` and `typescript`), so the
image isn't smaller. That's what the setting asks for.

**Tests:** `focused_commands` (`tier2_test.rs:1898`) expects the flag, and `default_commands`
doesn't get it.

### #4 Editing a patch doesn't change task hashes

- [ ] Add each package's patch hash to its lock entry
- [ ] Fixture and test

**Problem:** moon records a dependency's locked `hash`, the integrity, in task hashes (moon
`crates/task-hasher/src/task_hashing.rs:456-468`). A patch doesn't change the integrity. pnpm
records it as a `patch_hash` suffix on the package's key, in `snapshots` for v9
(`is-number@7.0.0(patch_hash=0a2b…): {}`) and in `packages` for v6, plus in the top-level
`patchedDependencies`. `parse_pnpm_lock_yaml` only reads `packages` and drops everything after `(`
(`toolchains/javascript/src/lockfiles/pnpm.rs:22-39`). **Live:** after editing
`patches/is-number@7.0.0.patch` and running `pnpm install`, the lockfile's patch hash went from
`0a2bcd01…` to `badbc1cc…` and `packages/b/node_modules/is-number` had the new patch, but
`moon run b:hi` was a cache hit with the same hash (`87e05cc3`).

**Change:** also read the `snapshots` keys (`FxHashMap<String, serde::de::IgnoredAny>`). For a key
whose parentheses hold `patch_hash=<hash>`, append the patch hash to the `hash` of the matching
`name@version` entry, like `<integrity>+patch.<hash>`. For v6, take it from the `packages` key
itself (after #6). Snapshot keys name an exact version, unlike `patchedDependencies` selectors,
which can be a bare name or a range, and their format changed in pnpm 11.

**Tests:** a fixture with a patched package, whose entry's hash includes the patch hash, and an
unpatched one, whose hash stays the integrity. `nub.lock` gets this too.

### #5 The default catalog is dropped from hashes when it's named `default`

- [ ] Map `catalogs.default` and `catalog:default` to the default catalog
- [ ] Tests

**Problem:** pnpm's default catalog can be defined as the top-level `catalog` or as
`catalogs.default`, and referenced as `catalog:` or `catalog:default`. The plugin stores the
top-level one as `__default__` (`lockfiles/pnpm.rs:54-66`), and looks `catalog:` up as `__default__`
and `catalog:<name>` by name (`tier2.rs:726-739`). So `catalog:default` misses a top-level
`catalog`, `catalog:` misses `catalogs.default`, and `parse_manifest` skips the dependency in task
hashes and install fingerprints. **Live** (pnpm 10.33.4 installs both forms): with
`"react": "catalog:default"`, `react` was missing from `a:hi`'s hash while `react-dom` (`catalog:`)
was there, and with the default catalog moved to `catalogs.default`, both were missing. A `master`
build drops `catalog:default` too.

**Change:** in `PnpmWorkspace::extract_catalogs`, move `catalogs.default` to `__default__` (pnpm
rejects defining both). In `create_manifest_dependency`, treat `Catalog(Some("default"))` like
`Catalog(None)`; the lookup is shared with Bun's and Yarn's catalogs.

**Tests:** extend `supports_catalogs_pnpm` (`tier2_test.rs:2374`) with a `catalog:default`
dependency, and add a fixture that defines `catalogs.default`.

### #6 pnpm 8 lockfiles give task hashes no locked versions

- [ ] Strip the leading `/` from v6 keys
- [ ] v6 fixture and test

**Problem:** lockfile v6 (pnpm 8) keys start with `/`: `/react@18.3.1`, `/@babel/core@7.22.5`,
`/react-dom@18.3.1(react@18.3.1)`. `parse_name_and_version` (`lockfiles/mod.rs:34-67`) then returns
the name `/react`, and for scoped packages the name `/` with a version like `babel/core@7.22.5`.
moon looks lock entries up by package name (`task_hashing.rs:439`), finds none, and records the
manifest requirement instead (`:474-479`). **Live:** with pnpm 8.15.9's `lockfileVersion: '6.0'`
lockfile, `a:hi`'s hash recorded `react: ^18.3.1`, `lodash: ^4.17.0` and `is-odd: ^3.0.1`, where the
v9 lockfile gives their integrities. So a lockfile bump within the range never changes a task hash.
Same with a `master` build.

**Change:** in `parse_pnpm_lock_yaml`, strip one leading `/` from each key before splitting. Leave
`parse_name_and_version` alone, since Bun and Deno share it.

**Not covered:** lockfile v5 (pnpm 7, `/name/1.0.0_peer`), which has no `@` before the version.
After the change those keys are skipped, instead of landing under the name `/`.

**Tests:** a `pnpm-v6` fixture generated with pnpm 8.15.9 from the same manifests as the `pnpm`
fixture, expecting the same dependencies.

### #7 Small fixes

- [ ] `register_toolchain` (`toolchains/node-depman/src/tier1.rs:37`): add `pn` and `pnx` to pnpm's
      `exe_names`. pnpm 11 and 12 ship them (pnpm 11.28.12's `bin` maps `pn` to `bin/pnpm.mjs`), the
      tool creates shims for them, and the JavaScript toolchain already lists them
      (`toolchains/javascript/src/tier1.rs:89-92`). Update `handles_pnpm` in `tier1_test.rs`.
- [ ] `initialize_toolchain`'s `config_url` (`toolchains/node-depman/src/tier1.rs:65`) is
      `https://moonrepo.dev/docs/guides/javascript`, which returns 404, as moon v2.6.5 has no
      `guides/javascript` index page. Point it at `guides/javascript/node-handbook`, which exists.
      npm, Yarn and Nub share this line, and `NPM.md` #5 has the same fix; make it once.

## Phase 2: after decisions

### #8 `star` and `version` formats break pnpm installs (D1)

- [ ] Map them to `workspace:` formats for pnpm
- [ ] Tests

**Problem:** `dependencyVersionFormat` picks the specifier `sync_project` writes for project
dependencies, and `get_supported_for` passes every format through for pnpm
(`toolchains/javascript/src/config.rs:94`). Since pnpm 9, `linkWorkspacePackages` defaults to
`false`, so only `workspace:` specifiers link a workspace package, and `*` or a plain range resolves
from the registry. Confirmed for a package that isn't published: `*` and `^1.0.0` link with pnpm
8.15.9, and fail with `ERR_PNPM_FETCH_404` on pnpm 10.33.4, 11.28.12 and 12.21.0, while
`workspace:^` links on all four. **Live:** with `dependencyVersionFormat: version-caret` and
`dependsOn: [zzb]`, moon synced `"moon-audit-zz-b": "^1.0.0"` into `packages/a/package.json`, and
the install failed with `ERR_PNPM_FETCH_404`. For a published name, pnpm silently installs the
registry copy instead of the local one.

**Recommended:** for pnpm, map `Star` and `Version` to `Workspace`, `VersionCaret` to
`WorkspaceCaret`, and `VersionTilde` to `WorkspaceTilde`, as npm already maps the `workspace`
formats to `file` (`config.rs:84-93`). `pnpm publish` replaces `workspace:*`, `workspace:^` and
`workspace:~` with the exact, caret and tilde version, so published manifests keep the requested
form, and repos with `linkWorkspacePackages: true` lose nothing.

**Alternative:** keep the formats, and document that pnpm 9+ needs `linkWorkspacePackages: true`.

**Not covered:** `file`, which pnpm installs as a hard-linked copy that a later build of the
dependency doesn't update.

**Tests** (`tier1_sync_test.rs`): pnpm with `star`, `version`, `version-caret` and `version-tilde`.

### #9 `packageManager` is synced beside `devEngines.packageManager` (D2)

- [ ] Sync `devEngines.packageManager` when it names the package manager
- [ ] Tests

**Problem:** `setup_environment` writes `packageManager: pnpm@<version>` whenever `version` is exact
(`toolchains/javascript/src/tier2.rs:843-873`). Since pnpm 11, `pnpm init` writes
`devEngines.packageManager` instead, and pnpm 11 and 12 ignore `packageManager` when it's present.
Confirmed: pnpm 11.28.12 warns
`"packageManager" and "devEngines.packageManager" specify different versions of pnpm in package.json. "packageManager" will be ignored`
on every install, even when the version satisfies the range, and pnpm 12.21.0 ignores it silently.
**Live:** with only `devEngines.packageManager: { name: pnpm, version: ^10.0.0 }`, moon added
`"packageManager": "pnpm@10.33.4"` beside it. On pnpm 11+, the sync only adds a warning, and when
the `devEngines` range excludes moon's version, pnpm (`pmOnFail: download` by default) downloads and
runs another pnpm.

**Recommended:** when `devEngines.packageManager` (an object, or an entry of an array) names the
package manager, set its `version` to the configured version with `lang-javascript-common`'s
`insert_dev_engine_version`, which the tool's `pin_version` uses, keep its `onFail`, and don't add
`packageManager`. Otherwise keep writing `packageManager`. The code is shared by npm, Yarn and Nub.
Keep writing `packageManager` for Yarn either way: Yarn 6's Yarn Switch reads only
`packageManager`, not `devEngines.packageManager` (zpm `packages/zpm-switch/src/manifest.rs:115-125`,
from `YARN.md`).

**Alternative:** skip the sync when `devEngines.packageManager` names the package manager, leaving
its range to the user.

**Tests:** a `package.json` with `devEngines.packageManager` gets its version updated and no
`packageManager`; one without keeps today's behavior.

### #10 Installs don't re-run after `pnpm-workspace.yaml` or `pnpm.*` changes (D3)

- [ ] moon: hash the package manager toolchain's config and the config files in the install
      fingerprint
- [ ] Re-check live once moon ships it

**Problem:** moon's install fingerprint holds the `javascript` toolchain's config, the first
lockfile, and manifest dependencies when a manifest changed (moon
`crates/actions/src/actions/install_dependencies.rs:137-143`, `:272-319`). Two things that change an
install aren't in it:

- `pnpm.installArgs` and `pnpm.version` (and npm's and Yarn's, per `YARN.md`), which the JavaScript
  plugin reads from the package manager's toolchain
  (`tier2.rs:420-423`). **Live:** changing `installArgs` to `['--config.node-linker=hoisted']`, and
  then `version` from 10.33.4 to 10.18.0, both logged "No JavaScript toolchain changes since last
  run, skipping install". This applies to every JavaScript package manager.
- `pnpm-workspace.yaml`, which holds catalogs, `overrides`, `patchedDependencies` and the member
  globs. **Live:** after a baseline run, changing `catalog.react` from `18.2.0` to `18.3.1` skipped
  the install, and `packages/a/node_modules/react` stayed 18.2.0. From the source, the task hash
  changes, since `parse_manifest` resolves the catalog, so tasks re-run against the old version.

**Recommended:** fix it in moon (see `MOON_COMPACT.md`). The plugin has no way to add to the
fingerprint, and listing `pnpm-workspace.yaml` as a manifest would send it to `parse_manifest` and
project detection.

**Not covered:** a hand edit to a patch file without `pnpm install`. `pnpm patch-commit` installs
and updates the lockfile itself.

## Phase 3: lower priority

### #11 Read integrities from v13 lockfiles

- [ ] Fall back to a top-level `integrity`

pnpm 12.19 added an experimental `lockfile.format: v13`: JSON-style YAML (tabs, trailing commas)
with `lockfileVersion: "13.0"`, one `packages` entry per version, and `integrity` at the entry's top
level instead of under `resolution`. pnpm reads both formats whatever the setting is. serde_yaml
parses it, and the parser already treats it as v9 (ADR-0008.9), but `resolution.integrity` is
missing. **Live:** with a v13 lockfile from pnpm 12.21.0, `a:hi`'s hash recorded versions
(`react: 18.3.1`) instead of integrities. Add `integrity` to `PnpmLockPackage` and use
`resolution.integrity.or(integrity)`. Versions are immutable, so this is only about consistency, and
the format may still change.

### #12 Document that `installArgs` don't reach `pnpm dedupe`

- [ ] Update the `installArgs` description

The dedupe command is a bare `pnpm dedupe` (`tier2.rs:625-645`). With a layout flag like
`--shamefully-hoist` in `installArgs`, pnpm 11.28.12's `dedupe` has to purge `node_modules`, and
fails without a TTY (`ERR_PNPM_ABORTED_REMOVE_MODULES_DIR_NO_TTY`, confirmed directly; pnpm 10.33.4
keeps the hoisting). Passing the args on isn't an option, since pnpm 12's `dedupe` only takes a few
flags (`--check`, `--lockfile-only` and so on). pnpm 11 reads settings only from
`pnpm-workspace.yaml` anyway, so extend `PnpmToolchainConfig.install_args`'s description
(`toolchains/node-depman/src/config.rs:38-40`) to say that `nodeLinker`, `shamefullyHoist` and other
layout settings belong in `pnpm-workspace.yaml`, as `pnpm dedupe` doesn't get these arguments.

## Not planned

- **Using `pnpm ci` (pnpm 11+) in CI:** pnpm already installs frozen in CI, `pnpm ci` deletes
  `node_modules` every time, and moon skips CI installs when `node_modules` exists.
- **Per-project versions from `importers`:** moon looks lock entries up by name across the workspace
  (`task_hashing.rs:439-454`), so the exact importer version can't be passed on.
- **Git and tarball URL dependencies in hashes:** their lock keys hold `://` and are skipped, but
  fixing that changes nothing, as moon skips manifest dependencies without a version requirement
  (`task_hashing.rs:433-436`).
- **The env lockfile document:** its `configDependencies`, `pnpm` and `@pnpm/exe` platform packages
  are merged into the lock entries (`lockfiles/pnpm.rs:16-20`). A config dependency that a project
  also uses at another satisfying version could be picked instead of the project's, which seems
  rare.
- **Lockfile v5 (pnpm 7)**, **`sharedWorkspaceLockfile: false`** (a member with its own lockfile
  becomes its own dependency root, and `pnpm install` there installs the whole workspace), and
  **graph edges for `*` without `linkWorkspacePackages`**, which pnpm 9+ installs from the registry
  when the package is published.
- **Catalog entries pointing at `workspace:`:** a `catalog:` dependency never becomes a graph edge
  (`tier2.rs:78-89`), even when the catalog says `workspace:*`.
- **Injected copies after a rebuild:** when peers differ, pnpm copies an injected package, and moon
  tasks don't trigger pnpm's `syncInjectedDepsAfterScripts`.
- **v1's `NODE_PATH`:** moon v1 set `NODE_PATH` to `node_modules/.pnpm/node_modules` for `node`
  tasks under pnpm (v1.41.8 `legacy/node/platform/src/actions/run_target.rs:85`), so phantom
  dependencies resolved. v2 doesn't, which matches running `node` under pnpm directly (handed over by
  `NODE.md`; not checked live). Revisit if migrating repos report `Cannot find module` for packages
  they don't declare.

## Outside this repo

moon changes are tracked in [`MOON_COMPACT.md`](MOON_COMPACT.md). This audit added toolchain
configs and config files to "Fingerprint everything that changes an install" (#10), and "A
toolchain key with no value loads no toolchains".

**nodejs_package_json** (`milesj/rust-js-ecosystem`):

- [ ] Accept any range after `workspace:`, and fall back to `Unknown` instead of failing the file
      (#2, live).

## Release

- `javascript_toolchain`: it has no tier 3, so it isn't blocked by the tools' proto 0.64 minimum; a
  `master` build loads in moon 2.6.4 (checked). Phase 1 changes Docker builds (#1, #3), the graph
  (#2, after the crate bump) and task hashes (#4–#6), so ship it as 1.4.0, with a changelog callout
  that task hashes change once for patched dependencies, `catalog:default` dependencies and pnpm 8
  lockfiles. Phase 2's #8 and #9 change what moon writes to `package.json`, so they need callouts
  too.
- `node_depman_toolchain`: #7 and #12 are a patch, but the toolchain embeds `node_depman_tool`
  0.21.0, which sets `minimum_proto_version` 0.64 (`tools/node-depman/src/proto.rs:44`). moon 2.6.x
  embeds proto_core 0.62, so it can't be released until moon ships proto 0.64, as `PYTHON.md`
  describes.
- `node_depman_tool`: nothing.

## Handed to other plans

- **JAVASCRIPT.md:** `docker scaffold` doesn't copy the `package.json` of workspace members that
  aren't moon projects, or `file:` directories outside projects. With pnpm 10.33.4, `docker setup`
  failed with `ERR_PNPM_WORKSPACE_PKG_NOT_FOUND` and `ERR_PNPM_LINKED_PKG_DIR_NOT_FOUND` (live); for
  pnpm the members come from `pnpm-workspace.yaml`.
- **JAVASCRIPT.md:** `prune_docker` (`tier1.rs:323-335`) only deletes the root `node_modules`. With
  pnpm's isolated layout, focused projects keep dangling links to dev dependencies, and unfocused
  projects keep their `node_modules` (live).
- **JAVASCRIPT.md:** moon fingerprints only the first existing entry of `lock_file_names`
  (`tier1.rs:56-71`), so a stray `bun.lock`, `package-lock.json` or `nub.lock` in a pnpm repo hides
  `pnpm-lock.yaml` changes.
- **JAVASCRIPT.md:** npm-alias dependencies (`"string-width-cjs": "npm:string-width@^4.2.0"`) never
  reach task hashes (live), as the lock entry is under the real name.
- **JAVASCRIPT.md:** one `package.json` that fails to parse fails `extend_project_graph` for the
  whole workspace (`tier2.rs:35`); see #2.
- **JAVASCRIPT.md:** `detect_package_manager` (`tier1.rs:155-179`) only reads `packageManager`, not
  `devEngines.packageManager`, which `pnpm init` writes since pnpm 11.
- **JAVASCRIPT.md:** `initialize_toolchain`'s `config_url` (`tier1.rs:106`) is the same 404
  (`guides/javascript`) as #7.
- **NODE.md:** pnpm (10.33.4 and 11.28.12 checked) installs a `devEngines.runtime` Node into
  `node_modules/.bin/node`, which the JavaScript toolchain puts on PATH ahead of moon's Node. With
  `node.version: 24.22.3`, a task printed `v22.22.3` while its hash recorded 24.22.3 (live).
- **NUB.md:** Nub shares `PnpmWorkspace` and `parse_pnpm_lock_yaml` (`nub.lock`), so #4, #5, #6 and
  #11 apply to it.
- **NPM.md / YARN.md:** #9's `devEngines.packageManager` sync is shared code; confirm how npm and
  Yarn treat the two fields.
- **NPM.md / YARN.md / NUB.md:** #7's `config_url` 404 is the same line for every package manager in
  `node_depman_toolchain`.
