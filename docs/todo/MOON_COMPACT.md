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

**Solution:** How should we solve this?

**Change:** let a toolchain say whether its dependencies are installed, for example through a field
on `SetupEnvironmentOutput` or a separate function. moon treats "not installed" like a first
install: it bypasses the fingerprint and the affected-mode skip, and doesn't take the CI skip.

**Source:** `PYTHON.md` #3 and "Outside this repo", `YARN.md` #14, `DENO.md` #14 and `NPM.md`.

### Store the install fingerprint after the install

- [ ] Hash the lockfile after the install runs

**Problem:** the fingerprint is computed and stored before the install runs
(`install_dependencies.rs:162`, lockfile hash at `:282-297`), so an install that rewrites the
lockfile, like `npm install` after a manifest edit or `npm dedupe`, runs again on the next run.
**Live** (npm, twice): after editing a dependency, two runs in a row each installed, and the third
skipped.

**Change:** compute the stored fingerprint once the install and dedupe have finished.

**Source:** `NPM.md` and `JAVASCRIPT.md`.

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

## PDK

### Run host function commands with the toolchain's environment

- [ ] Give `exec_command` the toolchain's PATH and version env

**Problem:** `exec_command` comes from warpgate's shared host functions
(`crates/plugin/src/host.rs:42`), so a command a plugin runs gets moon's own environment. **Live:**
`bun bun.lockb`, which the JavaScript toolchain runs to read a binary lockfile, used the proto
shim's default Bun ("Outdated lockfile version"), and failed with "Command or script bun does
not exist" when proto wasn't on PATH. `BUN.md` #3 sets `PROTO_BUN_VERSION` as a workaround.

**Source:** `BUN.md` #3 and #8.
