# Python toolchain fix plan

Fixes from the Python toolchain audit (2026-11-15, against moon v2.6.5 source, live-checked with
moon 2.6.4, proto 0.64.0, uv 0.13.8, Poetry 2.5.1 and pip 26.1.2). Covers:

- `toolchains/python` (`python_toolchain` 0.3.2, enabled as `unstable_python`). It implements tier 2
  for every package manager, so most fixes land here.
- `toolchains/python-pip` (`python_pip_toolchain` 0.1.5, `unstable_pip`), `toolchains/python-poetry`
  (`python_poetry_toolchain` 0.1.3, `unstable_poetry`) and `toolchains/python-uv`
  (`python_uv_toolchain` 0.1.6, `unstable_uv`). These only register metadata, config and
  requirements, and re-export their proto tool as tier 3 (pip has none).

The tools they re-export (`python_tool`, `python_poetry_tool` and `python_uv_tool`) were audited on
2026-11-06. They're only covered here where the toolchains depend on them. Item numbers (#1–#22)
match the audit findings.

## Testing

Build the WASM first. The test sandbox loads the prebuilt `.wasm`, so rebuild after every source
change; `moon run :build` can replay a stale cached build.

```bash
cargo build -p python_toolchain -p python_pip_toolchain -p python_poetry_toolchain -p python_uv_toolchain --target wasm32-wasip1 --release
```

```bash
cargo nextest run -p python_toolchain -p python_pip_toolchain -p python_poetry_toolchain -p python_uv_toolchain --no-default-features
```

In agent sessions, unset `AI_AGENT` and `CLAUDECODE` first, or wrapper banners can break JSON tests.

Many tests mock `load_toolchain_config` with `|_, _|`, which ignores the requested ID. That's how #1
slipped through. New tests for config loading should assert the ID.

Tool behavior was checked in scratch projects with uv 0.13.8 (and 0.12.3 for #4, #7, #15 and #16),
Poetry 2.5.1 and pip 26.1.2, on proto's CPython 3.14.6 and 3.14.7. Repeat those checks for the items
that depend on the tools rather than on plugin output (#2, #3, #4, #6, #7, #9, #15, #16, #18, #20
and #21).

Findings marked **live** were reproduced with `~/.proto/tools/moon/2.6.4/moon` in a scratch git repo
with an `app` project, against the released `python_toolchain` 0.3.2. Its tier 2 code matches
`master`. To test a fix, point the toolchain at the build:

```yaml title=".moon/toolchains.yml"
unstable_python:
  plugin: "file://<CARGO_TARGET_DIR>/wasm32-wasip1/release/python_toolchain.wasm"
  packageManager: "pip"
  version: "3.14.6"
unstable_pip: {}
```

## Decisions

| Decision                                                          | Recommendation                                                                    |
| ----------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| D1 (#1): which toolchain holds `uv-pip`'s install args?           | **Decided:** `unstable_pip`'s `installArgs`, while `unstable_uv` holds `venvArgs` |
| D2 (#15): should `unstable_python.version` pick uv's interpreter? | **Decided:** yes, `UV_PYTHON` for uv and `uv-pip` when `version` pins a version   |
| D3 (#16): frozen uv installs in CI?                               | **Decided:** yes, `uv sync --locked` when `uv.lock` exists, as JavaScript does    |
| D4 (#17): feed every declared dependency to the task hasher?      | **Decided:** yes, every group and the Poetry tables, with a `*` fallback          |
| D5 (#18): how should `docker setup` install local packages?       | **Decided:** stub `__init__.py` and referenced files, as Rust stubs targets       |
| D6 (#19): honor `unstable_pip.version`?                           | **Decided:** no, mark it unused and `#[deprecated]`, like Go's `vendorDir`        |

All decisions are made.

## Phase 1: clear-cut fixes

### #1 `uv-pip` can't be used at all

- [x] Require `unstable_uv` and `unstable_pip` for `uv-pip`
- [x] Document that `unstable_pip.installArgs` also applies to `uv pip install`
- [x] Tests that assert the requested toolchain ID

**Problem:** `define_requirements` builds the ID with `format!("unstable_{package_manager}")`
(`toolchains/python/src/tier2.rs:24`). `PythonPackageManager::UvPip` displays as `uv-pip`, so the
toolchain requires `unstable_uv-pip`, which doesn't exist. moon fails while building the action
graph (moon `crates/action-graph/src/action_graph_builder.rs:1404-1422`). **Live:** with
`packageManager: uv-pip` and `unstable_uv: {}`, `moon run app:test` fails with "Toolchain
unstable_python requires the toolchain unstable_uv-pip, but it has not been configured!".

By design (D1), `uv-pip` takes its install args from `unstable_pip` (`tier2.rs:238`, `:275-279`)
and its venv args from `unstable_uv` (`:372`). moon's `load_toolchain_config_by_id` errors when the
toolchain isn't configured (moon `crates/plugin/src/host.rs:347`), so both must be configured.

**Change (agreed):**

- Map `UvPip` to `["unstable_uv", "unstable_pip"]` in `define_requirements`, so a missing one is
  reported up front instead of failing in the install.
- Extend `unstable_pip`'s `installArgs` description to cover `uv pip install`, and the Python
  toolchain's `packageManager` docs to say `uv-pip` needs both toolchains.

**Tests** (`tier2_test.rs`): `define_requirements` for each package manager, and a `uv-pip` install
and venv setup whose `load_toolchain_config` mocks assert `pip` and `uv`.

### #2 `uv sync` at a workspace root skips the other members

- [x] Pass `--all-packages` when the root is a uv workspace
- [x] Tests

**Problem:** members collapse into one install at the workspace root, and the plugin runs a plain
`uv sync` there (`tier2.rs:259-274`). In uv 0.13.8, a plain `uv sync` at a root that has its own
`[project]` (what `uv init` creates) only syncs the root package. It installs no other member, and
removes members that are already installed ("Uninstalled 3 packages"). Member tasks then fail
importing their own dependencies. A virtual root (no `[project]`) does install every member.

**Change (agreed):**

- When `input.packages` is empty and the root's `pyproject.toml` has `[tool.uv.workspace]`, add
  `--all-packages`. Add it for virtual roots too: it changes nothing there, and saves telling them
  apart.
- Skip it when `syncArgs` already has `--all-packages` or `--package`, which uv would reject.
- `docker prune` keeps passing `--package` per focused project. uv accepts `--package` repeated
  (confirmed), so that path already works, and uv rejects `--package` with `--all-packages`.
- `--all-packages` was added in late 2024 (astral-sh/uv#8739), so there's no version check.

The shared root venv gets every member's dependencies, not just the root's. That's uv's workspace
model, and moon already installs once per root.

**Tests:** a root with a `[project]` table, a virtual root, a project that isn't a workspace (no
flag), the existing focused-projects case (no flag), and `syncArgs` with `--package` (no flag).

### #3 Recreate the venv when its Python doesn't match `version`

- [x] Read `pyvenv.cfg`
- [x] Delete a stale venv before emitting the create command
- [x] Tests

**Problem:** `setup_environment` returns early whenever the venv exists (`tier2.rs:396-400`), so a
`version` bump never reaches it. **Live:** after bumping `unstable_python.version` from 3.14.6 to
3.14.7, tasks still printed `3.14.6` from `app/.venv`.

The plugin's escape hatch, `venvArgs: ['--clear']` (`tier2.rs:394-396`), is worse:

- moon puts plugin paths ahead of the toolchain's (moon
  `crates/process-augment/src/augmented_command.rs:23-34`), so `python -m venv` runs the old venv's
  own Python and recreates it on 3.14.6.
- The install fingerprint didn't change, so the install was skipped, and the task failed with
  `ModuleNotFoundError: No module named 'six'`.

Running `python -m venv` with 3.14.7 over a 3.14.6 venv leaves it inconsistent: `pyvenv.cfg` says
3.14.7, while `bin/python` still runs 3.14.6.

**Change (agreed):**

- When `version` is set and the venv exists, read `<venv>/pyvenv.cfg`. The stdlib writes
  `version = 3.14.6`, and uv writes `version_info = 3.14.6`, so accept either key.
- Compare it to `version` without build metadata, since proto's `3.12.1+20240107` builds still
  report `3.12.1`. A requirement like `3.14` matches any 3.14.x. Skip the check for aliases.
- On a mismatch, delete the venv in the plugin (`fs::remove_dir_all`, ADR-0003), then emit the usual
  create command. The venv is then gone from PATH when the command runs, so the configured Python
  creates it. The bump also changes `toolchain_config`, which is part of moon's install fingerprint,
  so the install re-runs in the same pipeline. Which Python `uv venv` picks is #15.

**Needs moon:** moon skips a setup or install whose fingerprint has ever run before, not only one
that matches the last run. **Live:** after `rm -rf app/.venv`, both actions logged "No Python
toolchain changes since last run", the venv was never recreated, and the task failed with
`ModuleNotFoundError`. So a bump to a new version works, but reverting to one used before won't: the
plugin deletes the venv, and moon skips both the recreate and the install. That's no worse than
today's wrong venv, and it fails loudly. The fixes are tracked in `MOON_COMPACT.md`.

**Not covered:** a venv whose base Python was removed (`proto clean`) while `version` still matches.
Checking `home` would need mapping a real path back to a virtual one.

**Tests** (`tier2_test.rs`): a `pyvenv.cfg` with each key and a matching version (no command), a
mismatch (venv removed and command emitted), a requirement, and an unset `version` (no check).

### #4 pip and `uv pip` with only a pyproject.toml

- [x] Install the project or its dependencies when there's no requirements file
- [x] Emit no install when there's nothing to install
- [x] Tests

**Problem:** with no `requirements.txt` or `requirements.in`, the plugin runs
`python -m pip install` or `uv pip install` with no requirements (`tier2.rs:287-300`). **Live:** a
pip project with only a `pyproject.toml` failed every run with "ERROR: You must give at least one
requirement to install".

A plain `-e .` isn't enough. Without a `[build-system]`, pip and uv fall back to setuptools, whose
discovery fails on flat layouts: `pip install -e .` and `uv pip install -e .` failed with "Multiple
top-level packages" (`app/` and `lib/`) and "Multiple top-level modules" (`main.py` and `utils.py`).
`installArgs` are appended, so users couldn't remove it.

**Change (agreed):** follow uv's rule, where `uv sync` only installs the project itself when it
declares a `[build-system]`. When there's no requirements file and the root's `pyproject.toml` has a
`[project]` table:

- With a `[build-system]`, append `-e .` for both pip and `uv pip`.
- Without one, install only the dependencies. `uv pip` takes `-r pyproject.toml` (confirmed for
  every layout above, with uv 0.12.3). pip has no such mode, so pass `[project].dependencies` as
  arguments, parsed as `managers/pip.rs` already does. A project with `dynamic` dependencies needs a
  build backend, so it takes the `-e .` path. With no dependencies, return no `install_command`.

With neither a requirements file nor a `[project]` table (like a pyproject.toml that only configures
ruff), return no `install_command` either.

`-e .` needs the sources, so it fails in `docker setup`'s configs-only phase (#18).

**Tests:** with a `[build-system]` (`-e .`), without one (dependency args for pip, `-r
pyproject.toml` for `uv pip`), an empty dependency list and no pyproject (no command), and a
requirements file plus a pyproject (unchanged).

### #5 Put the real lockfile first in `lock_file_names`

- [x] Reorder the Python toolchain's list
- [x] Test

**Problem:** moon's install fingerprint only hashes the first lockfile in `lock_file_names` that
exists (moon `crates/actions/src/actions/install_dependencies.rs:282-297`). The order is
`pylock.toml`, `constraints.txt`, `requirements.txt`, `Pipfile.lock`, `poetry.lock`, `uv.lock`
(`toolchains/python/src/tier1.rs:42-52`), so:

- With a `constraints.txt`, edits to `requirements.txt` never re-run the install. **Live:** adding
  `six` to `requirements.txt` next to a `constraints.txt` skipped the install, and the task failed
  importing `six`.
- uv and Poetry projects that export a `requirements.txt` (`uv export`, `poetry export`), which is
  common for deploys, hash the export instead of `uv.lock` or `poetry.lock`.

**Change (agreed):** order the list as `uv.lock`, `poetry.lock`, `pylock.toml`, `requirements.txt`,
`Pipfile.lock`, `constraints.txt`. The pip toolchain's list (`pylock.toml`, `requirements.txt`,
`constraints.txt`) is already in this order. moon fingerprints the install with the Python
toolchain's list, so it doesn't affect installs anyway.

The list is static, since `register_toolchain` gets no config. So a pip project with a leftover
`uv.lock` hashes that, and misses hand edits to `requirements.txt`. That seems rare.

**Tests:** a `register_toolchain` test asserting the order (`tier1_test.rs` has none yet; Go's has
one).

**Not covered:** an edit to `constraints.txt` alone when `requirements.txt` also exists. The moon
fix is in `MOON_COMPACT.md`.

**Changelog:** the fingerprint changes once, so the next run reinstalls.

### #6 Point every package manager at the configured venv

- [x] Set `VIRTUAL_ENV` from `extend_command`
- [x] Set `UV_PROJECT_ENVIRONMENT` for uv with a custom `venvName`
- [x] Tests

**Problem:** only pip honors a `venvName` other than `.venv`, since it runs the venv's `python` from
PATH. Confirmed with the tools:

- After `uv venv custom-venv`, `uv sync` ignores it and creates and fills `.venv`. Tasks get the
  empty `custom-venv/bin` on PATH. `VIRTUAL_ENV` alone doesn't help: uv warns that it "does not
  match the project environment path `.venv` and will be ignored". `UV_PROJECT_ENVIRONMENT` works.
- Poetry ignores a venv named `venv` and creates its own in its cache directory, which never reaches
  PATH. `VIRTUAL_ENV` pointing at it makes Poetry install there.
- `uv pip install` finds `.venv` by walking up, but with only `custom-venv` it fails with "No
  virtual environment found". `VIRTUAL_ENV` fixes it.

**Change (agreed):** when `gather_shared_paths` locates the venv (`tier2.rs:139-152`):

- Return `VIRTUAL_ENV=<real venv path>` from `extend_command` for every package manager, as
  `activate` does. Tools like Poetry, pyright and mypy read it.
- When `packageManager` is `uv` and `venvName` isn't `.venv`, also return `UV_PROJECT_ENVIRONMENT`
  with the same path, so `uv sync` and `uv run` don't warn. Skip it for `.venv`, which is already
  uv's default: an absolute `UV_PROJECT_ENVIRONMENT` applies to every project uv touches, so a task
  running `uv run --project ../other` would sync that project into this venv, and uninstall its
  packages.

Env values are strings the host passes on, so use the real path (draft ADR: virtual paths in WASM,
real paths in strings). Nothing is set until the venv exists. Setup creates it before the install,
so installs and tasks get both.

**Tests:** extend the `extend_command` tests with the env for the default and a custom name, for
uv (with and without `UV_PROJECT_ENVIRONMENT`), pip and Poetry, and with no venv (no env).

### #7 Keep uv's fallback args when other args are configured

- [x] Always append the fallback args
- [x] Tests

**Problem:** `get_uv_fallback_args` (`tier2.rs:409-419`) is only used when `syncArgs` or `venvArgs`
is empty (`tier2.rs:302-308`, `:383-392`). Configuring any arg, like `syncArgs: ['--all-extras']`,
silently drops `--no-managed-python --no-python-downloads`. uv may then download its own Python and
ignore the version moon installed. `uv pip install` (`uv-pip`) never gets them, since the install
branch only checks for `Uv`.

**Change (agreed):** append the fallback args after the configured ones, for `uv sync`, `uv venv`
and `uv pip install` (which accepts all three). Checked with uv 0.12.3:

- Skip `--no-progress` when the user passed it. Passing it twice fails with "cannot be used multiple
  times".
- Skip `--no-managed-python` and `--no-python-downloads` when the user passed `--managed-python`.
  uv accepts it with `--no-managed-python`, so the user's choice would be silently overridden.
- Skip them for `--python-preference` too, matched as a prefix (`--python-preference=managed`). It
  fails with `--no-managed-python`.
- uv has no `--python-downloads` flag, only `UV_PYTHON_DOWNLOADS` and config, so there's nothing to
  match for it.

Setting any args no longer turns the fallback args off. `--managed-python` is the opt-out.

**Tests:** update `supports_uv_with_custom_args` and the setup equivalent to expect both, and add a
user `--no-progress` (no duplicate), `--managed-python` and `--python-preference=managed` (no Python
flags), and a `uv-pip` install.

### #8 Keep venvs out of `docker scaffold`

- [x] Add exclusions to `scaffold_globs`
- [x] Test

**Problem:** the toolchain has no `vendor_dir_name` (see "Not planned"), so nothing excludes the
venv, and the sources phase copies each project's `**/*` (moon
`crates/app/src/commands/docker/scaffold.rs:102-115`). moon's glob walk ignores `.gitignore`.
**Live:** `moon docker scaffold app` copied `app/.venv` (14 MB, 920 files) into
`.moon/docker/sources/app`, including a `pyvenv.cfg` with
`home = /Users/…/.proto/tools/python/3.14.6/bin`. `COPY` then lays that host venv, with its
host-built wheels, over the venv `docker setup` created in the image.

**Change (agreed):** `define_docker_metadata` (`tier1.rs:127-144`) returns `!<venvName>/**/*` and
`!**/<venvName>/**/*`, the same patterns moon adds for `vendor_dir_name` (moon
`crates/toolchain-plugin/src/toolchain_plugin.rs:317-319`). Its `toolchain_config` is already
merged with the project's, so a project's own `venvName` is used. The patterns also apply to custom
`sourcesPhaseGlobs`, and change nothing in the configs phase.

Leave caches out: `__pycache__` files are checked against their source and interpreter, the tool
caches only cost space, and `-e .` regenerates `*.egg-info`. Only the venv breaks the image.

**Tests:** extend `handles_image_version` in `tier1_test.rs`, or add a test, to cover the globs for
the default and a custom name.

### #9 Production installs should drop what they exclude

- [x] uv: `--no-default-groups` instead of `--no-dev`
- [x] Poetry: `poetry sync --only main`
- [x] Tests

**Problem:** `docker prune` runs `install_dependencies` with `production: true` to strip development
packages (moon `crates/app/src/commands/docker/prune.rs:160-200`). Confirmed with the tools:

- `uv sync --no-dev` only drops the `dev` group. With `default-groups = ["dev", "lint"]`, the lint
  group stays installed. `--no-default-groups` drops both.
- `poetry install --only main` after a full install leaves dev packages installed ("No dependencies
  to install or update"). `poetry sync --only main` removes them.
  `poetry install --sync` works, but warns that it's "deprecated and slated for removal".

`prune_docker` only deletes venvs in unfocused projects (`tier1.rs:146-165`). In a uv workspace the
venv is at the root, so that deletes nothing, and the exact sync above is what prunes.

**Change (agreed):**

- uv: `--no-default-groups` replaces `--no-dev` (`tier2.rs:269-271`). It was added in January 2025
  (astral-sh/uv#10618), so there's no version check.
- Poetry: run `poetry sync --only main` for production. `poetry sync` is new in Poetry 2, so use
  `poetry install --sync --only main` when `unstable_poetry.version` has major 1. With no version,
  assume Poetry 2. `--sync` alone would work on both, but Poetry 2 says it's "slated for removal".
- Non-production Poetry installs stay `poetry install`, so packages users installed by hand survive.
  pip and `uv pip` have no dev group to drop.

**Tests:** update `supports_uv_production` and `supports_poetry_production`, and add Poetry 1.x
(`install --sync`) and unset-version (`sync`) cases.

### #10 Parse requirements files the way pip does

- [x] Join continuation lines and strip comments and options
- [x] Record `--hash` values
- [x] Unit tests outside the `wasm` feature (ADR-0006)

**Problem:** both parsers (`toolchains/python/src/managers/pip.rs:58-106`) give each raw line to
`pep508_rs`, which rejects anything after the requirement. Confirmed with pep508_rs 0.9.2:

- `foo==1.0 \`: "Trailing `\` is not allowed"
- `foo==1.0 --hash=sha256:abc`: "Trailing `--hash=sha256:abc` is not allowed"
- `foo==1.0  # via bar`: "Trailing `# via bar` is not allowed"

`uv export` and `poetry export` write the first form by default, as does
`pip-compile --generate-hashes`. Every pinned package is then skipped, and the "lockfile"
contributes nothing to task hashes.

**Change (agreed):** a pure helper that turns requirements file contents into requirement strings,
following pip's own rules:

- join a line ending in `\` with the next line, as pip's `join_lines` does
- strip comments: `#` at the start of a line, or after whitespace, so `#egg=` URL fragments survive
- split off ` --option` suffixes, keeping the first `--hash=sha256:<hex>` for `LockDependency.hash`
  as bare hex, like the `uv.lock` and `pylock.toml` parsers store it
- skip lines that start with `-`, like `-r`, `-c`, `-e` and `--index-url`

Then parse each string with pep508_rs, and skip it with a `debug!` on failure (ADR-0008.5). Both
`requirements.txt` and `requirements.in` use it. `managers/` is behind the `wasm` feature, so the
helper goes in a new ungated module, like `src/requirements.rs` (ADR-0006). The managers keep the
file reading and logging.

**Not covered:** following `-r` and `-c` includes (exported and compiled lockfiles are flat), and
project edges from `-e ../lib` lines, which pip projects don't get today.

**Tests:** hashed `uv export` output, `# via` annotations inline and on their own line, option
lines, an `#egg=` URL, and an invalid line (skipped).

### #11 Missing uv workspace edges

- [x] Dependency groups and `tool.uv.dev-dependencies`
- [x] Sources inherited from the workspace root
- [x] Path sources
- [x] Fixture and tests

**Problem:** `extend_project_graph` (`tier2.rs:37-137`) only links `[project].dependencies` and
`optional-dependencies`, using the dependent's own `[tool.uv.sources]`:

- `[dependency-groups]` (PEP 735, where `uv add --dev` writes) and `tool.uv.dev-dependencies` are
  never read. The latter is parsed (`pyproject_toml.rs:144`) but unused. A shared test-utils package
  used as a dev dependency therefore has no edge.
- Members inherit the root's `[tool.uv.sources]`. Confirmed: with `a = { workspace = true }` only in
  the root, `b`'s `a>=0.1` locks as `source = { editable = "packages/a" }`. The plugin never reads
  the root, so that versioned edge is dropped.
- `ToolUvSourceOptions` only reads `workspace` (`pyproject_toml.rs:169-173`), so a versioned
  requirement with a `{ path = "../lib" }` source isn't linked.

**Change (agreed):**

- Link `[dependency-groups]` with `DependencyScope::Development` for every package manager (it's a
  standard table, and only unversioned entries link without uv sources), and
  `tool.uv.dev-dependencies` for uv only. Skip `{ include-group = … }` entries, since every group is
  read anyway. `pyproject-toml` 0.13.7 parses `dependency_groups`, but `PyProjectTomlWithToolsInner`
  needs the field.
- Find each member's uv workspace root, walking up to the first `pyproject.toml` with
  `[tool.uv.workspace]`, and load each root once. Merge its sources under the member's own, so the
  member wins for the same name, as uv does. Push the root's path into `input_files`.
- Add `path` to `ToolUvSourceOptions`. A path source is local when it resolves to another project's
  root, relative to the `pyproject.toml` that declares it (the root, for inherited sources). WASM
  can't canonicalize, so normalize `..` lexically. Paths to wheels or sdists match no project.

**Not covered:** projects the workspace excludes (`exclude`, or not matching `members`) still
inherit the root's sources, and would get an edge where uv installs from the index. Matching the
globs would fix it, but it's rare.

**Tests** (`tier2_test.rs`): a fixture with a dev-group edge, a source only in the root, a path
source in the member and one inherited from the root, and an `include-group` entry.

### #12 Poetry projects in the project graph

- [x] Read names from `[tool.poetry]`
- [x] Link path dependencies
- [x] Tests

**Problem:** the graph only reads `[project]`. Projects in the Poetry 1.x style, with
`[tool.poetry]` and no `[project]`, get no alias and no edges. Poetry 2.5.1 still installs them, and
`poetry check` only warns that `[tool.poetry.name]` is deprecated. Local packages are written as
`lib = { path = "../lib", develop = true }` in `[tool.poetry.dependencies]` or a
`[tool.poetry.group.<name>.dependencies]` table, in both styles, and the plugin never reads those
tables. Poetry 2 projects only get an edge when `[project]` lists the package with no version.

**Change (agreed):**

- Add `poetry` to the `Tool` struct (`pyproject_toml.rs:135-139`), with `name` and the dependency
  tables (`dependencies`, `group.<name>.dependencies` and the legacy `dev-dependencies`).
- Use `[tool.poetry].name` when there's no `[project]`, in the first pass, so 1.x projects get an
  alias and other projects can link to them.
- A table entry with `path` resolving to another project's root becomes an edge: `Production` from
  `dependencies`, and `Development` from groups and `dev-dependencies`. Resolve it relative to the
  project's `pyproject.toml`, with #11's lexical helper.
- Values can be a string, a table or an array of tables (per-Python or per-platform constraints).
  Parse them as an untagged enum with a catch-all (ADR-0008.4), and skip the `python` key. Only
  `path` entries link, since Poetry installs versioned ones from the index.
- Like the uv sources check, only do this when `packageManager` is `poetry`.

**Not covered:** `lib @ file:///…` URLs in `[project].dependencies`. URL requirements never link
today, and they're usually absolute.

**Tests:** fixtures in both Poetry styles, with main and dev-group path dependencies, an array of
tables, and a path to a wheel (no edge).

### #13 Parse pyproject.toml leniently

- [x] Own types for the fields the plugin reads
- [x] Skip requirements that can't be parsed
- [x] Tests

**Problem:** the plugin deserializes `pyproject_toml::Project`, whose dependency lists are typed
`Vec<Requirement>`. So does `ToolUv.dev_dependencies` (`pyproject_toml.rs:144`). One requirement
that pep508_rs rejects fails the whole file (confirmed: "TOML parse error" for an invalid
`optional-dependencies` entry). The error comes from three places:

- `extend_project_graph`, which fails the project graph for the whole workspace
- `locate_dependencies_root`
- `parse_manifest`, which fails every task that hashes the project

ADR-0008.5 says to skip the entry. `ToolUvSource`'s untagged enum also has no catch-all variant
(ADR-0008.4).

**Change (agreed):**

- Merge `PyProjectToml` (`pip.rs`) and `PyProjectTomlWithTools` (`tier2.rs`) into one struct of our
  own, holding only what the plugin reads (ADR-0008.2): `[project]` (`name` and `version` as
  `Option`s, the dependency lists as `Vec<String>`, and `dynamic` for #4), `[dependency-groups]`
  (#11), whether there's a `[build-system]` (#4), and `tool.uv` and `tool.poetry` (#12). That drops
  the `Default` workaround (`pyproject_toml.rs:26-40`) and the field clearing in
  `extend_project_graph`.
- Parse each requirement string with pep508_rs, and skip failures with a `debug!` naming the file
  and entry (ADR-0008.6).
- Add an `Unknown(TomlValue)` variant to `ToolUvSource` and `ToolUvWorkspaceSource`.
- Workspace detection only needs `tool.uv.workspace`, so it shouldn't read `[project]` at all.

`pyproject-toml` can then be dropped. The crate also types fields the plugin never reads, like
`readme`, `requires-python` and `authors`, so a value it rejects there fails the file today too.
Invalid TOML still fails, naming the file (ADR-0008.8), since the tools would fail too.

**Tests:** a manifest with one bad requirement still yields the others in all three callers, and a
manifest without `name`.

### #14 Small fixes

All agreed:

- [x] `initialize_toolchain`'s `config_url` (`tier1.rs:76`) is
      `https://moonrepo.dev/docs/guides/python`, which doesn't exist in moon v2.6.5's
      `website/docs/guides`. Point it at `config/toolchain#python`, like `docs_url` and Go's, until
      a Python guide exists (`MOON_COMPACT.md`).
- [x] `detect_package_manager` (`tier1.rs:105-120`) picks pip for `Pipfile` and `Pipfile.lock`,
      which pip can't install. Drop those checks, so Pipfile repos get the prompt. Also detect
      `[tool.uv]` and `[tool.poetry]` in the root `pyproject.toml` (#13's struct), checked in the
      order uv files, Poetry files, `[tool.uv]`, `[tool.poetry]`, then `requirements.txt`.
- [x] `define_docker_metadata` (`tier1.rs:134-141`) emits `python:3.15.0-rc.2` for a prerelease.
      Docker Hub tags use CPython's form, `3.15.0rc2`. Use `python_tool`'s `to_python_version` for
      exact versions, exported with `pub use version::to_python_version;` (no tool release, since
      its behavior doesn't change). Map aliases other than `latest`, like `stable`, to `latest`.
- [x] Remove the unused `PythonVenvLocation` enum and its commented-out setting (`config.rs:17-25`,
      `:36-38`).

Not doing: adding only the host OS's `Scripts` or `bin` (`tier2.rs:147-148`). A PATH entry that
doesn't exist does nothing, and MSYS2's Python uses `bin` on Windows.

**Tests:** `detect_package_manager` for each new rule and for a Pipfile repo (prompt), and Docker
tags for a prerelease, a `stable` alias and an exact version.

## Phase 2: after decisions

### #15 Make `version` pick uv's interpreter (D2)

- [x] Set `UV_PYTHON` for uv commands and tasks
- [x] Changelog

**Problem:** moon sets `PROTO_PYTHON_VERSION` and puts proto's Python on PATH, but uv picks its
interpreter from `.python-version` and `requires-python` first. With the plugin's
`--no-managed-python --no-python-downloads`, a `.python-version` of 3.14.7 and only 3.14.6 on PATH,
`uv venv` and `uv sync` fail with "No interpreter found for Python 3.14.7 in search path"
(confirmed). Without the fallback args, uv silently downloads its own 3.14.7. A task's `uv run`
re-resolves the same way, and recreates the venv whenever the file and moon disagree ("Removed
virtual environment at: .venv").

**Change (agreed):** when `version` is set and `packageManager` is `uv` or `uv-pip`, return
`UV_PYTHON` from `extend_command`, so plugin commands and tasks agree. `UV_PYTHON=3.14.6` overrides
a `.python-version` of 3.14.7 (confirmed). This matches the Rust toolchain's `RUSTUP_TOOLCHAIN`,
where moon's configured version wins over the toolchain file. `to_partial_string` drops operators,
so:

- Exact versions use `to_python_version` (#14), which drops build metadata and gives CPython's
  prerelease form, like `3.15.0rc2`.
- Requirements without a range operator, like `3.14`, `3.14.*` or `~3.14`, pass the partial string.
- `^3.12`, `>=3.11`, ranges and aliases set nothing, so uv chooses as it does today.

With `UV_PYTHON` set, `uv pip install` only targets a venv. On a mismatch it fails with "No virtual
environment found for Python 3.14.7" instead of installing into proto's Python (confirmed with uv
0.12.3), and #3 keeps them matched.

Not chosen: `--python` on the plugin's own commands only, since tasks running `uv run` would still
fight over the venv.

**Tests:** an exact version, a prerelease, `3.14`, `^3.12` (unset), an alias (unset), no `version`
(unset), and pip and Poetry (unset).

**Changelog:** this changes which Python uv uses when the two disagree, so it needs a callout.

### #16 Frozen uv installs in CI (D3)

- [x] Add `--locked` in CI
- [x] Tests

**Problem:** a plain `uv sync` silently re-locks when `uv.lock` is stale (confirmed: exit 0,
"Resolved 3 packages", lockfile rewritten). Poetry already fails on a stale lock ("pyproject.toml
changed significantly since poetry.lock was last generated"), and never re-locks silently.

**Change (agreed):** when `get_host_environment()?.ci` is set and the root has `uv.lock`, add
`--locked`, as the JavaScript toolchain uses `npm ci` and friends in CI. A stale lock then fails
with "The lockfile at `uv.lock` needs to be updated, but `--locked` was provided". Don't use
`--frozen`, which installs from the stale lock without checking it.

uv rejects `--locked` twice, `--frozen` with `--locked`, and `UV_LOCKED=1` with `--frozen`
(confirmed with uv 0.12.3). So skip it when `syncArgs` has `--locked` or `--frozen`, or the host has
`UV_LOCKED` or `UV_FROZEN` set (ADR-0004), since some CI setups set `UV_FROZEN=1` globally.

**Tests:** CI with and without `uv.lock`, not CI, `syncArgs` with `--frozen`, and `UV_FROZEN` set.

### #17 Every declared dependency in task hashes (D4)

- [x] Report every group and table, by scope
- [x] `*` fallback for unusable specifiers
- [x] Tests

**Problem:** moon's task hasher skips manifest dependencies that have no version requirement (moon
`crates/task-hasher/src/task_hashing.rs:433-436`). `parse_manifest`
(`managers/pip.rs:108-131`) only reports `[project].dependencies`:

- Unversioned requirements (`requests`) have no requirement, so they're skipped.
- `UnresolvedVersionSpec` can't parse several PEP 440 specifiers (confirmed with version_spec
  0.11.8): the compatible-release pins `~=1.4.2` and `~=1.4`, and `===1.0`, `==2.0rc1` and
  `>=2.0.0a1`. Those dependencies are skipped too.
- Optional dependencies, `[dependency-groups]`, `tool.uv.dev-dependencies`,
  `[build-system].requires` and all the Poetry tables are never read. Upgrading pytest or ruff in
  the lockfile changes no task hash, unless the lockfile is itself a task input.

**Change (agreed):**

- Report `*` when a requirement has no specifier, one that can't be parsed, or a URL. moon then
  records the first locked entry's hash or version, so the lockfile is the source of truth, like
  Ruby #15. Don't translate `~=` and friends: with one locked entry per name, the usual case, `*`
  picks the same one, and uv's forked locks get their first entry, which is still deterministic.
  `parse_version_spec` already moves prereleases into build metadata, so `*` matches them.
- Scope them like this:
  - `dependencies`: `[project].dependencies`, optional dependencies, and
    `[tool.poetry.dependencies]` (skip its `python` key)
  - `dev_dependencies`: `[dependency-groups]`, `tool.uv.dev-dependencies`, and the Poetry groups
    and `dev-dependencies`
  - `build_dependencies`: `[build-system].requires`
- Poetry's constraint syntax (`^2.31`, `~2.31`, `>=2,<3`, `*`) mostly parses as-is. Use the same `*`
  fallback otherwise. Normalize Poetry table keys (PEP 503) so they match the lockfile's names;
  pep508_rs names already are.
- `[build-system].requires` is never locked, so its hash only changes with the manifest. It's kept,
  since the build backend changes what `-e .` installs.

**Tests:** each table and scope, an unversioned requirement and `~=` (both `*`), a Poetry key that
needs normalizing, and the skipped `python` key.

**Changelog:** this changes every Python task hash once.

### #18 `docker setup` without sources (D5)

- [x] Stub files in the configs phase
- [x] Tests

**Problem:** `docker setup` installs from the configs skeleton, which only has the manifests and
lockfiles. Building a local package for its editable install then fails. Confirmed with workspace
members from `uv init --lib`:

- uv_build: "Expected a Python module at: `packages/b/src/b/__init__.py`". With the module stubbed
  but no README: "failed to open file `…/packages/a/README.md`".
- hatchling: "OSError: Readme file does not exist: README.md".
- Poetry: "…/src/proj does not contain any element", then "Readme path `…/README.md` does not
  exist".
- #2's `--all-packages` and #4's `-e .` hit this for every local package.

These install without the local packages: `uv sync --no-install-workspace`, `--no-install-local`
and `poetry install --no-root`. `--no-install-project` alone isn't enough when members depend on
each other.

**Change (agreed):** implement `scaffold_docker`. In the configs phase, for each project, write an
empty copy of each of these that exists in `input_dir`:

- every `__init__.py`, skipping the venv
- the files `pyproject.toml` references: `project.readme` (a string, or `{ file = … }`),
  `project.license-files` and `license = { file = … }`

The Rust toolchain stubs targets the same way (`toolchains/rust/src/tier1.rs:98-152`). Editable
installs point at the source directory, so they stay valid once the sources phase lays the real
files over the stubs, and the deps layer is only invalidated when a package is added or removed.

Also add `setup.py` and `setup.cfg` to `define_docker_metadata`'s globs. They aren't copied in the
configs phase today, so setuptools projects that use them have no build config there.

**Not covered:** single-module projects (`foo.py`), namespace packages, versions from VCS
(setuptools-scm, hatch-vcs) when `.git` isn't in the image, and versions read from a file
(`[tool.hatch.version] path`, setuptools `attr:`). These fail loudly at `docker setup`.

**Not chosen:** skipping local packages in the configs phase (`uv sync --no-install-local`,
`poetry install --no-root --no-directory`, and #4's dependency args for pip). It works for every
build backend, but needs moon to tell `install_dependencies` it's in Docker (`MOON_COMPACT.md`).
And in moon's documented flow, `moon run app:build` runs between the sources copy and
`docker prune`, and skips the install because setup stored its fingerprint, so local packages would
be missing until `docker prune`.

**Tests:** stubs for a `uv init --lib` member, a hatchling project with `readme` and
`license-files`, and a Poetry `src` layout, and no stubs in the sources phase or at the workspace
level.

### #19 `unstable_pip.version` (D6)

- [x] Fix the setting's description and deprecate it

**Problem:** `PipToolchainConfig.version` says "Configured version to download and install"
(`toolchains/python-pip/src/config.rs:17-18`). pip has no tier 3, and the venv uses the pip that
ships with Python, so the setting does nothing.

**Change (agreed):** follow Go's `vendor_dir` (`toolchains/go/src/config.rs:31-33`): describe it as
"Unused. The venv uses the pip that ships with Python.", and mark it `#[deprecated]`, so the schema
flags it in editors. Keep the field, so existing configs still validate. It's unused for `uv-pip`
too (#1).

Not chosen: a `host_log!` warning, since `setup_environment` runs on every command and the setting
never did anything, and running `python -m pip install pip==<version>` after creating the venv,
which would only apply to new venvs.

## Phase 3: lower priority

### #20 Install from `pylock.toml` with `uv pip`

- [ ] Prefer `pylock.toml` for `uv-pip`
- [ ] Tests

`pylock.toml` is a registered lockfile, but no install reads it. `uv pip install -r pylock.toml`
handles it, including the local project entry that `pip lock` writes (confirmed). pip 26.1.2 only
supports it experimentally, and fails on that same lock ("Can't verify hashes for these file://
requirements because they point to directories"), so leave pip as it is until that changes.

**Change (agreed):** for `uv-pip`, pass `-r pylock.toml` when it exists, ahead of
`requirements.txt` (#5's order). It counts as a requirements file for #4, so `-e .` isn't added,
and `-c constraints.txt` is still added. uv has read `pylock.toml` since spring 2025, so there's no
version check.

**Tests:** `uv-pip` with only `pylock.toml`, with it and `requirements.txt` (the lock wins), and pip
with `pylock.toml` (ignored).

### #21 Dependency groups for pip installs

- [ ] Add `--group dev` outside production
- [ ] Read pip's version from the venv
- [ ] Tests

pip supports `pip install --group dev` (PEP 735, confirmed with pip 26.1.2), from pip 25.1. A venv
gets the pip that shipped with its Python, so older Pythons can have an older one.

**Change (agreed):** on #4's path (no requirements file) and outside production, add `--group dev`
when the pyproject defines a `dev` group:

- `uv pip`: always (confirmed with `-r pyproject.toml --group dev`).
- pip: when the venv's pip is at least 25.1. Read its version from the
  `pip-<version>.dist-info` directory under `<venv>/lib/python*/site-packages` (`Lib/site-packages`
  on Windows), which exists since `setup_environment` runs first. Skip the flag with a `debug!`
  when it's older or not found.

Only `dev`, the convention and uv's default `default-groups`. Other groups can go in
`installArgs`, though they then apply in production too.

**Tests:** `uv pip` with and without a `dev` group, pip 25.1 and 24.x (fixture `dist-info`
directories), production, and a requirements file (no flag).

### #22 Detection names

- [ ] `exe_names` (`tier1.rs:54-64`): add `pip3` and `uvx`. Add `uvx` to the uv toolchain too, as
      bun lists `bunx`, and `pip3` to the pip toolchain.
- [ ] `config_file_globs` (`tier1.rs:26-31`): add `.python-versions`, which proto reads
      (`tools/python/src/proto.rs:45-46`).

This only matters for projects with several toolchains, or none detected: a task whose command
matches no toolchain inherits its project's (moon
`crates/task-builder/src/tasks_builder.rs:951-975`). So `pytest` and `ruff` need no entries, and
`python3.14`-style names are rare.

## Not planned

- **`vendor_dir_name = ".venv"`:** in CI, moon skips the install when the vendor directory isn't
  empty (moon `install_dependencies.rs:124-133`). `setup_environment` creates the venv before the
  install, so dependencies would never be installed in CI. #8 covers the scaffold side instead, and
  `MOON_COMPACT.md` has the moon fix (plugins reporting whether dependencies are installed).
- **Activating the venv for tasks only:** Ruby #7 moved binstubs to the task hooks, so they wouldn't
  shadow other toolchains' commands. Here, the pip install relies on the venv being first on PATH,
  and shadowing only happens when the venv has another toolchain's binaries, like `nodejs-wheel`'s
  `node`. Revisit if that's reported.
- **Hashing `.python-version`:** moon hashes `version` when it's set. Ruby removed its equivalent
  (#8).
- **Syncing `.python-version` from `version`**, and **pipenv** support.

## Outside this repo

moon changes are tracked in [`MOON_COMPACT.md`](MOON_COMPACT.md).

## Release

**Blocked:** every tool on `master` sets `minimum_proto_version` to 0.64 (#227), and the toolchains
embed them as tier 3. moon 2.6.4 and 2.6.5 embed proto_core 0.62, and refuse to load the plugin:
"Unable to use the Python plugin with identifier python, as it requires a minimum proto version of
0.64.0-alpha.0, but found 0.62.2 instead". This fails with or without `version`, so no toolchain
can be released from `master` until moon ships proto 0.64 (`MOON_COMPACT.md`), unless the embedded
tools lower their minimum. The test sandbox doesn't check the minimum, so the tests pass. Phase 1
was checked live with a local build whose `python_tool` and `python_uv_tool` minimums were lowered
to 0.62.

- `python_toolchain`: ship Phase 1 as 0.4.0 (the toolchain is unstable and pre-1.0), with changelog
  callouts for what changes installs: `uv-pip` needs `unstable_pip` too (#1), `--all-packages` (#2),
  venv recreation (#3), pyproject-only installs (#4), a one-time reinstall from the lockfile order
  (#5), `VIRTUAL_ENV` (#6) and the fallback args (#7), and production pruning (#9). Phase 2 changes
  which Python uv runs (#15), fails stale `uv.lock` files in CI (#16) and every task hash (#17), so
  it can follow in 0.4.x or 0.5.0 with its own callouts. Phase 3 is a patch.
- `python_pip_toolchain`: #1's `installArgs` description, #19's deprecation and #22's `pip3` are a
  patch.
- `python_uv_toolchain`: #22's `uvx` is a patch.
- `python_poetry_toolchain`: nothing.
- The tools need no release. #14 only exports `python_tool`'s `to_python_version`, which the
  toolchain builds from its path. The toolchains re-export the tools, so a later tool fix still
  needs a toolchain release.
