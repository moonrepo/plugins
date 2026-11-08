---
status: accepted
date: 2026-11-08
applies-to: [backends/**, crates/**, extensions/**, toolchains/**, tools/**, clippy.toml]
---

# Ask the host, not the WASM runtime

## Context

Plugins run as `wasm32-wasip1` modules inside the host, not on the user's machine. Standard library
APIs that describe the environment answer for the WASI sandbox instead:

- `std::env::var` and `std::env::set_var` read and change the plugin's own WASI environment, not
  the host's environment variables.
- `std::env::consts`, `cfg!(windows)` and `#[cfg(target_os = "...")]` describe the
  `wasm32-wasip1` target, so they never match the user's operating system.
- `std::process`, `std::thread` and `std::net` aren't supported by WASI, and fail at runtime.
- Extism only connects WASI's stdout and stderr when the `EXTISM_ENABLE_WASI_OUTPUT` environment
  variable is set, so output from `println!` and similar macros is dropped.

The PDKs ask the host for each of these instead. When this ADR was written, plugin code already did
so everywhere, and a root `clippy.toml` was added to keep it that way.

## Decision

1. Environment variables MUST be read and set through the host, with `get_host_env_var` and
   `set_host_env_var`. To read or set several at once, plugins SHOULD use `get_host_env_vars` and
   `set_host_env_vars`, which proto v0.64 adds, so it takes one host call instead of one per
   variable.
2. The operating system, architecture, libc and home directory MUST come from
   `get_host_environment()`. Plugin code MUST NOT use `std::env::consts`, `cfg!` or `#[cfg]` on the
   target operating system or architecture.
3. Commands MUST be run through the host, with `exec`, `exec_captured`, `exec_streamed` or
   `exec_command!`, not `std::process`.
4. Messages for users MUST be logged with `host_log!`. Debug output SHOULD use extism-pdk's log
   macros (`trace!`, `debug!`, `info!`, `warn!` and `error!`) or `tracing` events, which
   `enable_tracing()` forwards to the same log. Plugin code MUST NOT use `println!`, `eprintln!`,
   `print!`, `eprint!` or `dbg!`.
5. Plugin code MUST NOT use `std::thread` or `std::net`. HTTP requests go through the host, with
   the `fetch_*` functions or `send_request!`.
6. The APIs these rules forbid SHOULD be listed in the root `clippy.toml`, under
   `disallowed-methods`, `disallowed-macros` or `disallowed-types`, so clippy reports them.

## Exceptions

- Native tests, in `tests/` and `#[cfg(test)]` code, run on the host and MAY use these APIs. Where
  clippy reports one, the test allows it with `#[allow(clippy::disallowed_methods)]` or the
  matching lint.
- The panic hook in `crates/common` MAY call `std::thread::panicking()` (rule 5). It's needed to set
  up panics, as `std::panic::set_hook` panics if an earlier panic trapped, and it only reads the
  panic count, which works in WASI.

## Consequences

- Plugins see the user's environment, operating system and output, not the sandbox's.
- Clippy reports most violations. `#[cfg]` attributes and `std::env::consts` can't be listed in
  `clippy.toml`, so rule 2 relies on review.
- The `lint` task runs clippy with `-D warnings`, so a listed API fails the task.
- `enable_tracing()` only forwards `tracing` events whose target contains `moon`,
  `nodejs_package_json`, `proto`, `starbase` or `typescript_tsconfig_json`. Events from most plugin
  modules, like `go_toolchain::tier2`, are dropped, so extism-pdk's macros are the dependable choice
  for a plugin's own debug output.
