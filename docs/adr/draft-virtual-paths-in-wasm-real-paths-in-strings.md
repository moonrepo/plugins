---
status: proposed
date: 2026-11-08
applies-to: [backends/**, crates/**, extensions/**, toolchains/**, tools/**]
---

# Virtual paths in WASM, real paths in strings

## Context

Plugins can't see the host's file system directly. moon and proto map a few host directories to
virtual paths that WASI can open: `/userhome` for the home directory, `/workspace` for moon's
workspace root, `/moon` for moon's store, and `/proto` for proto's store. proto also maps `/temp`
from v0.64, and `/cwd` when the working directory is outside of the home directory. `VirtualPath`
holds a path inside that mapping, and `RealPath` a path on the host. warpgate's
`VirtualPath::create()` turns a real path into a virtual one, and `.to_real_path()` turns it back.
Both fail, or return `None`, for a path outside the mapping.

The host converts some paths itself, and passes others on as they are:

- Almost every path field in `moon_pdk_api` and `proto_pdk_api` is a `VirtualPath`, which the host
  converts.
- In `ExecCommandInput`, the host converts `command` when it contains a separator, plus `cwd` and
  every entry in `paths`. It converts each entry of a `PATH` value passed to `set_host_env_var`
  too.
- The `PATH` lists in plugin outputs, like `ExtendCommandOutput.paths` and
  `ActivateEnvironmentOutput.paths`, are documented as "real or virtual" paths.
- Command `args` and `env` values are plain strings, so the host passes them on unchanged. A path in
  one has to be a real path already.

Inside WASM, `Path` only treats `/` as a separator, so `join`, `parent` and `strip_prefix` can't
split a path that uses `\`. The host sends its directories in native form, and warpgate's separator
handling is behind `#[cfg(windows)]`, which never applies to a plugin compiled for WASI. On a
Windows host, that had two effects:

- `.to_real_path()` returned mixed separators, like `C:\Users\me\.proto/tools/node`.
- `VirtualPath::create()` couldn't match a real path below a mapped directory, like
  `C:\Users\me\.cargo`, because `C:\Users\me` isn't one of its components.

proto v0.64's `warpgate_pdk` fixes both, by using `/` for the host directories in its path map and
for the input to `create()`. Converting real paths to `\` instead would break `parent` and `join`,
which would see one long component. The fix is compiled into each plugin, so it only applies to
plugins built with that release. Its tests simulate a Windows host, and it hasn't run on a real
one.

When this ADR was written, plugin code had 30 `.to_real_path()` calls:

- 18 build `args` or `env` values, like `--root` for `cargo install`, `GOROOT`, `JAVA_HOME`,
  and script paths passed to a shell.
- 8 build `PATH` entries, in the Deno, Go, JavaScript, Python, Ruby and Rust toolchains, and 2
  build a `command`, in the Poetry and Rust tools. The host would have converted these.
- 2 only check that a path is mapped, with `.to_real_path()?.is_some()`.

No plugin read a real host path inside WASM without converting it first. Four used hard-coded
virtual prefixes: `/moon/temp` in the unpack extension, `/moon/temp/rust` in the Rust toolchain,
`/proto/backends/asdf/...` in the asdf backend, and `/userhome/.npmrc` in the node-depman tool.
proto v0.64 adds a `temp_dir` to its build, download and native install inputs. moon has no input
for a temp directory.

The node-depman tool was the only plugin to convert `/` to `\`, for its global lookup directories.
The Rust toolchain's `get_relative_path` compares paths from `cargo metadata`'s output as strings,
and replaces `\` with `/` in the result, as warpgate never sees that text.

## Decision

1. Plugin code MUST read and write files through virtual paths. A real path that arrives as text,
   like an environment variable's value or a command's output, MUST be converted with
   `VirtualPath::create()` before it's used.
2. Paths inside WASM MUST use `/` as the separator on every host, including real paths. Plugin code
   MUST NOT write a path with `\` or convert `/` to `\`. Text that may hold a Windows path, like a
   tool's output, MUST go through `VirtualPath::create()`, or have `\` replaced with `/`, before
   it's split or compared.
3. A path inside a string the host passes on unchanged MUST be a real path, from
   `.to_real_path()`. This covers command arguments and environment variable values.
4. Where the host converts paths itself, plugins SHOULD pass virtual paths as they are. This covers
   `VirtualPath` fields, `ExecCommandInput`'s `command`, `cwd` and `paths`, and `PATH` lists
   documented as "real or virtual".
5. When a path can't be mapped, plugin code MUST return an error, or skip the path when it's
   optional. It MAY use `.expect()` for directories the host always maps, like `install_dir` and
   `tool_dir`, under ADR-0005.3.
6. Plugins SHOULD take directories from the input, the context or `get_host_environment()`, instead
   of hard-coding a virtual prefix. A prefix MAY be hard-coded when the host provides no input for
   that directory, like moon's temp directory.

## Exceptions

- Tests, in `tests/` and `#[cfg(test)]` code, MAY build virtual and real paths directly, since they
  set up the mapping themselves.
- The Node tool's `.\vcbuild` is a Windows-only command that the host runs from the build directory,
  and isn't a path used inside WASM.

## Consequences

- This ADR can't be accepted until plugins build against `proto_pdk` and `moon_pdk` releases that
  use proto v0.64's `warpgate_pdk`.
- Real paths in command arguments and environment variable values use `/` on Windows, like
  `C:/Users/me/.proto/tools/go`. Windows APIs accept that, but a tool that compares paths as text,
  or a command run through `cmd.exe`, may not.
- Inside WASM, paths are always virtual, so file access works the same on every host.
- Only review enforces these rules. `VirtualPath` and `RealPath` both display as plain strings, so
  the compiler can't tell which one a command argument holds.
- The 10 `PATH` and `command` conversions can be removed, so the host converts those paths and uses
  `\` on Windows.
- When moon adds a temp directory input, the hard-coded `/moon/temp` prefixes SHOULD switch to it.
