# Changelog

## Unreleased

#### 💥 Breaking

- `uv sync` at a uv workspace root now passes `--all-packages`. A plain `uv sync` at a root with its
  own `[project]` only synced the root package, and uninstalled the other members.
- A venv is now recreated when its Python doesn't match `version`, so a version bump takes effect.
  Before, tasks kept running the venv's old Python.
- Commands and tasks now get `VIRTUAL_ENV` for the venv, so Poetry and `uv pip` use a custom
  `venvName`. uv also gets `UV_PROJECT_ENVIRONMENT` when `venvName` isn't `.venv`, as `uv sync`
  otherwise creates and fills `.venv`.
- uv commands now always get `--no-progress`, and `--no-managed-python --no-python-downloads` when
  `version` is set, after any configured `syncArgs` or `venvArgs`. Configuring any arg used to drop
  them, so uv could download its own Python. Pass `--managed-python` to use uv's Python.
- Production installs from `docker prune` now remove what they exclude: uv uses
  `--no-default-groups` instead of `--no-dev`, so every default group is dropped, and Poetry runs
  `poetry sync --only main` (`poetry install --sync --only main` for Poetry 1.x).
- uv commands and tasks now get `UV_PYTHON` when `version` is a version, or a partial version
  like `3.14`, with `uv` and `uv-pip`. uv used to pick a `.python-version` or `requires-python`
  over moon's Python, so it failed to find it, downloaded its own, or recreated the venv on every
  `uv run`. Requirements with a range, like `^3.12` or `>=3.11`, still leave the choice to uv.
- `uv sync` now passes `--locked` in CI when `uv.lock` exists, so a stale lockfile fails the
  install, instead of being silently re-locked. It's skipped when `syncArgs` has `--locked` or
  `--frozen`, or `UV_LOCKED` or `UV_FROZEN` is set.

#### 🚀 Updates

- pip and `uv pip` now install a project with only a `pyproject.toml`. A project with a
  `[build-system]` is installed with `-e .`, and one without only gets its dependencies, as with
  uv. Nothing is installed when there's nothing to install.
- `uv.lock` and `poetry.lock` now come first in the toolchain's lockfiles, followed by
  `pylock.toml`, `requirements.txt`, `Pipfile.lock` and `constraints.txt`. moon only hashes the
  first lockfile that exists, so editing `requirements.txt` next to a `constraints.txt`, or
  `uv.lock` next to an exported `requirements.txt`, never re-ran the install. Dependencies are
  reinstalled once.
- The project graph now includes `[dependency-groups]` and `tool.uv.dev-dependencies` as
  development dependencies, uv sources inherited from the workspace root, uv `path` sources, and
  Poetry `path` dependencies. Poetry 1.x projects, with only `[tool.poetry]`, now get an alias.
- `moon docker scaffold` no longer copies venvs into the sources skeleton.
- `moon docker scaffold` now stubs each project's `__init__.py` files, and the readme and license
  files that its `pyproject.toml` points to, in the configs skeleton, and copies `setup.py` and
  `setup.cfg`. `docker setup` failed to build local packages without them.
- Task hashes now include every declared dependency, by scope: optional dependencies,
  `[dependency-groups]`, `tool.uv.dev-dependencies`, `[build-system].requires` and the Poetry
  tables. A requirement without a version, or with one that can't be parsed like `~=1.4.2`, now
  uses its locked version, where before it was skipped. Every Python task hash changes once.
- Package manager detection now checks for `[tool.uv]` and `[tool.poetry]` in `pyproject.toml`, and
  no longer picks pip for a `Pipfile`, which pip can't install.
- Panics now report their message, instead of an `unreachable` WASM trap error.
- Lockfile parse errors now name the lockfile.

#### 🐞 Fixes

- Fixed `uv-pip` requiring a toolchain named `unstable_uv-pip`, so it could never be used. It now
  requires `unstable_uv` for the venv and `unstable_pip` for its install args.
- Fixed `requirements.txt` files with hashes, continuation lines or comments, like those from
  `uv export` and `pip-compile`, being parsed as empty.
- Fixed a requirement or field in `pyproject.toml` that can't be parsed failing the whole file,
  which failed the project graph. The requirement is now skipped.
- Fixed the default Docker image for pre-release versions, like `python:3.15.0rc2`, and aliases.
- Fixed the config link shown by `moon toolchain add`.
- Fixed a version that can't be parsed in `poetry.lock`, `uv.lock` or `pylock.toml` failing every
  task hash. The version is now skipped.

## 0.3.2

#### 🐞 Fixes

- Fixed missing project dependencies for versioned requirements using `workspace = true` in the project's `[tool.uv.sources]` with uv.
- Prevented self-dependency cycles from optional requirements.

## 0.3.1

#### 🚀 Updates

- Updated to support moon v2.6 release.

## 0.3.0

#### 🚀 Updates

- Ensures that package manager toolchains are installed before setting up the environment.
- Added support to the Docker pruning workflow where we remove `.venv` directories for non-focused projects (those that were explicitly scaffolded).

## 0.2.1

#### 🚀 Updates

- Updated to support moon v2.5 release.

## 0.2.0

#### 🚀 Updates

- Added Poetry as a supported package manager.

## 0.1.10

#### 🐞 Fixes

- Updated `uv sync` to use `--no-dev` when installing production only dependencies.

## 0.1.9

#### 🐞 Fixes

- Updated venv commands to be skipped during environment setup if the directory already exists. Pass `--clear` to force re-initializing.

## 0.1.8

#### 🐞 Fixes

- Fixed an issue where the wrong arguments were passed to `uv sync` depending on whether proto is managing the Python version.
- Fixed an issue where venv paths were not available to commands ran through the toolchain, like `uv sync`.

## 0.1.7

#### 🐞 Fixes

- Fixed this toolchain depending on pip/uv, when it should be reversed.

## 0.1.6

#### 🐞 Fixes

- Fixed install/venv args being passed incorrectly in some situations.

## 0.1.5

#### 🐞 Fixes

- Fixed an issue where install commands didn't have access to venv bins.
- Fixed an issue where project dependencies were not being inferred correctly when the dependency contains extras metadata.

## 0.1.4

#### 🚀 Updates

- Normalized package/dependency names to PEP 503 during graph extending.

#### 🐞 Fixes

- Fixed an issue where package manager toolchain settings were not being inherited correctly.

## 0.1.3

#### 🚀 Updates

- Added `extend_project_graph` support. Will now read `pyproject.toml` dependencies to determine project relationships.

## 0.1.2

#### 🚀 Updates

- Updated with latest moon v2 plugin APIs.

## 0.1.1

#### 🚀 Updates

- Updated with moon v2 plugin APIs.

## 0.1.0

#### 🚀 Updates

- Initial release!
