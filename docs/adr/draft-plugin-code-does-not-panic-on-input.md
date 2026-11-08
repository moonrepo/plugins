---
status: proposed
date: 2026-11-07
applies-to: [backends/**, crates/**, extensions/**, toolchains/**, tools/**]
---

# Plugin code doesn't panic on input

## Context

Release builds set `panic = "abort"`, so a panic ends the plugin call in a WASM trap. Rust writes
the panic message to stderr, which Extism drops, so users only saw "wasm trap: wasm `unreachable`
instruction executed". `enable_tracing()` now installs a panic hook that passes the message to
Extism as the call's error, which the host shows instead of the trap.

The hook makes panics readable, not safe. The plugin instance keeps running after a trap: its
memory, its statics and its "still panicking" state carry into later calls, and anything the panic
interrupted stays half-done. The hook also has to be installed before the first panic, because
`set_hook` panics once an instance is panicking.

When this ADR was written, non-test plugin code had 40 `.unwrap()` calls:

- 14 on values the code wrote itself, like regex and TOML literals, which can't fail.
- 14 on values that nearby code guaranteed, like a key it had just inserted.
- 12 on paths or text from moon or users. Most could only fail in edge cases, like a file path
  without a parent, but the download extension panicked on a URL without a `/`.

## Decision

1. `.unwrap()` MAY be used on values the code writes itself, like parsing or compiling a literal,
   or serializing the plugin's own types, since they can't fail.
2. A value that nearby code guarantees MUST be unwrapped with `.expect()`, not `.unwrap()`. This
   includes an `Option` the code just set, a key it just inserted, and a case that an earlier check
   or `match` ruled out. The message SHOULD say why the value can't fail.
3. Where input from the host, user config, files or the network can make an operation fail, plugin
   code MUST return an error, with `?` or a `PluginError`. It MAY use `.expect()` instead when the
   input can't realistically cause the failure, like a file path without a parent, with a message
   saying so.
4. `unreachable!()` MAY be used for cases that earlier code rules out, like an architecture that
   `check_supported_os_and_arch` has already rejected.
5. Every plugin MUST call `enable_tracing()` at the start of its register function, so the panic
   hook is installed before the plugin's other functions run.

## Exceptions

- Tests, in `tests/` and `#[cfg(test)]` code, MAY panic, since that's how they fail.

## Consequences

- Bad input returns an error users can act on, instead of aborting the plugin call.
- An `.expect()` that still fails explains itself, and the panic hook shows that message with its
  source location. The plugin instance is left in an unknown state for later calls.
- Only review enforces rules 1 to 4. Clippy's `unwrap_used` and `expect_used` lints can't tell a
  literal from input, so they would flag code that rule 1 allows.
- proto's loader calls `register_tool` when it loads a plugin, so tools get the hook before their
  other functions. moon's order for toolchains and extensions hasn't been checked.
