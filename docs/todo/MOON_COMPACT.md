# moon upstream changes

Changes needed in moon (`moonrepo/moon`), found while auditing the plugins in this repo. This is the
one list of them. The plugin plans link here instead of keeping their own.

Paths and line numbers are at moon v2.6.5. Each item names the plan and item it came from. Changes
outside moon, like proto and the registry, stay in their plans.

## Releases

### Ship proto 0.64

- [ ] Update `proto_core` to 0.64 in a moon 2.x release

**Problem:** the tools on plugins' `master` set `minimum_proto_version` to 0.64 (plugins#227), and
toolchains embed them as tier 3. moon 2.6.5 still embeds proto_core 0.62.1, so it refuses to load
any toolchain built from `master`, with or without `version`. **Live** (moon 2.6.4): "Unable to use
the Python plugin with identifier python, as it requires a minimum proto version of 0.64.0-alpha.0,
but found 0.62.2 instead". No toolchain has been released since, so users aren't affected yet, but
none can be. moon_pdk_test_utils doesn't check the minimum, so the plugins' tests pass.

**Source:** found while checking `PYTHON.md` Phase 1 live.

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

**Change:** let a toolchain say whether its dependencies are installed, for example through a field
on `SetupEnvironmentOutput` or a separate function. moon treats "not installed" like a first
install: it bypasses the fingerprint and the affected-mode skip, and doesn't take the CI skip.

**Source:** `PYTHON.md` #3 and "Outside this repo".

### Hash every lockfile in the install fingerprint

- [ ] Hash every lockfile that exists

**Problem:** the fingerprint only hashes the first lockfile in `lock_file_names` that exists
(`install_dependencies.rs:282-297`). **Live:** `constraints.txt` comes before `requirements.txt` in
the Python toolchain's list, so editing `requirements.txt` next to a `constraints.txt` never re-ran
the install.

**Change:** hash every lockfile that exists. The order of `lock_file_names` then stops mattering.

**Source:** `PYTHON.md` #5.

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

### Preserve TOML formatting in `toml_config!`

- [ ] Save through `toml_edit` (v3)

**Problem:** `toml_config!` (`crates/pdk-api/src/macros.rs`) saves through serde `toml`, which
strips comments, sorts keys and expands inline tables. The Rust toolchain works around it with its
own `toml_edit` save (`RUST.md` #2).

**Change:** switch the macro to `toml_edit`. This also fixes
`toolchains/python/src/pyproject_toml.rs`, but changes `save_field`'s signature for every caller, so
it's for v3.

**Source:** `RUST.md` D1 option B.

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

## Done

- **plugins#118**, fixed in moon v2.5.0: `AugmentedCommand::get_toolchain_ids` now returns every
  configured toolchain for commands with no project or task
  (`crates/process-augment/src/augmented_command.rs:126`), so workspace-level setup and install
  commands get the toolchains' PATH and env. `RUBY.md` still listed it as live.
- **moonrepo/moon#2799**, on `master`: inherited dev and build dependency versions are hashed. This
  replaced the `apply_toolchain_dependencies_by_scope` item in `RUST.md`.
