# Nub toolchain fix plan

Fixes from the Nub toolchain audit (2026-11-17, against moon v2.6.5 source, live-checked with moon
2.6.4, proto 0.64.0 and nub 0.11.1, the current release, plus nub 0.7.4 where versions matter).
Covers everything Nub-specific in:

- `toolchains/node-depman` (`node_depman_toolchain` 1.1.1), enabled for Nub as `unstable_nub`. It
  also serves `npm`, `pnpm` and `yarn`, picking the manager from the plugin ID, and only registers
  metadata, config and requirements for Nub.
- The `JavaScriptPackageManager::Nub` arms of `toolchains/javascript` (`javascript_toolchain` 1.3.2),
  which install dependencies, parse lockfiles, build the project graph and infer tasks for Nub.
- `tools/node-depman` (`node_depman_tool` 0.21.0), which the node-depman toolchain re-exports as its
  tier 3 API. It was audited on 2026-11-06, so it's only covered where the toolchain depends on it.

Item numbers (#1–#10) match the audit findings.

## Testing

Build the WASM first. The test sandbox loads the prebuilt `.wasm`, so rebuild after every source
change; `moon run :build` can replay a stale cached build. Build the tool in its own cargo run.

```bash
cargo build -p javascript_toolchain -p node_depman_toolchain --target wasm32-wasip1 --release
cargo build -p node_depman_tool --target wasm32-wasip1 --release
```

```bash
cargo nextest run -p javascript_toolchain -p node_depman_toolchain --no-default-features
```

In agent sessions, unset `AI_AGENT` and `CLAUDECODE` first, or wrapper banners can break JSON tests.

The JavaScript tests mock `load_toolchain_config` with `|_, _|`, which ignores the requested ID, as
Python's did, and nothing tests `define_requirements`, so #1's ID was never checked. New tests for
requirements and config loading should assert the ID.

Findings marked **live** were reproduced with `~/.proto/tools/moon/2.6.4/moon` in scratch git repos,
against the released `node_depman_toolchain` 1.1.1 and `javascript_toolchain` 1.3.2. Their Nub arms
match `master`: since the tags, `toolchains/javascript` only changed its lenient parsing, and
`toolchains/node-depman` only its changelog and manifest. nub ran from an isolated `PROTO_HOME`.

`unstable_nub: {}` doesn't load in any moon release (see `MOON_COMPACT.md`), so every check needs
the plugin's URL:

```yaml title=".moon/toolchains.yml"
javascript:
  packageManager: "nub"
unstable_nub:
  plugin: "https://github.com/moonrepo/plugins/releases/download/node_depman_toolchain-v1.1.1/node_depman_toolchain.wasm"
  version: "0.11.1"
```

The fixture is one workspace: `packages/a` depends on `is-odd` (and `is-number` as a dev
dependency), and `packages/b` depends on `a` through `workspace:*`. It was checked as a Nub-owned
repo, and with lockfiles written by pnpm 10.33.4 (members and a catalog in `pnpm-workspace.yaml`),
npm 11.13.0, Yarn 4.18.1 (`nodeLinker: node-modules`) and Bun 1.4.0 (catalog in `package.json`).
Commit a `.gitignore` for `node_modules`, or moon warns about hashing the members' symlinks.

The JavaScript toolchain embeds no tool, so a `master` build loads in moon 2.6.x. Point
`javascript.plugin` at `file://<CARGO_TARGET_DIR>/wasm32-wasip1/release/javascript_toolchain.wasm` to
check #1–#8. That's how #1's fix was checked.

**Checked, no change needed:**

- `nub.lock` is still byte-identical to `pnpm-lock.yaml` v9 in nub 0.11.1: `nub pm use nub` renamed
  pnpm's lockfile unchanged, and moved `pnpm-workspace.yaml`'s members and catalog into
  `package.json` `workspaces`. Live, task hashes held the locked integrity of `is-odd` and
  `is-number` from a fresh `nub.lock`, with the `catalog:` range resolved from `package.json`.
- Workspace members from `package.json` (array and object forms), `workspace:` edges and the root
  install: live, `moon project b` listed `a`, and members collapsed into one root install.
- `vendor_dir_name`: `node_modules` exists after every install, even with no dependencies (it holds
  `.store`, `.modules.yaml` and `.nub-engine`).
- Production installs: after a full install, `nub install --prod` removed `is-number` from
  `packages/a`, and `--filter-prod 'b...'` installed `b` and `a` only. Live, `moon docker scaffold`
  copied `nub.lock` and every `package.json`, and `docker setup` and `docker prune` (running
  `nub install --prod --filter-prod b...`) succeeded in the configs skeleton.
- Frozen in CI: with `CI` set, `nub install`, `nub install --prod` and `nub ci` all exit 16 on a
  stale lockfile ("lockfile is out of date with package.json").
- Compat mode: with npm, pnpm and Bun lockfiles, `nub install` and `nub dedupe` left them unchanged,
  and `nub ci` worked with every lockfile, including `yarn.lock`.
- No `node` requirement: nub provisions Node for dependency build scripts itself, as moon's docs say.
- `.prototools` versions reach `unstable_nub`, and tasks get `PROTO_NUB_VERSION` (live).
- `config_file_globs`: `nub.jsonc` is Nub's only config file name.
- ADRs: the Nub arms parse leniently (ADR-0008.5) and only unwrap literals (ADR-0005.1).

## Decisions needed

| Decision                                                             | Recommendation                                                         |
| -------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| D1 (#6): run Nub's inferred tasks on moon's `node` when it's enabled? | Yes: list `node` in their toolchains, which moon drops when it's unset |
| D2 (#7): sync `file:` instead of `workspace:` for Nub in npm repos?  | Yes, when the root has `package-lock.json` or `npm-shrinkwrap.json`    |

**Shared with pnpm:** Nub reads `nub.lock` with `parse_pnpm_lock_yaml` and its workspace with
`PnpmWorkspace`, so `PNPM.md` #4–#6 and #11 (patch hashes, the `default` catalog, pnpm 8 lockfile
keys and v13 integrities) fix Nub too (handed over by `PNPM.md`). `hashes_pnpm_patches` has a
`nub.lock` case. The others don't apply: nub declines v6 lockfiles, v13 is pnpm 12's, and repos Nub
owns keep catalogs in `package.json`.

**Not covered:** `package.json` catalogs don't map `catalogs.default` to the default catalog, as
`pnpm-workspace.yaml`'s now do. It's unchecked whether nub treats it as the default, and Bun reads
the same field.

**Found while implementing:** since `PNPM.md` #12, `syncPackageManagerField` updates the version in
`devEngines.packageManager` when it names the package manager. In repos Nub owns, nub writes
`"^0.11.1"` there, so moon now replaces it with the exact version, like `nub pm use nub@<version>`,
and only updates `packageManager` when it exists. Not checked live.

## Phase 1: clear-cut fixes

### #1 Require `unstable_nub`, the ID moon configures

- [x] Add a toolchain ID helper and use it everywhere (`get_toolchain_id`)
- [x] Tests that assert the ID: `requires_the_package_manager_toolchains`, and the Nub mocks in
      `tier2_test.rs` and `tier2_env_test.rs`

**Problem:** moon only maps `unstable_nub` to `node_depman_toolchain` (moon
`crates/config-loader/src/toolchains_config_ext.rs:25`), and its docs enable Nub as `unstable_nub`.
The JavaScript toolchain calls it `nub`: `define_requirements` pushes `format!("{package_manager}")`
(`toolchains/javascript/src/tier2.rs:216`), the config loads pass the same name (`:333`, `:421-422`,
`:850-852`), and `get_runtime_toolchain` returns `nub` (`config.rs:26`).

moon finds an unstable config from a stable ID (moon `crates/config/src/toolchains_config.rs:117`),
so the config loads work. But the action graph builds the requirement's spec under the ID it was
given (moon `crates/action-graph/src/action_graph_builder.rs:278`, `:1544`), and the registry loads
the plugin again under that ID (moon `crates/plugin/src/registry_loader.rs:16`).

**Live:** every run had both `SetupToolchain(nub:0.11.1)` and `SetupToolchain(unstable_nub:0.11.1)`.
They run in parallel, and both regenerate proto's shims when they're outdated. On the second run
after a fresh nub install, `moon run` failed 4 times out of 4 with `proto::store::shim::create_failed`
("Failed to create shim …/shims/nubx. No such file or directory"). With a local build that requires
`unstable_nub`, the graph had one `SetupToolchain(unstable_nub)`, and 6 runs over two fresh installs
all passed.

**Change:** add `JavaScriptPackageManager::get_toolchain_id()`, returning `unstable_nub` for Nub and
the manager's name otherwise. Use it in `define_requirements`, the three config loads, and the
inferred task toolchains (`infer_tasks.rs:145`, `:150`). moon already resolves `nub` to
`unstable_nub` for task toolchains (moon `crates/toolchain/src/lib.rs:89`), so tasks don't change,
but one ID is easier to follow. If moon stabilizes Nub as `nub`, rename both together.

**Tests:** there's no `define_requirements` test in `toolchains/javascript/tests`, so add one per
package manager. Make the Nub install tests' `load_toolchain_config` mocks assert the ID
(`tier2_test.rs:1740`).

### #2 Don't rewrite `packageManager` in a repo another manager owns

- [x] Skip the sync for Nub in compat mode. `nub::detect_nub_identity` follows nub's order
      (`packageManager`, `devEngines.packageManager`, then the lockfile), and #5, #7 and #8 use it
      too.
- [x] Tests: `doesnt_set_field_for_nub_in_other_managers_repos`, and `sets_field_for_nub` with
      `nub.lock`
- [x] Changelog

**Problem:** `syncPackageManagerField`, which is on by default, writes
`"packageManager": "nub@<version>"` whenever `unstable_nub.version` is exact, including a version
inherited from `.prototools` (`tier2.rs:843-873`). It also replaces a `packageManager` that names
another manager.

nub decides which manager's repo it's in from `packageManager`, then `devEngines.packageManager`,
then the lockfile on disk ([nub docs](https://nubjs.com/docs/install#lockfile-compatibility)). In a
repo that another manager owns, nub installs in that manager's format and reads its config. The sync
switches the repo to Nub's own mode, which ignores `pnpm-workspace.yaml` (live) and, per nub's docs,
the `pnpm.*` fields, `.pnpmfile.cjs` and Bun's `trustedDependencies`.

**Live:** in the pnpm workspace, moon wrote `"packageManager": "nub@0.11.1"`. `nub install` then:

- printed "pnpm-workspace.yaml is not read under nub identity"
- rewrote `pnpm-lock.yaml` without the members' importers, the catalog or any package (46 lines
  removed)
- installed nothing into the members, so the task failed with `Cannot find module 'is-odd'`

`nub ci` with `CI` set did the same ("Already up to date"). In the npm and Bun workspaces the switch
left the lockfile unchanged and installs still worked, as both keep their members in `package.json`.

**Change:** for Nub, skip the sync when the root has a lockfile from `NUB_LOCK_NAMES` other than
`nub.lock`, or when `packageManager` or `devEngines.packageManager` names another manager. Repos
that Nub owns keep the sync. nub already writes `devEngines.packageManager` there (`"^0.11.1"` with
`"onFail": "ignore"`), and the exact `packageManager` pins it.

**Tests** (`tier2_env_test.rs`, next to `sets_field_for_nub`): no write with `pnpm-lock.yaml`, no
write over `"packageManager": "pnpm@10.0.0"`, and a write with `nub.lock`.

**Changelog:** say that moon no longer pins Nub in repos it installs in another manager's format.

### #3 Use `nub install --frozen-lockfile` in CI, not `nub ci`

- [x] Replace `nub ci`
- [x] Freeze production installs in CI too
- [x] Tests: `freezes_the_lockfile_in_ci`, `freezes_the_lockfile_with_other_pm_lock` and
      `doesnt_freeze_the_lockfile_against_install_args`

**Problem:** in CI, when any lockfile from `NUB_LOCK_NAMES` exists, the plugin runs `nub ci` and
appends `unstable_nub.installArgs` (`tier2.rs:514-528`, `:585-589`). `nub ci` accepts far fewer
flags than `nub install`: nub 0.11.1's `nub ci --help` has no `--prefer-offline`, `--offline`,
`--node-linker`, `--frozen-lockfile`, `--pnpmfile` or `--force`, and the ones tried
(`--prefer-offline`, `--node-linker=hoisted` and `--frozen-lockfile`) exit 2 with "unexpected
argument". **Live:** with `installArgs: ['--prefer-offline']`, installs passed locally. In CI, moon
ran `nub ci --prefer-offline`, which failed with `error: unexpected argument '--prefer-offline'
found`.

The only other thing `nub ci` does is delete `node_modules` first, which never matters here: in CI,
moon skips the install whenever `node_modules` exists (moon
`crates/actions/src/actions/install_dependencies.rs:126`).

Production installs use `nub install --prod`, and rely on nub freezing the lockfile in CI. nub only
checks for a `CI` variable (nubjs/nub `vendor/aube/crates/aube-util/src/env.rs:221`), while moon's
`env.ci` also detects CI providers that don't set it. Production installs on those aren't frozen.

**Change:** when `env.ci` is set and a lockfile from `NUB_LOCK_NAMES` exists, run
`nub install --frozen-lockfile`, adding `--prod` for production installs. Skip the flag when
`installArgs` already has a flag ending in `frozen-lockfile`, since nub rejects it with
`--no-frozen-lockfile` ("cannot be used with"). A repeated `--frozen-lockfile` is accepted, and so
is `--fix-lockfile` with it.

nub 0.7.4 and 0.11.1 both accept `--frozen-lockfile`, with Yarn and Bun lockfiles too, so there's no
version check. A stale lockfile fails as it does today.

**Tests:** in the `nub` module of `tier2_test.rs` (`:1661`), update the two `nub ci` cases, and add a
production install in CI (frozen) and `installArgs` with `--no-frozen-lockfile` (no flag).

### #4 Give production installs a project-local store

- [x] Set `npm_config_enable_global_virtual_store=false` for production
- [x] Test (`focused_commands`)

**Problem:** outside CI, nub links `node_modules/.store/*` into a store shared by the whole machine,
`~/.cache/nub/pm/store/v1`, in 0.7.4 and 0.11.1. nub's Docker docs say to install with `nub ci` in
multi-stage builds, because "a plain `nub install` links packages from a machine-global store the
final stage does not have" ([nub docs](https://nubjs.com/docs/deployment/docker)). `docker prune`
runs `nub install --prod`, and Docker builds don't usually set `CI`.

**Live:** after `moon docker setup` and `moon docker prune` in an emulated configs skeleton,
`node_modules/.modules.yaml` held `virtualStoreDir: /Users/…/.cache/nub/pm/store/v1`, and every
`.store` entry was an absolute symlink into it. moon's generated Dockerfile only copies
`/root/.proto` and `/app` into its start stage (moon
`crates/docker/templates/Dockerfile.tera:63-64`), so no dependency resolves there.

**Change:** when `input.production` is set, add `npm_config_enable_global_virtual_store=false` to
the install command's env. nub 0.7.4 and 0.11.1 both read it ("linker isolated
(npm_config_enable_global_virtual_store)"), and `.store` then holds real directories. The variable
outranks `.npmrc`, which is what production installs need.

**Not covered:** `docker setup` installs look like local ones, so an image that skips `docker prune`
keeps the links. MOON_COMPACT's "Tell `install_dependencies` when it runs in Docker" would cover it.
CI providers that moon detects but that don't set `CI` also get the shared store, which only matters
for caching `node_modules`.

**Tests:** the production install case asserts the env.

### #5 Don't run `nub dedupe` in Yarn repos

- [x] Skip the dedupe when Yarn owns the repo
- [x] Test (`skips_dedupe_in_yarn_repos`)

**Problem:** the plugin always returns `nub dedupe` as the dedupe command (`tier2.rs:618-624`). nub
won't write `yarn.lock`, and `nub dedupe` there exits 1: "refusing to modify yarn.lock … Run it with
yarn directly: yarn dedupe". Outside CI, moon runs the dedupe after every install except the first
(moon `install_dependencies.rs:224-231`). **Live:** in the Yarn workspace, after `yarn add` changed
`yarn.lock`, `moon run` ran `nub install` and then `nub dedupe`, which failed the
`InstallDependencies` action. With npm, pnpm and Bun lockfiles, `nub dedupe` exits 0.

**Change:** return no `dedupe_command` for Nub when the root has `yarn.lock` and no `nub.lock`.

**Tests:** a Nub install with `yarn.lock` has no dedupe command.

## Phase 2: after decisions

### #6 Run Nub's inferred tasks on moon's Node (D1)

- [x] Add `node` to Nub's inferred task toolchains (`infers_toolchains_for_each_package_manager`)
- [ ] Docs for hand-written tasks: moon's website, tracked in `MOON_COMPACT.md`

**Problem:** nub runs scripts on stock Node. It picks the version from `NODE_EXECUTABLE`,
`nub.jsonc`, `devEngines.runtime`, `.node-version`, `.nvmrc`, `.tool-versions` or `engines.node`, and
otherwise uses the `node` on PATH ([nub docs](https://nubjs.com/docs/node)). Nub's inferred tasks
only get `javascript` and `nub` (`infer_tasks.rs:141-146`), since `is_standalone` is true for Nub
(`config.rs:38-40`). So moon's `node` toolchain isn't on their PATH, and its version isn't set or
hashed for them.

**Live:** with `node.version: '22.22.3'`, the inferred `nub run nodever` task printed `v26.0.0`, while
a `node -e` task in the same project printed `v22.22.3`. `moon task` listed
`['javascript', 'unstable_nub']` for the first.

**Recommended:** list `node` for Nub's inferred tasks too, giving `javascript`, `unstable_nub` and
`node`. moon drops `node` when it isn't configured (moon `crates/toolchain/src/lib.rs:89`). Return
`node` from `get_runtime_toolchain` for Nub, and keep `is_standalone` for `define_requirements`,
since nub provisions Node itself when there's no `node` toolchain. nub prefers a `node` on PATH that
satisfies the repo's pin, so moon's Node only loses to a pin that disagrees with it.

**Alternative:** leave it, and document that nub's own pin files decide.

**Not covered:** hand-written tasks that run `nub` get `javascript` and `unstable_nub` from
`exe_names`, and need `toolchains: ['node']`. Document it in the `unstable_nub` docs.

**Tests:** inferred Nub tasks list the three toolchains.

### #7 `workspace:` ranges in npm repos (D2)

- [x] Sync npm's format for Nub when npm owns the repo, from the nearest lockfile's root
- [x] Test (`syncs_npm_format_for_nub_in_npm_repos`)

**Problem:** `get_supported_for` keeps every format for Nub (`config.rs:94`), so the default
`workspace:*` is synced into each `package.json`. nub accepts it in a repo with `package-lock.json`,
but writes it into the lockfile too, which npm can't read. **Live:** nub 0.11.1 installed with
`"a": "workspace:*"` and wrote it into `package-lock.json`. npm 11.13.0's `npm ci` and `npm install`
then failed with `EUNSUPPORTEDPROTOCOL`, `Unsupported URL Type "workspace:"`.

**Recommended:** in `sync_project_workspace_dependencies` (`tier1.rs:247-249`), use npm's row of
`get_supported_for` for Nub when the workspace root has `package-lock.json` or `npm-shrinkwrap.json`.
That only matters to teams that still run npm in the repo.

**Alternative:** leave it, since moon's configured manager is Nub.

## Phase 3: lower priority

### #8 Read workspace config the way Nub does

- [x] Read `pnpm-workspace.yaml` only in pnpm repos (`ignores_pnpm_workspace_when_nub_owns_the_repo`)

For Nub, `extract_workspace_members_and_catalogs` and `locate_dependencies_root` read
`pnpm-workspace.yaml` first (`tier2.rs:271-282`, `:342-344`). nub only reads it when pnpm owns the
repo. In a repo Nub owns, it prints "pnpm-workspace.yaml is not read under nub identity" and uses
`package.json` (live), so moon's members and catalogs can differ from nub's. Read it for Nub only
when `pnpm-lock.yaml` exists and `nub.lock` doesn't.

Nub now reads the config files of whichever manager owns the repo, so `.yarnrc.yml` catalogs are
read in Yarn repos too.

### #9 Detection names and docs links

- [x] `exe_names`: add `nubr`, which nub ships since 0.9.3 (`npm/nub/package.json` `bin`), in
      `toolchains/node-depman/src/tier1.rs:26` and `toolchains/javascript/src/tier1.rs:86-87`.
- [x] `NubToolchainConfig`'s docs link (`toolchains/node-depman/src/config.rs:21`) points at `#nub`.
      moon's heading is `unstable_nub`.

### #10 Nub's globals lookup directories

- [x] Add the other directories nub uses (`locates_globals_in_user_bin_dir`)

`get_global_lookup_dirs` lists `$XDG_BIN_HOME` and `$HOME/.local/bin` for Nub
(`tools/node-depman/src/package_manager.rs:223-228`). nub 0.11.1 checks the `globalBinDir` setting
(`npm_config_global_bin_dir`) first, and `$XDG_DATA_HOME/../bin` before `~/.local/bin` (nubjs/nub
`vendor/aube/crates/aube/src/commands/global.rs:155-193`). Add both. Both are rare, and this is a
tool change, so it ships with the next toolchain release.

## Not planned

- **`nub ci --prod`:** added in nub 0.8.3 (nubjs/nub#838). It would also give production installs
  their own store (#4), but #3 stops using `nub ci`.
- **A default Docker image:** with only `javascript` and `unstable_nub`, `moon docker file` uses
  `scratch` (moon `crates/app/src/commands/docker/file.rs:288-305`). Returning a `node` image from
  node-depman could replace the `node` toolchain's versioned image, depending on load order. Document
  enabling `node` or setting `docker.file.image` instead.
- **Hashing nub's Node pins** (`.node-version`, `engines.node` and so on): with #6, moon hashes
  `node.version`. Ruby and Python removed their equivalents.
- **Yarn Plug'n'Play:** nub refuses it before writing anything, so there's nothing to handle.

## Outside this repo

moon changes are tracked in [`MOON_COMPACT.md`](MOON_COMPACT.md). This audit added "Load
`unstable_nub` without a `plugin`", "Resolve required toolchain IDs to the configured ID" (#1), and
the Nub website docs.

**nub (upstream):**

- [ ] In Yarn Berry repos, the shared store links transitive dependencies under a doubled name:
      `~/.cache/nub/pm/store/v1/is-odd@3.0.1-…/node_modules/is-number` points at
      `../../is-number@is-number@6.0.0/…`, which doesn't exist, so `require('is-odd')` fails with
      `Cannot find module 'is-number'`. The per-project store used in CI links
      `../../is-number@6.0.0/…` correctly. **Live** with nub 0.11.1 and a Yarn 4.18.1 lockfile;
      npm, pnpm, Bun and Nub lockfiles resolved fine.

## Release

**Blocked in part:** every tool on `master` sets `minimum_proto_version` to 0.64, and moon 2.6.x
embeds proto_core 0.62, so `node_depman_toolchain` can't be released until moon ships proto 0.64
(`MOON_COMPACT.md`). `javascript_toolchain` embeds no tool, and a `master` build loads in moon 2.6.4.

- `javascript_toolchain`: #1–#5 only touch this crate, so they can ship now as 1.4.0, with changelog
  callouts for #2 (no `packageManager` sync in other managers' repos) and #3 (`nub ci` replaced, and
  production installs frozen in CI). #6–#8 and #9's `exe_names` entry can follow in 1.4.x. None of
  this helps users until moon's locator fix ships, since `unstable_nub` doesn't load without a
  `plugin` today.
- `node_depman_toolchain`: #9 and #10 are a patch. The release also carries the tool's Nub fixes from
  the 2026-11-06 audit, which 1.1.1 doesn't have. **Live** with moon 2.6.4: the `nubx` shim had no
  `__NUB_ARGV0`, so it ran nub's main mode ("nub 0.11.1 — the all-in-one Node.js toolkit"), there
  was no `nubr` shim, and globals were looked up in `~/Library/pnpm`.
- `node_depman_tool`: #10 is a patch. The toolchain re-exports it, so it also needs a toolchain
  release.

## Handed to other plans

- `JAVASCRIPT.md`: `initialize_toolchain`'s `config_url` is `https://moonrepo.dev/docs/guides/javascript`
  (`toolchains/javascript/src/tier1.rs:106`, and `toolchains/node-depman/src/tier1.rs:65` for npm,
  pnpm, Yarn and Nub), which returns 404.
- `JAVASCRIPT.md`: `detect_package_manager` (`toolchains/javascript/src/tier1.rs:155-179`) only reads
  `packageManager`, not `devEngines.packageManager`, where nub records itself. Repos Nub owns are
  still found through `nub.lock`.
- `JAVASCRIPT.md`: the tier 2 tests mock `load_toolchain_config` with `|_, _|` for every package
  manager (`tier2_test.rs`, `tier2_env_test.rs`).
- `JAVASCRIPT.md`: `docker scaffold`'s configs phase doesn't copy `patchedDependencies` patch files,
  which pnpm, Bun and Nub installs read. Not checked live.
