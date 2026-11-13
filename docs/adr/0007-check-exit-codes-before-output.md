---
status: accepted
date: 2026-11-10
applies-to: [backends/**, crates/**, extensions/**, toolchains/**, tools/**]
---

# Check the exit code before using a command's output

## Context

Plugins run commands on the host with `exec`, `exec_captured`, `exec_streamed` or the
`exec_command!` macro from `warpgate_pdk`. The host only returns an error when it can't find or
start the command. Otherwise it returns an `ExecCommandOutput` with the command's `exit_code`, plus
its `stdout` and `stderr` when the output was piped. A command that fails looks like one that
succeeds until the plugin checks the exit code. Many tools, like `cargo install`, also write
progress and warnings to stderr when they succeed, so a non-empty stderr doesn't mean that the
command failed.

proto v0.64's `warpgate_pdk` adds an `error_on_nonzero` field to `ExecCommandInput`, which is off
by default. When it's on, `exec` returns an error with the command, its exit code and its stderr,
instead of the output. The PDK checks it, not the host, so plugins can still handle the error.
`exec_captured` and `exec_streamed` leave it off, and the `exec_command!` macro calls the host
directly, so it ignores the field.

Skipping the check causes three kinds of bugs:

- A misleading error. The Rust toolchain parses the stdout of `cargo metadata` as JSON, so a broken
  `Cargo.toml` fails with a JSON parse error instead of cargo's message.
- A wrong result with no error. The JavaScript toolchain parses the stdout of `bun bun.lockb` as a
  Yarn lockfile, so a failed command gives an empty lockfile. The asdf backend caches the empty
  output of a failed `nproc` as its CPU count.
- A failure reported as success. The unpack extension reports success after `tar` or `unzip` fails,
  and the Rust tool reports a toolchain as installed after `rustup toolchain install` fails.

Commands that a toolchain returns for moon to run, as an `ExecCommand`, are out of scope. moon
checks their exit codes itself, unless they set `allow_failure`.

When this ADR was written, plugin code had 27 exec calls:

- 8 checked the exit code first: the asdf backend's scripts, `npm view` in the npm backend,
  `bundle` in the Ruby toolchain, `tar` and `cargo-binstall -V` in the Rust toolchain,
  `/etc/os-release` in the Swift tool, and `grep avx2` in the Bun tool, where the exit code is the
  answer.
- 1 checked it, then parsed the output anyway after a warning: `go list -e` in the Go toolchain.
- 7 parsed or compared the output without checking: `cargo metadata` in the Rust toolchain,
  `bun bun.lockb` in the JavaScript toolchain, `rustup toolchain list` in the Rust tool, `sysctl`
  and `nproc` in the asdf backend, `sysctl` in the Bun tool, and `node --version` in the
  node-depman tool.
- 3 used stderr as the install error before checking: the installs in the cargo and npm backends,
  and the Poetry tool's installer. proto ignores the error when the install succeeds.
- 8 ignored the result: the rustup installer, `rustup toolchain install`, and both
  `rustup toolchain uninstall` calls in the Rust tool, `poetry config` in the Poetry tool,
  `proto install npm bundled` in the Node tool, and `tar` and `unzip` in the unpack extension.

## Decision

1. Plugin code MUST check a command's `exit_code` before it reads the command's `stdout` or
   `stderr`, or runs any logic that depends on the command. This covers every `exec`,
   `exec_captured`, `exec_streamed` and `exec_command!` call, including streamed commands, whose
   output is never read.
2. Whether a command failed MUST be decided by its exit code alone. A non-empty stderr isn't a
   failure.
3. On a non-zero exit code, plugin code MUST return an error. When the command is optional, like a
   probe for a CPU feature, it MAY fall back to a default value or log a warning instead. It MUST
   NOT parse, compare or cache the failed command's output.
4. When a failed command should return an error, plugin code SHOULD set `error_on_nonzero` and
   call `exec`, instead of checking the exit code by hand. Plugins check it by hand when they fall
   back on failure, or when the tool writes its errors to stdout, which the error leaves out.
5. After a non-zero exit code, plugin code MAY read `stdout` and `stderr` to build the error or
   warning. The message SHOULD name the command and include its trimmed stderr, or its stdout for
   tools that write errors there.
6. When the exit code is the answer, like whether `grep` found a match, checking it is enough.

## Exceptions

- The Go toolchain's `go list -e` call MAY parse stdout after a non-zero exit code, once it has
  logged a warning, since it infers relationships from whatever packages `go list` could load.

## Consequences

- Fixing the 18 calls turns some silent failures into errors, like a failed
  `rustup toolchain install` or `tar`. Optional steps, like installing the Node tool's bundled npm,
  may log a warning instead, so they don't fail the whole install.
- Four plugins write the same check and error by hand: the asdf backend's `handle_exec_result`,
  the npm backend's `view_field`, the Ruby toolchain's `exec_bundle`, and the Rust toolchain's
  `cargo-binstall` setup. `error_on_nonzero` can replace them.
- Plugins can't use `error_on_nonzero` until they build against `proto_pdk` and `moon_pdk`
  releases with proto v0.64's `warpgate_pdk`. Until then, they check the exit code by hand.
- Only review enforces these rules. `ExecCommandOutput`'s fields are public, so nothing stops code
  from reading `stdout` first.
