# Bun toolchain fix plan

Fixes from the Bun toolchain audit (2026-11-17, against moon v2.6.5 source, live-checked with moon
2.6.4, proto 0.64.0, and Bun 1.2.20, 1.4.0 and 1.4.4, the latest release; 1.4.1 to 1.4.3 were used
to bisect #5 and #6). Covers:

- `toolchains/bun` (`bun_toolchain` 1.0.5, ID `bun`): tier 1 metadata, config, Docker metadata and
  `executeArgs`.
- `tools/bun` (`bun_tool` 0.17.0), which the toolchain re-exports as its tier 3 API. It was audited
  on its own on 2026-11-06, so it's only covered here where the toolchain depends on it.
- The Bun parts of `toolchains/javascript` (`javascript_toolchain` 1.3.2, ID `javascript`), which
  installs dependencies, parses `bun.lock` and `bun.lockb`, and builds the project graph for Bun.

Item numbers (#1–#12) match the audit findings.

## Testing

Build the WASM first. The test sandbox loads the prebuilt `.wasm`, so rebuild after every source
change; `moon run :build` can replay a stale cached build. Build tools and toolchains in separate
cargo runs.

```bash
cargo build -p bun_tool --target wasm32-wasip1 --release
cargo build -p bun_toolchain -p javascript_toolchain --target wasm32-wasip1 --release
```

```bash
cargo nextest run -p bun_tool -p bun_toolchain -p javascript_toolchain --no-default-features
```

In agent sessions, unset `AI_AGENT` and `CLAUDECODE` first, or wrapper banners can break JSON tests.

The Bun install mocks of `load_toolchain_config` used `|_, _|`, which ignores the requested ID. That's
how Python #1 slipped through. They now assert the ID is `bun` (JavaScript pass), and new tests
should too.

Findings marked **Live** were reproduced with `~/.proto/tools/moon/2.6.4/moon` in a scratch git repo,
which loads the released `bun_toolchain` 1.0.5 and `javascript_toolchain` 1.3.2 by default (moon
`crates/config-loader/src/toolchains_config_ext.rs:16`, `:19`). `bun_toolchain` on `master` only
differs from 1.0.5 in its changelog, and the JavaScript toolchain's install logic is unchanged since
1.3.2; only its lockfile parsing changed. The repo was a Bun workspace with `packages/*` members:

- `a` depends on `b` (`workspace:*`) and `ms`, with `kleur` as a dev dependency.
- `b` depends on `picocolors`, with `is-odd` as a dev dependency.
- `c` depends on `left-pad`, and the root has `typescript` as a dev dependency.

```yaml title=".moon/toolchains.yml"
javascript:
  packageManager: "bun"
bun:
  version: "1.4.0"
```

To test a JavaScript toolchain fix, point `javascript` at the build with
`plugin: "file://<CARGO_TARGET_DIR>/wasm32-wasip1/release/javascript_toolchain.wasm"`. It embeds no
tool, so a `master` build loads in moon 2.6.4. A `bun_toolchain` build doesn't (see Release), so #1
was checked with a throwaway build whose `bun_tool` minimum was lowered to 0.62. To simulate Bun's
installer without touching your home directory, run moon with `HOME=<scratch>` and
`PROTO_HOME=~/.proto`, and copy a `bun` binary to `<scratch>/.bun/bin`. `moon docker setup` was
emulated by copying `.moon/docker/configs` to a fresh git repo.

**Checked, no change needed:**

- `vendor_dir_name: node_modules` exists after both linkers. Bun 1.4 writes `configVersion: 1` and
  uses the isolated linker for new workspaces, which keeps its store in the root
  `node_modules/.bun`. A third `moon run` skipped the install, and `bun dedupe` ran after the second.
- Production installs: `bun install --production` drops root and member dev dependencies under the
  isolated linker, and fails on a stale lockfile, as it implies `--frozen-lockfile` (1.2.20, 1.4.0
  and 1.4.4). **Live:** after `moon docker setup`, `moon docker prune` for `a` left only `ms` and
  `picocolors` in the store.
- `--filter <name>` also installs the workspace dependencies' production dependencies on 1.2.20 and
  1.4.x. `<name>...` installs nothing on 1.2.20, so the `>=1.4.0` gate (`tier2.rs:450-458`) is
  right. `bun ci` exists in 1.2.20, and `bun dedupe` in 1.4.0.
- Workspaces in the array and object forms, and `catalog` and `catalogs` at the top level or in
  `workspaces`: **live**, `"ms": "catalog:"` and `"kleur": "catalog:ui"` were hashed with their
  locked integrities.
- Lockfile names and order (`bun.lock` first): migrating with `--save-text-lockfile` deletes
  `bun.lockb`, and Bun 1.4.4 keeps writing `bun.lockb` when it's the only lockfile.
- `bun.lock` shapes, against the serializer at bun-v1.4.4 (`src/install/lockfile/bun.lock.rs`): each
  package shape the writer emits (`:718-905`) maps to a `BunLockPackage` variant: npm, git and
  github with and without integrity, local and remote tarballs, `file:`, `link:`, `root:` and
  `workspace:`. The top-level `configVersion`, `trustedDependencies`, `patchedDependencies`,
  `overrides` (including v3's scoped rules), `catalog`, `catalogs` and `packageExtensions` (`:370-625`)
  are ignored. `master` hashed a lockfile with all of these, written by 1.4.4. Released 1.3.2 failed
  every task on its local tarball (`./vendor/mri-1.2.0.tgz`: "expected semver"), which `master`'s
  unreleased lockfile fixes already cover.
- ADR-0006: the `lockfiles` module is gated with `any(feature = "wasm", test)`, and its 9 Bun unit
  tests run with `--no-default-features`.
- Version detection: `moon toolchain add bun` took `1.4.0` from `.bun-version` and from
  `packageManager`, `>=1.2.0` from `engines.bun`, and `^1.3.0` from `devEngines.runtime`, through the
  tool's `parse_version_file`, so moon and proto agree.
- `bunx`: moon appends `~/.proto/bin`, where proto links `bunx`, so `bunx --version` ran with only
  `/usr/bin:/bin` on PATH.
- `bunfig.toml` belongs in `config_file_globs`: it was copied into the configs skeleton (but see
  `MOON_COMPACT.md` for the install fingerprint).
- `oven/bun` has `1.4`, `1.2.0`, `canary` and `latest` tags, so `define_docker_metadata`'s
  `to_partial_string` works for versions and aliases.

## Decisions needed

| Decision                                                                  | Outcome                                                     |
| ------------------------------------------------------------------------- | ----------------------------------------------------------- |
| D1 (#8): when `bun bun.lockb` can't run, fail every task hash or skip it? | **Decided:** skip with a warning                            |
| D2 (#9): should `bun` require the `javascript` toolchain?                 | **Decided:** no, `moon toolchain add bun` adds it (in moon) |

Decided and implemented on 2026-11-18, on the `js-audit` branch with `JAVASCRIPT.md`. The pass also
did `PNPM.md` #1, which fixes #4.

## Phase 1: clear-cut fixes

### #1 A Bun in `~/.bun/bin` replaces the configured version

- [x] Add `activate_environment` to `tools/bun`, returning the tool directory
- [x] Tests
- [x] Changelog (tool and toolchain)

**Problem:** `locate_executables` lists `$BUN_INSTALL_BIN`, `$BUN_INSTALL/bin`,
`$XDG_CACHE_HOME/.bun/bin` and `$HOME/.bun/bin` as globals directories
(`tools/bun/src/proto.rs:351-356`; the tool embedded in 1.0.5 only has `$HOME/.bun/bin`). Bun's
install script puts `bun` itself in `$BUN_INSTALL/bin`, which defaults to `~/.bun/bin`, and so does
`oven-sh/setup-bun` (`src/action.ts:56`). proto returns every lookup directory whose variables are
set, and moon puts a toolchain's globals directories ahead of its own executable directory (moon
`crates/toolchain-plugin/src/toolchain_plugin.rs:232-237`). So with `bun.version` set, a Bun from
the installer or setup-bun runs moon's own `bun install` and every task.

**Live** (`bun.version: '1.4.0'`, Bun 1.2.20 at `$HOME/.bun/bin/bun`): `moon run` printed
`bun install v1.2.20`. It rewrote `bun.lock` from `lockfileVersion: 2` with `configVersion: 1` to
`lockfileVersion: 1` with no `configVersion`, which newer Bun reads as the hoisted linker, and a
`bun --version` task printed `1.2.20`. This is Go #6's shape, but `~/.bun/bin` is also where
`bun add -g` links bins, so it can't be dropped from the lookup.

**Change:** add `activate_environment`, returning the real `tool_dir` in `paths`, as `tools/go`
returns `GOROOT`. moon applies activation paths before the toolchain's executable paths
(`crates/process-augment/src/augmented_command.rs:29-34`, `:395-420`), and only when `version` is
set and the tool is set up (`toolchain_plugin.rs:253-257`). proto also applies it in `proto run`
and `proto activate`, where it's harmless.

**Live** with a throwaway build: PATH became the `node_modules/.bin` directories,
`~/.proto/tools/bun/1.4.0`, then `~/.bun/bin`. The install printed `bun install v1.4.0`, the
lockfile was untouched, the task printed `1.4.0`, and `~/.bun/bin` stayed on PATH for global bins.

A moon change that orders each toolchain's executable directory first would also work (see
`MOON_COMPACT.md`).

**Tests** (`tools/bun/tests/activate_test.rs`): `activate_environment` returns the tool directory.
It's a real path, as proto adds the paths to `PATH` as they are, while moon converts virtual ones.

### #2 Frozen installs in CI without a version, or with `bun.lockb`

- [x] Add `--frozen-lockfile` in CI whenever a Bun lockfile exists
- [x] Tests
- [x] Changelog

**Problem:** `install_dependencies` only uses `bun ci` when `env.ci` is set, the install isn't for
production, `bun.lock` exists, and `bun.version` satisfies `>=1.2.20`
(`toolchains/javascript/src/tier2.rs:432-441`). `version_satisfies` (`config.rs:153-175`) is false
when `bun.version` is unset or an alias like `latest`, and reads `1.2` as 1.2.0. `bun.lockb` never
matches. CI then runs a plain `bun install`, and Bun doesn't freeze installs in CI: with `CI=true` and
a dependency added to a member's `package.json`, 1.2.20, 1.4.0 and 1.4.4 each rewrote `bun.lock`,
while `bun ci` and `bun install --production` failed with "lockfile had changes, but lockfile is
frozen". **Live** (`bun: {}`, `CI=true`, stale lockfile): moon ran `bun install`, which printed
"Saved lockfile", and `git status` showed `M bun.lock`.

**Change:** when `env.ci` is set, the install isn't for production, and `bun.lock` or `bun.lockb`
exists, run `bun install --frozen-lockfile`. Every Bun 1.x has the flag, and it works with
`bun.lockb` (confirmed on 1.2.20 and 1.4.4). `bun ci` is only an alias for it, so the version check
can go. Production installs are unchanged, since `--production` already implies it. A second
`--frozen-lockfile` from `installArgs` is accepted.

**Tests:** `freezes_the_lockfile_in_ci` (an unset, old and alias `version`, and `bun.lockb`) and
`production_implies_frozen_in_ci` replace `switches_to_ci_in_ci` and
`skips_ci_on_older_versions_and_production`.

**Changelog:** a stale lockfile now fails CI installs when `bun.version` is unset or below 1.2.20.

### #3 `bun bun.lockb` runs whichever `bun` moon itself finds

- [x] Pass `PROTO_BUN_VERSION` from the `bun` toolchain's `version`
- [x] Re-enable the `bun.lockb` test

**Problem:** `parse_bun_lockb` runs `bun bun.lockb` and parses the Yarn-format output
(`lockfiles/bun.rs:178-193`). A command run from inside a plugin function gets moon's own process
environment, not the toolchain's: no `PROTO_BUN_VERSION`, and no tool directory on PATH. moon calls
`parse_lock` for every task hash in the default `accuracy` mode
(`crates/task-hasher/src/task_hashing.rs:269-285`). **Live** (`bun.version: '1.4.0'`, a workspace
with `bun.lockb`):

- proto's shim resolved Bun 1.2.20 from the global `.prototools`, which exited 1 with "error parsing
  lockfile: Outdated lockfile version" on the file 1.4.0 had saved. Released 1.3.2 parsed the empty
  stdout and failed every task with "Error parsing yarn.lock file". `master` fails every task with
  "Failed to parse /workspace/bun.lockb: Failed to read bun.lockb with bun: error parsing lockfile:
  Outdated lockfile version".
- With only `/usr/bin:/bin` on PATH, as in CI where moon installs Bun but proto's shims aren't on
  PATH: "Command or script bun does not exist. Unable to execute from plugin."
- With `PROTO_BUN_VERSION=1.4.0` set for moon, the task ran, and its hash had `kleur` and `ms` with
  their integrities, so yarn-lock-parser 0.14 parses that output.

**Change:** in `parse_bun_lockb`, call `load_toolchain_config("bun")`, and when it returns a
`version`, set `PROTO_BUN_VERSION` on the command. Ignore an error from the call, since an npm or
pnpm repo with a stray `bun.lockb` has no `bun` toolchain. This fixes the proto shim case. A `bun`
that comes first on moon's own PATH, or no `bun` at all, still fails; #8 covers those.

**Tests:** `parses_bun_classic` is re-enabled, and skipped when `bun` isn't on PATH, as CI runners
may not have it. The test sandbox runs real commands, so the `PROTO_BUN_VERSION` env var itself isn't
covered.

### #4 Docker scaffold doesn't copy `bun patch` files

- [x] Fixed by `PNPM.md` #1, done in this pass

**Problem:** `bun patch --commit` writes `patches/<name>@<version>.patch` and records it in the root
`package.json`'s `patchedDependencies` and in `bun.lock`. `bun install` reads the file.
`define_docker_metadata` only adds `*.{js,cjs,mjs}` (`toolchains/bun/src/tier1.rs:59-62`), and moon
adds the config file globs, lockfiles and manifests (`toolchain_plugin.rs:313-315`). **Live:**
`moon docker scaffold a` left `patches/` out of the configs skeleton, and `moon docker setup` there
failed with `error: Couldn't find patch file: 'patches/ms@2.1.3.patch'`. With `patches/**/*` in
`docker.scaffold.configsPhaseGlobs`, setup passed and the patch was applied.

**Change:** `PNPM.md` #1 adds the same `patches/**/*` glob to the JavaScript toolchain, for every
package manager. That covers Bun, and it ships sooner, since `bun_toolchain` is blocked on proto
0.64 and `javascript_toolchain` isn't (see "Release"). Not covered: patches in a custom
`--patches-dir`, which users can add to `configsPhaseGlobs`.

### #5 `bun dedupe` fails on Bun 1.4.0 with `bun.lockb`

- [x] Gate dedupe on `>=1.4.1`
- [x] Tests

**Problem:** the dedupe command is returned for `>=1.4.0` (`tier2.rs:598-607`). moon runs it after
every install outside CI except the first, and fails the run when it fails
(`crates/actions/src/actions/install_dependencies.rs:224-251`). Bun 1.4.0's `bun dedupe` can't read
`bun.lockb`: "error: bun.lock does not match package.json, nothing to dedupe", exit 1. **Live**
(`bun.version: '1.4.0'`, a `bun.lockb` workspace): the second `moon run` failed with "Process bun
failed: exit code 1" right after the install. 1.4.1 to 1.4.4 dedupe the same workspace, and 1.4.0
dedupes a `bun.lock` one.

**Change:** require `>=1.4.1`.

**Tests:** `focused_commands_v140` expects no dedupe, and `dedupes_on_v141` does.

### #6 Dependency version formats for Bun

- [x] Map `link` to `file`
- [x] Keep `workspace-caret` and `workspace-tilde`
- [x] Unit tests in `config.rs`
- [x] Changelog

**Problem:** `get_supported_for` (`toolchains/javascript/src/config.rs:77-83`) turns `workspace:^`
and `workspace:~` into `workspace:*` for Bun, and passes `link:` through. Confirmed by installing the
workspace with each spec for `a`'s dependency on `b`:

- `workspace:^` and `workspace:~` install and link `b` on 1.2.20 and 1.4.4, so the configured format
  is dropped for no reason, and `bun publish` writes an exact version instead of the range.
- `link:../b` fails on every Bun before 1.4.4 with `Package "b" is not linked` (1.2.20 and 1.4.0 to
  1.4.3), as `link:` named a package registered with `bun link` until then. `file:../b` links the
  directory on every version.

**Change:** for Bun, map `Link` to `File`, and pass the other formats through.

**Tests:** unit tests for each format in `config.rs`, which isn't behind the `wasm` feature.

**Changelog:** synced `package.json` specs change once for `workspace-caret`, `workspace-tilde` and
`link`.

### #7 Small fixes

- [x] `parse_bun_lock` passes npm and git or github integrities through as they are
      (`lockfiles/bun.rs:125-138`). Bun writes `""` when the registry sent no integrity or shasum,
      as `Integrity` displays nothing for an unknown tag (bun-v1.4.4 `src/install/integrity.rs:245-249`,
      written at `bun.lock.rs:849`). moon records a hash in preference to the version
      (`task_hashing.rs:457-461`), so that package's upgrades never change a task hash. Check every
      integrity with `is_integrity_hash`, as `Dependency3` does. Not reproduced, as it needs such a
      registry.
- [x] `tools/bun/src/proto.rs:63`: `parse_version_file` parses the whole `package.json` as
      `PackageJson`, so an invalid `version` or dependency specifier silently drops the
      `devEngines.runtime`, `engines.bun` and `packageManager` pins. Switch to the version-fields
      struct that `NODE.md` #1 adds to `lang-javascript-common` (handed over by `NODE.md`, which
      reproduced it for `node_tool`).
      Done in the JavaScript pass: `nodejs_package_json` now skips fields it can't parse.
- [x] Remove the unused `exec_command` host function (`tools/bun/src/proto.rs:15-18`), left over since
      0.17.0 took the baseline build from proto's CPU level.

## Phase 2: after decisions

### #8 Don't fail every task hash when `bun bun.lockb` can't run (D1)

- [x] Warn and return no locked dependencies
- [x] Tests

**Problem:** after #3, `parse_lock` still fails every task when the `bun` first on moon's PATH can't
read the file, when there's no `bun` on PATH, or for a stray `bun.lockb` in a repo that doesn't use
Bun. moon parses every lockfile in the JavaScript toolchain's `lock_file_names` that exists at the
root (`task_hashing.rs:269-285`, `toolchains/javascript/src/tier1.rs:56-71`), whatever the package
manager. `master` returns an error on a non-zero exit (`lockfiles/bun.rs:185-190`), and the host's
error for a missing command fails the call too.

**Recommended:** handle both by logging a warning with `host_log!`, naming the command and its
trimmed stderr, and returning no dependencies. ADR-0007.3 allows this for optional commands, and the
lockfile only refines the hash: without it, moon records each manifest requirement instead
(`task_hashing.rs:474-480`). `parse_lock` runs once per task, so set a var after the first warning
and log later ones with `debug!`.

**Alternative:** keep `master`'s error. It's loud, but blocks every task until the user fixes PATH.

**Tests:** `skips_a_bun_lockb_that_bun_cant_read` returns no dependencies for a file that isn't a
lockfile, which `bun` exits 1 on ("InvalidLockfile"), or that the host can't run `bun` for.

### #9 `moon toolchain add bun` leaves out `javascript` (D2)

- [x] Decide where to fix it: in moon (`MOON_COMPACT.md`), nothing in this repo

**Problem:** moon's docs say the `bun` toolchain requires `javascript` (`website/docs/config/toolchain.mdx`,
`## bun`), but nothing enforces it: the toolchain has no `define_requirements`, and
`moon toolchain add bun` only writes `bun:`. **Live:** with only `bun:` configured, tasks ran with
Bun, but nothing was installed and `moon project a` showed no dependencies, with no warning. The
Node and Deno toolchains have the same gap.

**Recommended:** fix it in moon: `moon toolchain add bun` also adds `javascript`, which detects
`packageManager: bun` from `bun.lock`.

**Alternative:** `define_requirements` with `requires: ["javascript"]` and `for_setup_toolchain`, so
moon errors with "requires the toolchain javascript, but it has not been configured"
(`crates/action-graph/src/action_graph_builder.rs:1544-1568`). That breaks repos that use Bun only
to run scripts, without installs. The JavaScript toolchain already requires `bun`, which moon
allows, as it skips requirements already in the cycle.

## Phase 3: lower priority

### #10 Docker image for lower bounds

- [x] Use `latest` for `>`, `>=`, `<` and `<=` requirements

`define_docker_metadata` uses `to_partial_string` (`tier1.rs:51-58`), which keeps the version for
every operator except `!=` (version_spec 0.11.8 `unresolved_spec.rs:125-147`). `moon toolchain add`
writes `engines.bun`'s `>=1.2.0` as is (confirmed above), so the default image is `oven/bun:1.2.0`,
the oldest version allowed. `<2` would give `oven/bun:2`, which doesn't exist.

### #11 Test gaps

- [x] Assert the requested ID in the Bun install mocks (see Testing), done in the JavaScript pass
- [x] Unit tests in `lockfiles/bun.rs` for the shapes the fixture lacks: remote and local tarballs,
      `file:`, `root:`, git with and without integrity, and an aliased key like
      `"my-ms": ["ms@2.1.2", …]`

### #12 Detection names

- [x] `config_file_globs` (`toolchains/bun/src/tier1.rs:17`): add `.bun-version` and `.bumrc`, which
      the tool reads (`tools/bun/src/proto.rs:46-50`). This also copies them into the configs
      skeleton.

## Not planned

- **Syncing `packageManager: bun@<version>`:** `syncPackageManagerField` skips Bun
  (`tier2.rs:844-846`). Bun ignores the field, and Corepack doesn't manage Bun.
- **`bun dedupe` with `frozenLockfile = true` in `bunfig.toml`:** it fails when there's a duplicate
  to remove ("1 duplicate version can be removed, but --frozen-lockfile was passed", 1.4.4). Repos
  that freeze locally can set `dedupeOnLockfileChange: false`.
- **Lifecycle scripts in members:** a member's `postinstall: bun scripts/setup.js` fails
  `docker setup` (**live:** `Module not found "scripts/setup.js"`), since only `*.{js,cjs,mjs}` at
  each project root is scaffolded. The Node toolchain has the same glob, and `configsPhaseGlobs`
  covers it. `NODE.md` #5 (D1) proposes copying the files that lifecycle scripts run; if that's
  accepted, put it in the JavaScript toolchain so Bun repos get it too.
- **Globals on PATH without `version`:** moon only adds globals directories when `version` is set
  (Go #5). Bun's installer and setup-bun put `~/.bun/bin` on PATH themselves, and proto's shim adds
  it when it runs `bun`.

## Outside this repo

moon changes are tracked in [`MOON_COMPACT.md`](MOON_COMPACT.md). This audit added config files to
"Fingerprint everything that changes an install", "Add `javascript` with `moon toolchain add bun`,
`deno` or `node`" (#9), "Run host function commands with the toolchain's environment" (#3, #8), and
"Put a toolchain's executable directory before its globals directories" (optional, instead of #1).
"Ship proto 0.64" blocks every `bun_toolchain` release.

## Release

**Blocked for `bun_toolchain`:** it embeds `bun_tool` 0.17.0, which sets `minimum_proto_version` to
0.64 (`tools/bun/src/proto.rs:29`). **Live:** moon 2.6.4 refuses a `master` build: "Unable to use the
Bun plugin with identifier bun, as it requires a minimum proto version of 0.64.0-alpha.0, but found
0.62.2 instead". So #1 and #12 wait for moon to ship proto 0.64 (`MOON_COMPACT.md`).

`javascript_toolchain` embeds no tool, and its `master` build loads in moon 2.6.4 (live), so it can
ship now. moon's default locators then need a bump (`toolchains_config_ext.rs:16`, `:19`) for users
to get either release.

- `javascript_toolchain`: #2, #3, #4, #5, #6, #7's integrity check and #8, in 1.4.0 with
  `JAVASCRIPT.md`, with callouts for CI installs that now fail on stale lockfiles (#2) and synced
  specs (#6). It also ships the
  unreleased lockfile fixes on `master`, which fix the live 1.3.2 failure on a local tarball's
  version.
- `bun_tool`: #1 and #7's `package.json` parse and cleanup are a patch (0.17.1), which proto 0.64
  users get right away.
- `bun_toolchain`: #10, #12, plus the `bun_tool` re-export, as 1.0.6. #4 ships in `javascript_toolchain`
  with `PNPM.md` #1.

## Handed to other plans

- `JAVASCRIPT.md`: `locate_dependencies_root` puts a package outside the root's `workspaces` globs
  under the root lockfile's workspace, so moon never installs it (**live**, Bun 1.4.0: `tools/x` got
  no `InstallDependencies`, while `bun install` there makes its own `bun.lock`).
- `JAVASCRIPT.md`: `prune_docker` only deletes the root `node_modules`. With isolated layouts (Bun's
  default for new workspaces, and pnpm), members' `node_modules` keep dangling symlinks to the
  pruned dev dependencies (**live** after `moon docker prune`).
- `JAVASCRIPT.md`: `extend_project_graph` doesn't link a workspace package required by a semver
  range like `"b": "^1.0.0"`, which Bun resolves to the workspace package (**live**, 1.2.20 and
  1.4.4), as npm and Yarn do.
- `JAVASCRIPT.md`: `parse_lock` runs for every JavaScript lockfile name that exists, whatever
  `packageManager` is, so a stray `bun.lockb` runs `bun` on every task hash (`BUN.md` #3, #8).
- `JAVASCRIPT.md`: npm aliases (`"my-ms": "npm:ms@2.1.2"`) are dropped by
  `create_manifest_dependency`, and lockfile parsers record them under the real name, so an
  aliased package's upgrades never change a task hash.
- `JAVASCRIPT.md`: lockfile parse errors name the virtual path, like
  `Failed to parse /workspace/bun.lockb`.
- `DENO.md`: `tools/deno` lists `$HOME/.deno/bin`, where Deno's installer puts `deno`, as a globals
  directory, which moon puts ahead of the configured Deno, as in `BUN.md` #1.
- `PNPM.md` and `YARN.md`: check whether a globals lookup directory can hold the manager itself,
  like `PNPM_HOME` for pnpm's standalone installer, as in `BUN.md` #1.
- `PNPM.md`: check that `docker scaffold` copies `patches/` for `pnpm patch`, which Bun's is missing
  (`BUN.md` #4).
