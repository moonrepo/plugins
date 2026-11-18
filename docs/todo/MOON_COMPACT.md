# moon upstream changes

Changes needed in moon (`moonrepo/moon`), found while auditing the plugins in this repo. This is the
one list of them. The plugin plans link here instead of keeping their own.

Paths and line numbers are at moon v2.6.5. Each item names the plan and item it came from. Changes
outside moon, like proto and the registry, stay in their plans.

## Releases

### Ship proto 0.64

- [ ] Update `proto_core` to 0.64 in a moon 2.x release
- [ ] Bump the default locators for the toolchains released with it

**Problem:** the tools on plugins' `master` set `minimum_proto_version` to 0.64 (plugins#227), and
toolchains embed them as tier 3. moon 2.6.5 still embeds proto_core 0.62.1, so it refuses to load
any toolchain built from `master`, with or without `version`. **Live** (moon 2.6.4): "Unable to use
the Python plugin with identifier python, as it requires a minimum proto version of 0.64.0-alpha.0,
but found 0.62.2 instead". No toolchain has been released since, so users aren't affected yet, but
none can be. moon_pdk_test_utils doesn't check the minimum, so the plugins' tests pass.

The JavaScript audits hit the same for `bun_toolchain` (**live**), `deno_toolchain`,
`node_toolchain` and `node_depman_toolchain`. `javascript_toolchain` embeds no tool, so a `master`
build loads in moon 2.6.4 and can ship now. Users only get new toolchain releases once the default
locators move (`crates/config-loader/src/toolchains_config_ext.rs:12-38`).

**Source:** found while checking `PYTHON.md` Phase 1 live. Also blocks the releases in `BUN.md`,
`DENO.md`, `NODE.md`, `NPM.md`, `PNPM.md`, `YARN.md` and `NUB.md`.

## Action pipeline

### Setup and install skips match any past run

- [ ] Compare against the last run of each action
- [ ] Tests

**Problem:** `create_hash_and_return_lock_if_changed` (`crates/actions/src/utils.rs:78-109`) skips
the action when a hash manifest exists for the fingerprint, so a fingerprint that has ever run is
skipped. It's only used by `setup_environment.rs:94` and `install_dependencies.rs:162`. **Live**
(moon 2.6.4, Python with pip):

- `rm -rf app/.venv` after the run that created it: both actions log "No Python toolchain changes
  since last run". The venv is never recreated, and tasks fail.
- `requirements.txt` changed from A to B and back to A: the third install is skipped, so B's
  packages stay. This affects every toolchain, like switching back to a git branch.
- With `PYTHON.md` #3, this can now leave a project with no venv. moon calls the plugin's
  `setup_environment` before checking the fingerprint (`setup_environment.rs:89-114`), and the
  plugin deletes a venv whose Python doesn't match `version` inside that call
  (`toolchains/python/src/tier2.rs:714-715`). When the returned create command's fingerprint ran
  before, moon skips it. So `version` going from A to B and back to A, or a venv recreated outside
  moon with another Python (like a plain `uv run` that follows `.python-version`), ends with the
  venv deleted and not recreated, and the install is skipped too. Before #3, the stale venv stayed.
  **Live** (moon 2.6.4, uv): after a venv was recreated outside moon, setup deleted it and didn't
  recreate it, and only the task's own `uv run` did.

**Change:** compare the fingerprint to the last one persisted for that action, not to every hash
ever seen. Plugins that return the same commands every run (plugins#137) are still skipped. The
tests in `crates/actions/tests/utils_test.rs` may expect the current behavior. This also fixes the
#3 case: a matching venv returns no commands, so after it, a create command never matches the last
run.

**Source:** `PYTHON.md` #3. Ship it with proto 0.64 (see Releases), as `python_toolchain` can't be
released safely with #3 before it.

### Let plugins report whether dependencies are installed

- [ ] Add a plugin hook or output field
- [ ] Treat a negative answer as a first install

**Problem:** a first install, which skips the fingerprint check, only happens when `vendor_dir_name`
is set and that directory is empty (`install_dependencies.rs:100-101`). In CI, a vendor directory
that isn't empty skips the install (`:124-133`). Python can't set `vendor_dir_name`, because
`setup_environment` creates the venv first, so CI would never install. Without it:

- `--affected` runs and VCS hooks skip the install on a fresh clone, and the tasks fail.
- When a venv is recreated (deleted by hand, or by `PYTHON.md` #3 on a version change), the install
  fingerprint hasn't changed, so the new venv gets no packages. The previous item doesn't fix this.

The opposite case, where `vendor_dir_name` is set but the directory never exists, makes every run a
first install: it bypasses the fingerprint and the affected-mode skip, and dedupe never runs.
JavaScript has three of them, as its `vendor_dir_name` is `node_modules`:

- Yarn Plug'n'Play (`nodeLinker: pnp`, the default for Yarn 2+ and Yarn 6) creates no
  `node_modules` (`YARN.md` #14, **live**).
- A Deno workspace with only `deno.json` files never gets one (`DENO.md` #14, **live**).
- `npm install` creates none in a project without dependencies (`NPM.md`, confirmed).

**Change:** let a toolchain say whether its dependencies are installed, for example through a field
on `SetupEnvironmentOutput` or a separate function. moon treats "not installed" like a first
install: it bypasses the fingerprint and the affected-mode skip, and doesn't take the CI skip.

**Source:** `PYTHON.md` #3 and "Outside this repo", `YARN.md` #14, `DENO.md` #14 and `NPM.md`.

### Hash every lockfile in the install fingerprint

- [ ] Hash every lockfile that exists

**Problem:** the fingerprint only hashes the first lockfile in `lock_file_names` that exists
(`install_dependencies.rs:282-297`). **Live:** `constraints.txt` comes before `requirements.txt` in
the Python toolchain's list, so editing `requirements.txt` next to a `constraints.txt` never re-ran
the install.

JavaScript hits it whenever another package manager's lockfile is left behind, since the list runs
`bun.lock`, `bun.lockb`, `deno.lock`, `package-lock.json`, `npm-shrinkwrap.json`, `nub.lock`,
`pnpm-lock.yaml` and `yarn.lock`, and `bun install` keeps the `package-lock.json` it migrated from.
**Live:** in a pnpm repo with a stray `package-lock.json`, a change to `pnpm-lock.yaml` alone
skipped the install, and without the stray file the same change ran it. The task hasher has the
opposite problem, as it merges every lockfile, which `JAVASCRIPT.md` #11 fixes in the plugin.

**Change:** hash every lockfile that exists. The order of `lock_file_names` then stops mattering.

**Source:** `PYTHON.md` #5, `JAVASCRIPT.md` #11 ("Not covered"), `PNPM.md`, `DENO.md` and
`NPM.md`.

### Fingerprint everything that changes an install

- [ ] Add the configs of the toolchains the plugin requires
- [ ] Hash the config files at the dependencies root
- [ ] Hash the paths and URLs of dependencies without a version

**Problem:** `create_hash_content` (`crates/actions/src/actions/install_dependencies.rs:264-322`)
hashes the installing toolchain's config, the first lockfile, and manifest dependencies when a
manifest changed. Three things that change an install aren't in it:

- **Other toolchains' configs.** The JavaScript toolchain installs with the package manager
  toolchain's `installArgs` and `version` (`toolchains/javascript/src/tier2.rs:420-423`), and
  only `javascript`'s config is hashed (`:137-143`). **Live:** changing `pnpm.installArgs` to
  `['--config.node-linker=hoisted']`, and then `pnpm.version`, both logged "No JavaScript
  toolchain changes since last run". The same applies to npm and Yarn. Add the configs of the
  toolchains in `define_requirements`.
- **Config files.** Edits to `bunfig.toml`, `.npmrc`, or `pnpm-workspace.yaml`'s catalogs,
  `overrides`, `patchedDependencies` and `packages` never reinstall. **Live:** after setting
  `linker = "hoisted"` in `bunfig.toml`, moon skipped the install and `node_modules` kept the
  isolated layout. Hash the files matching the toolchain's `config_file_globs` that exist at the
  dependencies root.
- **Dependencies without a version.** `inject_deps` (`:389-404`) only records `get_version()`, so
  `file:`, `link:`, Git and URL dependencies never change the fingerprint. **Live:** switching
  `"left": "file:../../vendor/left2"` to `file:../../vendor/left` skipped the install, and
  `node_modules/left` still pointed at `vendor/left2`. Also hash `ManifestDependencyConfig`'s
  `path` and `url`.

**Source:** `PNPM.md` #10 (D3), `BUN.md`, `YARN.md` and `JAVASCRIPT.md`.

### Store the install fingerprint after the install

- [ ] Hash the lockfile after the install runs

**Problem:** the fingerprint is computed and stored before the install runs
(`install_dependencies.rs:162`, lockfile hash at `:282-297`), so an install that rewrites the
lockfile, like `npm install` after a manifest edit or `npm dedupe`, runs again on the next run.
**Live** (npm, twice): after editing a dependency, two runs in a row each installed, and the third
skipped.

**Change:** compute the stored fingerprint once the install and dedupe have finished.

**Source:** `NPM.md` and `JAVASCRIPT.md`.

### Schedule `SetupEnvironment` for toolchains without a dependencies root

- [ ] Add the action at the workspace root for toolchains with `setup_environment` but no
      `locate_dependencies_root`

**Problem:** `SetupEnvironment` is only added from a located dependencies root, after
`supports_tier_2()`, which means "has `locate_dependencies_root`"
(`crates/action-graph/src/action_graph_builder.rs:411-449`), and `moon setup` only adds it when that
root is the workspace root (`:1465-1481`). Toolchains whose dependencies root belongs to the
JavaScript toolchain never run their `setup_environment`:

- `deno.bins` have never been installed in moon v2 (`DENO.md` #10, **live**).
- `yarn.plugins` have never run (`YARN.md` #10, **live**).
- Node's `syncVersionManagerConfig` never wrote a file (plugins#96), and was deprecated for it.

**Change:** schedule it once at the workspace root for such toolchains. `DENO.md` D1 was decided
for this (2026-11-18), instead of moving Deno's setup into the JavaScript toolchain, so Deno needs
it. `YARN.md` D2 recommends deprecating `yarn.plugins` instead.

**Source:** `DENO.md` #10 (Alternative B) and `YARN.md` #10.

### Hash the manifests of toolchains without a dependencies root

- [ ] Parse a toolchain's manifests for task hashes when another toolchain owns the root

**Problem:** the task hasher starts each toolchain's dependency versions from its own
`locate_dependencies_root`, and skips a toolchain without one
(`crates/task-hasher/src/task_hashing.rs:255-266`). The Deno toolchain's `parse_manifest` is never
called, as the JavaScript toolchain owns Deno's root, so `deno.json` imports never reach task
hashes (`DENO.md` #12, **live**).

**Change:** for a toolchain with `parse_manifest` but no `locate_dependencies_root`, parse its
manifests from the root of a toolchain it's used with, like `javascript` for `deno`. `DENO.md` D1
chose this over parsing `deno.json` in the JavaScript toolchain.

**Source:** `DENO.md` #12.

### Tell `install_dependencies` when it runs in Docker

- [ ] Add the Docker phase to `InstallDependenciesInput`
- [ ] Don't store the fingerprint of a configs-only install

**Problem:** `InstallDependenciesInput` (`crates/pdk-api/src/toolchain/tier2.rs:124`) doesn't say
whether the install runs for `docker setup` (configs only), `docker prune`, or neither.
`production` is only set by `docker prune`, and `docker setup` looks the same as a local install.
So plugins can't adjust the install to the configs-only phase. Python stubs source files for it
instead (`PYTHON.md` #18).

**Change:** add a field, like `docker: Option<ScaffoldDockerPhase>` or a `DockerInstallPhase` enum
for setup and prune. With it, Python could install only third-party packages during
`docker setup` (`uv sync --no-install-local`, `poetry install --no-root --no-directory`), which
works for every build backend.

For that to work, moon must also not store the fingerprint of a configs-only install. moon's
documented flow runs `moon run app:build` between copying the sources and `docker prune`, and that
run skips the install when the fingerprint matches setup's, so local packages would never be
installed before the build.

**Source:** `PYTHON.md` #18.

## Tasks

### Fix `detect_task_usage`

- [ ] Match proto's versioned binaries
- [ ] Match relative paths by file name

**Problem:** `detect_task_usage` (`crates/toolchain-plugin/src/toolchain_plugin.rs:375-391`):

- The proto-binary branch splits each `exe` name and requires `name == exe`, so it can never match.
  Tasks running `node-20` or `python-3.12` aren't detected. It should split `command`.
- It compares the whole command, so `bin/rails` and `./bin/rspec` aren't detected as Ruby tasks.
  Match the file name when the command is a relative path.

**Source:** `RUST.md`, `GO.md`, `RUBY.md` and `PYTHON.md`.

### Order a toolchain's paths ahead of the toolchains it requires

- [ ] Put a requiring toolchain's paths before its requirements'

**Problem:** `inherit_from_toolchains` adds each toolchain's paths in the task's toolchain order
(`crates/process-augment/src/augmented_command.rs:383-418`), and `expand_task_usage` keeps detected
toolchains ahead of the ones it adds
(`crates/toolchain-plugin/src/toolchain_registry_actions.rs:215-245`).
A task whose command is `node`, or a script starting with `node`, gets `[javascript, node, npm]`,
so Node's `bin`, which also holds its bundled `npm`, `npx` and `corepack`, comes before proto's npm.
**Live:** in such a script, `npm --version` printed Node 22's bundled 10.9.8 instead of the
configured 11.13.0, so a `node scripts/release.mjs` that runs `npm publish` misses npm's trusted
publishing (11.5.1+). With Corepack enabled, its `yarn` shim came before proto's Yarn (**live**,
`YARN.md`), and its `pnpm` shim would too.

**Change:** add a toolchain's paths before those of the toolchains it requires (from
`define_requirements`), or have `expand_task_usage` insert a requiring toolchain ahead of its
requirements. Installs already run proto's npm.

**Source:** `NODE.md` and `YARN.md`.

### Put a toolchain's executable directory before its globals directories

- [ ] Reorder `get_command_paths`, or leave it to the plugins

**Problem:** `get_command_paths` (`crates/toolchain-plugin/src/toolchain_plugin.rs:219-241`) puts
a toolchain's globals directories ahead of its own executable directory. Bun's installer and
setup-bun put `bun` itself in `~/.bun/bin`, so with `bun.version` set, that Bun ran moon's
installs and tasks (**live**: 1.2.20 instead of 1.4.0, and it downgraded `bun.lock`). Deno's
`~/.deno/bin` and Yarn's `~/.yarn/bin` can hold the tool the same way.

**Change:** optional. `BUN.md` #1 fixes Bun in the tool with an `activate_environment` that
returns the tool directory, which moon applies first. Reordering here would cover every tool at
once, but changes which `npm` wins over one installed with `npm i -g npm`.

**Source:** `BUN.md` #1, `DENO.md` #3 and `YARN.md` #16.

### Hash the resolved toolchain version

- [ ] Hash the version that `SetupToolchain` resolved

**Problem:** `apply_toolchain` fingerprints the configured spec
(`crates/task-hasher/src/task_hashing.rs:191-208`, and the same on `develop-3.0`). **Live:** with
`node.version: '~22'`, the hash manifest recorded `{"toolchain":"node","version":"~22"}` while
the task ran 22.22.3, the highest installed. A fresh machine installs the newest 22.x, and both
share a hash and remote cache entries. `moon toolchain add` writes partial specs like `~24` from an
`.nvmrc` of `v24` (**live**), so this is the default for Node repos.

**Source:** `NODE.md`.

### Don't fail every task on a lockfile that can't be parsed

- [ ] Log the error and hash without the lockfile

**Problem:** `hasher.optimization` defaults to `accuracy`
(`crates/config/src/workspace/hasher_config.rs:8-11`), and then any `parse_lock` error fails every
task that hashes the toolchain (`crates/task-hasher/src/task_hashing.rs:269-285`). A new lockfile
format breaks every task until the plugin ships a parser. **Live:** Yarn 6's JSON `yarn.lock`
(`YARN.md` #2) and npm 12's v4 `package-lock.json` (`NPM.md` #1) each failed every task in the
workspace.

**Change:** log the error and hash the task without that lockfile's versions. `BUN.md` #8 (D1)
proposes the same in the plugin, for a `bun.lockb` that `bun` can't read.

**Source:** `YARN.md` #2, `NPM.md` #1 and `BUN.md` #8.

### Port moonrepo/moon#2799 to `develop-3.0`

- [ ] Port the inherited-version hashing fix
- [ ] Add an end-to-end hashing test

moonrepo/moon#2799 (`crates/task-hasher/src/task_hashing.rs`) is on `master` but not `develop-3.0`.
Add an end-to-end test when porting it. The `tc-tier2` test plugin on 3.0 parses manifests and
lockfiles, and `master`'s doesn't.

**Source:** `RUST_INHERITED_VERSIONS.md`.

## Project graph

### Let plugins return input globs

- [ ] Accept globs as graph cache inputs

**Problem:** the graph cache only takes exact paths from plugins (`map_plugin_input_paths`,
`crates/workspace/src/workspace_cache.rs:113-125`), so a new source file can't invalidate it. Go
infers edges from source files, so its graph goes stale until a manifest changes.

**Change:** accept globs, like `apps/a/**/*.go`.

**Source:** `GO.md` #17.

## Configuration

### Load `unstable_nub` without a `plugin`

- [ ] Add `unstable_nub` to `inherit_plugin_locators`

**Problem:** `inherit_plugin_locators` only fills in locators for a hard-coded list of IDs, which
leaves out `unstable_nub` (`crates/config-loader/src/toolchains_config_ext.rs:126-160`, list at
`:133-135`), although `get_plugin_locator` maps it (`:25`). This is the case in every release since
v2.5.0, and on `develop-3.0`. **Live:** the documented `unstable_nub: {}` fails with
"unstable_nub.plugin: a locator is required for plugins". `moon toolchain add unstable_nub`
succeeds and writes a config that every later moon command then rejects. The docs
(`website/docs/config/toolchain.mdx`) and the v2.5 blog post show `unstable_nub: {}`.

**Source:** `NUB.md`.

### Resolve required toolchain IDs to the configured ID

- [ ] Map each `require_id` to the configured stable or unstable ID before `get_spec`

**Problem:** the JavaScript toolchain requires `nub`, but the configured ID is `unstable_nub`.
moon then loads a second plugin instance (`crates/action-graph/src/action_graph_builder.rs:278`,
`:1404`, `:1544`, and `crates/plugin/src/registry_loader.rs:16`), and the two `SetupToolchain`
actions race on proto's shims. **Live:** the second run after a fresh nub install failed 4 of 4
times with `proto::store::shim::create_failed`. `NUB.md` #1 works around it in the plugin by
requiring `unstable_nub`.

**Change:** resolve each `require_id` the way `filter_and_resolve_toolchain_ids` does for tasks
(`crates/toolchain/src/lib.rs:89`).

**Source:** `NUB.md` #1.

### A toolchain key with no value loads no toolchains

- [ ] Treat a null entry as `{}`, or fail

**Problem:** with `pnpm:` (no value) in `.moon/toolchains.yml`, no toolchain plugins load at all,
without an error. **Live:** only `system` registered, `moon run` installed nothing, and tasks ran
without Node. Probably the flattened `plugins` map
(`crates/config/src/toolchains_config.rs:104-106`); not traced further.

**Source:** `PNPM.md`.

### Add `javascript` with `moon toolchain add bun`, `deno` or `node`

- [ ] Add `javascript` too, when it isn't configured

**Problem:** `moon toolchain add bun` (`crates/app/src/commands/toolchain/add.rs:32`) writes only
`bun:`. Installs, the project graph and `node_modules/.bin` come from `javascript`, so nothing
installs, with no warning. The same applies to `deno` and `node`.

**Source:** `BUN.md` #9 (D2).

## Docker

### Focus `docker prune` installs by package name, without the root

- [ ] Don't pass a project whose root is the dependencies root
- [ ] Focus by package name, not alias

**Problem:** `get_install_packages_for_projects` (`crates/app/src/commands/docker/prune.rs:225-237`)
passes each focused project's alias as a package to install:

- A project at the dependencies root is included, and npm rejects it as a workspace. **Live:**
  every single-package npm repo fails `moon docker prune` with "No workspaces found:
  --workspace=app", after `node_modules` was already deleted. `NPM.md` #2 fixes it in the plugin.
- With `inheritAliases: false`, or for a package without a name, there's no alias, so the
  production install covers every workspace. Found from source.

**Source:** `NPM.md` #2 and `JAVASCRIPT.md`.

### `moon docker file` without a TTY writes `FROM scratch`

- [ ] Investigate

Observed, not investigated: `moon docker file app` without `--defaults` and without a TTY logged
`image=""` and wrote `FROM scratch`, instead of the toolchain's default image
(`crates/app/src/commands/docker/file.rs:140-156`).

**Source:** `DENO.md` #7.

## PDK

### Preserve TOML formatting in `toml_config!`

- [ ] Save through `toml_edit` (v3)

**Problem:** `toml_config!` (`crates/pdk-api/src/macros.rs`) saves through serde `toml`, which
strips comments, sorts keys and expands inline tables. The Rust toolchain works around it with its
own `toml_edit` save (`RUST.md` #2).

**Change:** switch the macro to `toml_edit`. This also fixes
`toolchains/python/src/pyproject_toml.rs`, but changes `save_field`'s signature for every caller, so
it's for v3.

**Source:** `RUST.md` D1 option B.

### Run host function commands with the toolchain's environment

- [ ] Give `exec_command` the toolchain's PATH and version env

**Problem:** `exec_command` comes from warpgate's shared host functions
(`crates/plugin/src/host.rs:42`), so a command a plugin runs gets moon's own environment. **Live:**
`bun bun.lockb`, which the JavaScript toolchain runs to read a binary lockfile, used the proto
shim's default Bun ("Outdated lockfile version"), and failed with "Command or script bun does
not exist" when proto wasn't on PATH. `BUN.md` #3 sets `PROTO_BUN_VERSION` as a workaround.

**Source:** `BUN.md` #3 and #8.

## Website docs

- [ ] Rust (`RUST.md` #3, landed): revise the claim in `website/docs/guides/rust/handbook.mdx`
      that every task runs "using the exact version". Explain that tasks now get
      `RUSTUP_TOOLCHAIN`, which overrides `rust-toolchain.toml`.
- [ ] Go (`GO.md` #13, #15):
  - `__partials__/create-task/go/inputs.mdx` uses `src/**/*`, which isn't a Go convention. Use
    `**/*.go`, and mention `/go.mod` and `/go.sum` for single-module repos until #15 lands.
  - The Go section of `config/toolchain.mdx` only says to run `moon toolchain info go`. Document
    `GOTOOLCHAIN`, `go tool`, and that `bins` installs into `GOBIN`.
- [ ] Ruby (`RUBY.md` #12):
  - `__partials__/setup-toolchain/ruby/tier3.mdx` says tier 3 builds Ruby from source, which hasn't
    been true since `ruby_tool` 0.2.9.
  - The `create-task/ruby` partials should use `bundle exec` or binstubs.
- [ ] Python (`PYTHON.md` #1, #14):
  - There's no Python guide, and the toolchain docs say nothing about venvs, `venvName` or Docker.
  - Once #1 lands, `__partials__/setup-toolchain/python/tier2.mdx` should say that `uv-pip` needs
    both `unstable_uv` and `unstable_pip`.
  - Document what the toolchain now does for you (`PYTHON.md` #15, #16, #18, #21): `version` sets
    `UV_PYTHON`, which wins over `.python-version`; `uv sync` runs with `--locked` in CI; the
    configs skeleton gets stub `__init__.py`, readme and license files; and pip installs the `dev`
    group.

- [ ] JavaScript (`JAVASCRIPT.md` #14, #18): `/docs/guides/javascript`, which every JavaScript
      toolchain's `initialize_toolchain` links to, is a 404; an index page would fix the link for
      all of them. The Node handbook says synced versions update when they change, which they
      don't.
- [ ] Node (`NODE.md` #3, #4, #6): the `node` section of `config/toolchain.mdx` only says to run
      `moon toolchain info node`.
  - Document `executeArgs` and `profileExecution`: they apply to `node` task commands only, and
    say where profiles go after #4. Say `syncVersionManagerConfig` is unused.
  - It says the toolchain requires `javascript`. Say instead that installs and
    `node_modules/.bin` on PATH come from `javascript`.
  - `guides/javascript/node-handbook.mdx`: note that a member's `engines.node` beats the root's
    `.nvmrc` unless `node.version` or `.prototools` pins it.
- [ ] npm (`NPM.md` #3, #5): `config/toolchain#npm` should say that `installArgs` also apply to
      `npm ci` and `npm dedupe`, and note npm 12's `allowScripts`, `allow-git` and strict flag
      parsing.
- [ ] Deno (`DENO.md` #1, #2, #10, #11, #13):
  - `guides/javascript/deno-handbook.mdx` says the `javascript` toolchain must use Bun as the
    package manager; it's `deno`. It also says edges come from `imports`, `importMap` and
    `deps.ts`, which nothing infers.
  - The `deno` section of `config/toolchain.mdx` only says to run `moon toolchain info deno`.
    Document where `executeArgs` go, `bins`, frozen CI installs, and declaring workspace
    dependencies in `imports`.
- [ ] Yarn (`YARN.md` #3, #9–#11, #13):
  - `__partials__/setup-toolchain/node/tier2.mdx` shows `yarn: {}`. With Yarn 1, that fails every
    install after the first until #3 lands.
  - Document PnP for tasks, the `plugins` deprecation, `YARN_IGNORE_PATH`, and frozen CI installs
    for Yarn 1 and 6.
- [ ] Nub (`NUB.md` #6): `unstable_nub: {}` doesn't load until "Load `unstable_nub` without a
      `plugin`" ships. Add the `toolchains: ['node']` note for inferred tasks.

## Done

- **plugins#118**, fixed in moon v2.5.0: `AugmentedCommand::get_toolchain_ids` now returns every
  configured toolchain for commands with no project or task
  (`crates/process-augment/src/augmented_command.rs:126`), so workspace-level setup and install
  commands get the toolchains' PATH and env. `RUBY.md` still listed it as live.
- **moonrepo/moon#2799**, on `master`: inherited dev and build dependency versions are hashed. This
  replaced the `apply_toolchain_dependencies_by_scope` item in `RUST.md`.
