# npm toolchain fix plan

Fixes from the npm toolchain audit (2026-11-17, against moon v2.6.5 source, live-checked with moon
2.6.4, proto 0.64.0, npm 11.21.2 and 12.2.2 (the latest of each), and the npm 11.19.1 that Node
24.22.3 bundles). Covers everything npm-specific in:

- `toolchains/node-depman` (`node_depman_toolchain` 1.1.1, toolchain ID `npm`). One crate serves
  `npm`, `pnpm`, `yarn` and `unstable_nub`, and picks the package manager from the plugin ID. It
  only registers metadata, config and requirements, and re-exports `tools/node-depman` as tier 3.
- `toolchains/javascript` (`javascript_toolchain` 1.3.2, toolchain ID `javascript`), whose
  `JavaScriptPackageManager::Npm` arms install, dedupe, locate the dependencies root and parse
  `package-lock.json`. Most fixes land here.

`tools/node-depman` (`node_depman_tool` 0.21.0) was audited on 2026-11-06, and is only covered where
the toolchain depends on it. Item numbers (#1–#8) match the audit findings. Items marked "shared"
change code that pnpm, Yarn and Nub also use.

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

The only npm lockfile fixture (`tests/__fixtures__/lockfiles/npm`) is a v3 file with no nested or
aliased entries. The audit generated v1, v2 and v3 lockfiles from one workspace with
`npm install --lockfile-version=<n>` (npm 11.21.2), and a v4 one with npm 12.2.2 and a
`packageExtensions` entry. The workspace had `packages/{a,b,c}`, an `npm:` alias, a tarball URL
dependency, and a `ms` version conflict that npm nests under `packages/c/node_modules`. #1 should
add those four files as fixtures. `inherits_args_from_npm_toolchain` mocks `load_toolchain_config`
with `|_, _|` (`tests/tier2_test.rs:1576`), so it can't tell whether `npm`'s config was loaded; new
tests should assert the ID.

Findings marked **Live** were reproduced with `~/.proto/tools/moon/2.6.4/moon` in scratch git repos,
against the released `javascript` 1.3.2 and `npm` 1.1.1 plugins:

- **Single package:** a root `package.json` (`app: '.'`) with `ms` and a dev dependency.
- **Workspace:** a root with `"workspaces": ["packages/*"]` and a production dependency, and
  projects `packages/{a,b,c}`, where `a` depends on `b`.

`.moon/toolchains.yml` had `javascript: { packageManager: npm }`, `node: { version: '24.22.3' }` and
`npm: {}`, so tasks ran the npm bundled with Node. For npm 12, set `npm: { version: '12.2.2' }` and
run moon with a scratch `PROTO_HOME` (symlinking an installed Node into it saves the download).
`javascript_toolchain` embeds no proto tool, so a `master` build loads in moon 2.6.4, with
`javascript.plugin` pointing at the `.wasm`:
`file://<CARGO_TARGET_DIR>/wasm32-wasip1/release/javascript_toolchain.wasm`. That's how #1 was
confirmed on `master`. `node_depman_toolchain` from `master` can't load (see
"Release"). `moon docker setup` and `prune` were emulated by copying `.moon/docker/configs` into a
fresh git repo, and then the sources (without `.moon/cache`).

**Checked, no change needed:**

- `npm ci` runs in CI when a lockfile exists (live), and `npm ci --workspace` honors the filter.
- Focused production installs leave `package-lock.json` untouched. npm removes omitted dev
  dependencies that `prune_docker` left in nested `packages/*/node_modules` ("removed 1 package").
- `get_supported_for`'s npm row (`config.rs:84-93`) is right: npm 12.2.2 rejects `workspace:*` and
  `link:` with `EUNSUPPORTEDPROTOCOL`, and links `file:../b`. Graph edges from `file:` work (live).
- `npm init -w ./packages/a` writes `packages/a`, so moon's member globs match what npm writes.
- `install-strategy=linked` (npm 12) writes the same lockfile shape as the hoisted layout, and the
  root `node_modules` still exists, so `vendor_dir_name` holds.
- `moon toolchain add npm` and proto agree on the version: `devEngines.packageManager` wins over
  `packageManager` (live: `^11.13.0` over `npm@10.0.0`).
- `define_requirements` requires `node` for npm, and `setup_environment` returns nothing for npm,
  so there's no fingerprint to go stale (plugins#137).
- Task hashes record the lockfile integrity of hoisted dependencies (live: `kind-of`), and v1
  lockfiles keep nested versions.
- `.npmrc` reaches the configs skeleton, and `npm ci` tolerates a `workspaces` glob that matches a
  missing directory.
- npm 12.2.2 still accepts every flag the plugin emits. Only `--production` warns (#5).

## Decisions needed

| Decision                                                                    | Recommendation                                                       |
| --------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| D1 (#6): should focused `docker prune` installs keep the root's deps?       | Yes: add `--include-workspace-root` whenever `--workspace` is passed |
| D2 (#7): link npm workspace dependencies declared by version range?         | Moved to `JAVASCRIPT.md` D1, which also covers Yarn and Bun          |

## Phase 1: clear-cut fixes

### #1 Parse `package-lock.json` with our own types

- [ ] Replace `package-lock-json-parser` with types in `lockfiles/npm.rs`
- [ ] Keep nested entries, and name members by package name
- [ ] Unit tests with v1–v4 fixtures (ADR-0006)
- [ ] Changelog

**Problem:** `parse_package_lock_json` (`lockfiles/npm.rs:67-78`) uses `package-lock-json-parser`
0.4.0, which gets three things wrong:

- **npm 12 lockfiles fail every task.** npm 12 writes `lockfileVersion: 4` when a project uses
  `packageExtensions` or `npm patch` (`patchedDependencies`). A fresh v4 lockfile has no top-level
  `name` (npm only writes it while building the legacy v1–v3 data), but the crate's
  `PackageLockJson.name` is a required `String` (crate `lib.rs:16`, against ADR-0008.3).
  `parse_lock` then fails, and moon's task hasher returns that error for every task in the
  workspace under the default `hasher.optimization: accuracy` (moon `task_hashing.rs:273`).
  **Live:** with npm 12.2.2 and a `packageExtensions` entry, `moon run app:run` failed with "Error
  parsing file: missing field `name` at line 48 column 1" on 1.3.2, and with "Failed to parse
  /workspace/package-lock.json: …" on a `master` build. A v3 lockfile that npm upgrades to v4 keeps
  its `name`, so whether a repo fails depends on its history.
- **Nested versions are dropped.** The crate skips every `node_modules/x/node_modules/y` key (crate
  `lib.rs:178`). moon picks the first locked entry that satisfies a project's requirement, and
  records the requirement itself when none does (moon `task_hashing.rs:474`). **Live:** with `c`
  requiring `ms@~2.0.0` while the root hoists 2.1.3, c's task hash recorded `"ms": "~2.0.0"`
  instead of the nested 2.0.0's integrity, so a lockfile-only bump never changes it. In the
  generated v3 lockfile, `is-number` kept 1 of its 3 locked versions.
- **Members are keyed by path.** A workspace member's key, like `packages/a`, has no `name` field
  when it matches the folder name, and the crate then keeps the key (`lib.rs:191`). So does
  `packages/c/node_modules/ms`. The plugin's `strip_prefix("workspaces/")` (`npm.rs:51`) only helps
  when members happen to live in a `workspaces/` folder; npm adds no such prefix.

The crate also calls `.unwrap()` on each `engines` array entry (`lib.rs:162`), which panics on a
non-string value (ADR-0005.3).

**Change:** drop the crate and deserialize only what's used (ADR-0008.2), with every field optional:

- `lockfileVersion`, `packages` and `dependencies` at the top level, each entry's `name`, `version`,
  `integrity` and `link`, and each v1 entry's nested `dependencies`.
- Deserialize each `packages` entry on its own, and skip one that fails with `debug!` (ADR-0008.5).
- For v2 and later, skip the `""` root and `link: true` entries. The name is the text after the last
  `node_modules/` in the key. A key without one is a workspace member: use its `name`, or the last
  path segment, as npm omits `name` when it matches the folder. An alias is keyed by its alias
  (`node_modules/myms` with `name: ms`), which is the name the manifest uses, so keep the key.
- Push hoisted entries (`node_modules/<name>`) before nested ones, since moon takes the first
  satisfying entry and an unversioned requirement matches anything.
- For v1, walk `dependencies` recursively, as today.

**Not covered:** a member that uses its own nested copy while the hoisted copy also satisfies its
range gets the hoisted entry. v1 aliases (`"version": "npm:ms@2.0.0"`) stay skipped.

**Tests:** unit tests in `lockfiles/npm.rs`, which compile in test builds (ADR-0006): v1, v2, v3 and
v4 fixtures from "Testing". Assert that v4 parses, `ms` has 2.1.3 and 2.0.0, members are keyed `a`,
`b` and `c`, and `myms` and the tarball dependency have versions.

**Changelog:** task hashes change once for npm projects whose dependencies resolve to nested copies
or workspace members.

### #2 `docker prune` fails for a package at the root

- [ ] Leave the root package out of `--workspace`
- [ ] Tests

**Problem:** moon passes each focused project's JavaScript alias (its package name) as `packages`,
including a project at the dependencies root (moon `prune.rs:177`, `:225`). The npm arm turns every
name into `--workspace <name>` (`tier2.rs:507-510`), but npm only accepts workspace members there.
**Live:** in the single-package repo, `moon docker prune` first deleted `node_modules`
(`prune_docker`, `tier1.rs:323-335`), then failed with "npm error No workspaces found:
--workspace=app". Every single-package npm repo that follows moon's Docker flow hits this, and so
does a workspace whose root project is focused next to members (`--workspace=root` fails the same
way, confirmed with npm 12.2.2).

**Change:** load `<root>/package.json` and drop its `name` from `input.packages` before adding
`--workspace`. When nothing is left, run a plain production install. A moon-side alternative is in
`MOON_COMPACT.md`.

**Tests:** `packages` with only the root's name (no `--workspace`), and the root plus a member (only
the member).

### #3 `npm dedupe` ignores `installArgs`

- [ ] Pass the install args to `npm dedupe`
- [ ] Tests

**Problem:** `installArgs` are only appended to the install command (`tier2.rs:585-589`), not to
`npm dedupe` (`:611-617`). Dedupe re-resolves the whole tree, so a repo that needs
`--legacy-peer-deps`, common with React peer ranges, fails it with `ERESOLVE`. **Live:** with
`npm: { installArgs: ['--legacy-peer-deps'] }` and `react@18.3.1` plus `react-dom@17.0.2`, the first
run passed (moon skips dedupe on a first install). After a `package.json` edit,
`npm install --legacy-peer-deps` passed, then `npm dedupe` failed with "ERESOLVE could not resolve",
and the task never ran. A failed install doesn't store its fingerprint, so it fails on every run
until `dedupeOnLockfileChange: false`. `npm dedupe --legacy-peer-deps` works with npm 11.21.2 and
12.2.2.

**Change:** append `package_manager_config.install_args` to `npm dedupe` too. npm reads every flag
as config, so install flags apply to dedupe, and npm 12 only rejects unknown ones (confirmed:
`--bogus-flag` fails). A positional package name in `installArgs` would break dedupe, but that's
not a realistic use. `legacy-peer-deps=true` in `.npmrc` already reached both.

**Tests:** extend `inherits_args_from_npm_toolchain` to expect the args on dedupe, and assert the
`npm` ID in its mock.

### #4 `docker setup` without npm 12's patch files

- [x] Fixed by `PNPM.md` #1 (Bun pass, 2026-11-18)
- [x] Add an npm 12 case to its test: not needed, as the plugin only returns the glob, and moon
      copies the files

**Problem:** `npm patch` (npm 12) records patches in the root `package.json`, like
`"patchedDependencies": { "ms@2.1.3": "patches/ms@2.1.3.patch" }`, and a missing patch file is a
hard install error. The configs skeleton only gets config globs, lockfiles and manifests (moon
`toolchain_plugin.rs:313-315`). **Live:** `moon docker setup` with npm 12.2.2 failed with
"EPATCHNOTFOUND … patch file not found: patches/ms@2.1.3.patch".

**Change:** `PNPM.md` #1 adds `patches/**/*` as a JavaScript toolchain scaffold glob for every
package manager, which covers npm's default. This plan first proposed reading the
`patchedDependencies` paths in a `scaffold_docker` configs call instead, which also covers a custom
`patches-dir`. `JAVASCRIPT.md` #12 adds `scaffold_docker` anyway, so switch to that if custom
directories come up.

**Tests:** add a root `package.json` with an npm 12 patch entry to `PNPM.md` #1's test.

### #5 Small fixes

- [ ] `tier2.rs:503-505`: emit `--omit=dev` instead of `--production`. npm 12.2.2 still accepts it,
      but prints "npm warn config production Use `--omit=dev` instead." on every production
      install, and npm 12 now throws on unknown flags, so a later removal would fail them. `--omit`
      exists since npm 7.
- [ ] `toolchains/node-depman/src/tier1.rs:65` (shared): `initialize_toolchain`'s `config_url` is
      `https://moonrepo.dev/docs/guides/javascript`, which returns 404. Use
      `https://moonrepo.dev/docs/guides/javascript/node-handbook`, which exists.
- [x] `tools/node-depman/src/proto.rs:78` (shared): `parse_version_file` parses the whole
      `package.json` as `PackageJson`, so an invalid `version` or dependency specifier silently drops
      the `packageManager` and `devEngines.packageManager` pins. Switch to the version-fields struct
      that `NODE.md` #1 adds to `lang-javascript-common` (handed over by `NODE.md`; reproduced there
      for `node_tool`, not for this tool).
      Done in the JavaScript pass: `nodejs_package_json` now skips fields it can't parse.
- [ ] `toolchains/node-depman/src/config.rs:10`: `installArgs` says "`npm install` commands". Say
      it also applies to `npm ci` and, after #3, `npm dedupe`.

**Tests:** update the production expectations in `tier2_test.rs` (`focused_commands`).

## Phase 2: after decisions

### #6 Focused production installs drop the root's dependencies (D1)

- [ ] Add `--include-workspace-root` with `--workspace`
- [ ] Tests

**Problem:** `npm install --workspace a` installs `a` and its dependencies, including other
workspaces, but not the root package's own dependencies unless `--include-workspace-root` is passed
(confirmed with npm 11.21.2 and 12.2.2). **Live:** `moon docker prune` focused on `a` left
`a b kind-of ms` in `node_modules`, and dropped the root's production dependency `is-number`. With
`rootPackageDependenciesOnly`, moon's "one version policy" setting, every dependency lives in the
root, so the image ends up with no third-party packages at all. Apps that resolve a root dependency
through Node's parent `node_modules` lookup then fail at runtime.

**Recommended:** add `--include-workspace-root` whenever #2 leaves at least one `--workspace`. It
only adds the root's production dependencies, which are usually few. Confirmed:
`npm ci --omit=dev --workspace a --include-workspace-root` installs `is-number`. The flag exists
since npm 8.1 (checked in the published config definitions; 8.0.0 lacks it).

**Alternative:** add it only when `rootPackageDependenciesOnly` is on.

**Tests:** a focused production install expects the flag; an unfocused one doesn't.

### #7 Workspace dependencies declared by version range get no edge (D2)

- [ ] Fixed by `JAVASCRIPT.md` #15

**Merged into `JAVASCRIPT.md` #15 (D1)**, which applies the same rule to npm, Yarn and Bun and
keeps pnpm out. The npm evidence: `npm install b -w a` writes `"b": "^1.0.0"`, npm links
`node_modules/b` to `packages/b` whenever b's version satisfies it, and **live**, `moon project a`
showed no dependencies until the range was `*`. New edges can expose cycles, so #15 needs a
changelog callout.

## Phase 3: lower priority

### #8 Hash npm 12 patches

- [ ] Include `patched.integrity` in the locked hash

npm 12 records a patched entry as
`"patched": { "integrity": "sha512-…", "path": "patches/ms@2.1.3.patch" }` next to the tarball's own
`integrity`, which doesn't change when the patch does. Editing a patch therefore changes no task
hash unless the patch files are task inputs. In #1's parser, append the patch integrity to `hash`
when it's present.

## Not planned

- **Writing `^<version>` instead of `file:` for npm:** `get_supported_for` maps the default
  `workspace:*` to `file:../b` for npm (`config.rs:84-93`). A range would be publish-safe, but sync
  only adds missing dependencies, so the range is never bumped. Once the versions stop matching, npm
  installs the package from the registry instead of linking it. `file:` always links.
- **`npm-shrinkwrap.json` with npm 12:** npm 12 ignores it at the root. `npm install` silently
  writes a `package-lock.json` beside it, and `npm ci` fails with `EUSAGE` (confirmed). Keep using
  `npm ci` for a shrinkwrap in CI, so CI fails loudly rather than installing unlocked; npm's release
  notes tell users to rename the file.
- **npm 12's install policy:** dependency lifecycle scripts are blocked unless `allowScripts`
  allows them. The install still exits 0 with a warning (confirmed with `core-js`). `allow-git` and
  `allow-remote` default to `none`, and unknown or abbreviated flags in `installArgs` now fail.
  These are npm's policy, so they belong in moon's docs (see `MOON_COMPACT.md`).
- **`NODE_ENV=production` in a Dockerfile:** npm then omits dev dependencies from `docker setup`'s
  install too. That's the user's environment.
- **Hashing `.npmrc` into the install fingerprint.**

## Outside this repo

moon changes are tracked in [`MOON_COMPACT.md`](MOON_COMPACT.md). This audit added "Store the
install fingerprint after the install", the root project in "Focus `docker prune` installs by
package name, without the root" (a moon-side alternative to #2), npm's case in "Let plugins report
whether dependencies are installed", and the npm website docs.

## Release

- `javascript_toolchain`: #1–#3, #5's `--omit=dev`, #6 and #8 change task hashes (#1, #8) and
  installs (#2, #3, #6), so ship them as 1.4.0 with a callout for #1. #4 and #7 ship with
  `PNPM.md` #1 and `JAVASCRIPT.md` #15. The toolchain embeds no proto tool, so a `master` build
  loads in moon 2.6.4 (live), and it isn't blocked on proto 0.64.
- `node_depman_toolchain`: #5's `config_url` and description are a patch. It re-exports
  `node_depman_tool`, which sets `minimum_proto_version` 0.64, so like the Python toolchains it
  can't ship from `master` until moon embeds proto 0.64 ("Ship proto 0.64" in `MOON_COMPACT.md`).
- `node_depman_tool`: #5's `package.json` parse is a patch, after `NODE.md` #1 adds the shared
  struct. The toolchain re-exports the tool, so it also needs a toolchain release.

## Handed to other plans

- **JAVASCRIPT:** projects without their own `package.json` under a single-package root (no
  `workspaces`, so `members: None`) get no `InstallDependencies`. **Live:** `moon run docs:run`
  failed with "Cannot find module 'ms'" on a fresh clone.
- **JAVASCRIPT:** the task hasher parses and merges every lockfile in `lock_file_names` that exists
  at the root, and later files win per name (moon `task_hashing.rs:269-285`, `:342`). The install
  fingerprint hashes only the first that exists, with `bun.lock` and `deno.lock` ahead of
  `package-lock.json` (`tier1.rs:56-71`). A stale lockfile from another package manager therefore
  overrides npm's. Found from source.
- **JAVASCRIPT:** `create_manifest_dependency` skips `npm:` aliases (`tier2.rs:719-725`), so aliased
  dependencies never reach task hashes, though the lockfile keys them by alias.
- **JAVASCRIPT:** edits to `overrides` or `resolutions` don't change the install fingerprint, since
  `parse_manifest` only reports dependency fields. Found from source.
- **JAVASCRIPT:** `initialize_toolchain`'s `config_url` (`tier1.rs:106`) is
  `https://moonrepo.dev/docs/guides/javascript`, which returns 404.
- **JAVASCRIPT:** the other `|_, _|` mocks of `load_toolchain_config` in `tier2_test.rs` and
  `tier2_env_test.rs` don't assert the toolchain ID.
- **PNPM, YARN, NUB, BUN:** check whether focused production installs in `docker prune` (`--filter`,
  `workspaces focus`) drop the root package's dependencies, or fail when a root-level project's name
  is passed (npm #2, #6).
- **PNPM, YARN, NUB, BUN:** `dedupe_command` gets no `installArgs` (npm #3). Check whether their
  dedupe needs them.
- **PNPM:** patch files from `patchedDependencies` aren't in the configs skeleton (npm #4).
  **BUN:** #4 covers `bun patch`'s `patchedDependencies`; confirm Bun fails the same way without the
  file.
- **YARN, BUN:** check whether workspace dependencies declared by version range get no project edge
  (npm #7), since Yarn's transparent workspaces link them too.
