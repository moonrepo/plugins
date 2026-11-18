# Changelog

## Unreleased

#### 🚀 Updates

- Panics now report their message, instead of an `unreachable` WASM trap error.
- `dx`, which `deno x --install-alias` creates, is now a Deno executable.
- `deno.bins` installs now always pass `--force`, as `deno install --global` fails on an existing
  install. moon v2 doesn't run them yet, as the Deno toolchain has no dependencies root.

#### 🐞 Fixes

- Fixed the default Docker image not existing for a partial version, like `2.9` or `~2.9.0`, an
  alias, `canary`, or a version bound, as Deno only publishes full versions, like `2.9.8`. It's now
  `denoland/deno:latest`.
- Fixed Deno's bin directory, where `deno install --global` writes, not being on `PATH` for tasks
  without `version`, and `$DENO_INSTALL_ROOT/bin` being left out when it's set. When
  `DENO_INSTALL_ROOT` ends with `bin`, Deno v2.2.11+ uses it as-is, and so does the toolchain.
  `DENO_HOME`, which Deno doesn't read, is no longer used.
- Fixed `executeArgs` failing every `deno run` and `deno task` task, as they were placed before the
  subcommand. They now follow `run`, `test`, `bench`, `serve`, `eval`, `repl`, `compile` and `x`,
  come before a script, like `deno main.ts`, and are left out for other subcommands, which reject
  them.
- Fixed a Deno in `~/.deno/bin`, where Deno's installer puts it, running moon's tasks instead of
  the configured `version`.
- Fixed `docker scaffold` copying `.ts` files at project roots, like `main.ts`, into the configs
  skeleton, so every source edit rebuilt the dependency layer.
- Fixed a version that can't be parsed in `deno.json` failing every task hash. The version is now
  skipped.

## 1.1.3

#### 🚀 Updates

- Added `bins.*.args` support, which are passed to `deno install` before the module, for example to grant additional permissions.

## 1.1.2

#### 🚀 Updates

- Updated to support moon v2.6 release.

## 1.1.1

#### 🚀 Updates

- Updated to support moon v2.5 release.

## 1.1.0

#### 🚀 Updates

- Added Deno v2.8 catalogs support.

## 1.0.3

#### 🚀 Updates

- Updated with latest moon v2 plugin APIs.

## 1.0.2

#### 🐞 Fixes

- API compatibility.

## 1.0.1

#### 🚀 Updates

- Updated with moon v2 plugin APIs.

## 1.0.0

#### 🚀 Updates

- Official major release for moon v2.

## 0.1.0

#### 🚀 Updates

- Initial release!
